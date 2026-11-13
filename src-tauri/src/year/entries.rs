//! Amounts and trips entered on the tax year page. An amount is a value as typed, with an evidence PDF kept
//! at `docs/<year>/evidence/` (required or optional per item); a trip is a date, a destination and the
//! kilometres, valued at the rate the year's rules give for the item's vehicle on that date.

use std::path::{Path, PathBuf};

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::date;
use crate::db::{Change, Db, Op, Source, Written};
use crate::documents::files;
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::statements::import::slug;
use crate::taxrules::model::{ItemKind, Ruleset};

pub const ENTRIES: &str = "entries";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum EntryKind {
    Amount,
    Trip,
}

impl EntryKind {
    fn parse(text: &str) -> EntryKind {
        if text == "trip" {
            EntryKind::Trip
        } else {
            EntryKind::Amount
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EntryRow {
    pub id: String,
    pub year: i32,
    pub item_key: String,
    pub kind: EntryKind,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// An amount's value; None for a trip.
    pub cents: Option<i64>,
    /// A trip's kilometres; None for an amount.
    pub km: Option<i64>,
    /// A trip's destination and purpose, or an amount's note.
    pub text: String,
    /// An amount's evidence PDF, relative to the data folder.
    pub evidence: Option<String>,
    pub created_at: i64,
}

/// Input for `add`, tagged by kind.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum NewEntry {
    #[serde(rename_all = "camelCase")]
    Amount {
        item_key: String,
        date: String,
        amount: String,
        text: String,
        /// The chosen evidence PDF's full path, copied in on add.
        evidence: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Trip {
        item_key: String,
        date: String,
        km: String,
        text: String,
    },
}

pub fn entries(db: &Db, year: i32) -> AppResult<Vec<EntryRow>> {
    db.read(|conn| {
        let mut query = conn.prepare(&format!(
            "SELECT id, year, item_key, kind, date, cents, km, text, evidence, created_at FROM {ENTRIES} \
             WHERE year = ? ORDER BY date, created_at"
        ))?;
        let rows = query.query_map([year], |row| {
            let kind: String = row.get(3)?;
            Ok(EntryRow {
                id: row.get(0)?,
                year: row.get(1)?,
                item_key: row.get(2)?,
                kind: EntryKind::parse(&kind),
                date: row.get(4)?,
                cents: row.get(5)?,
                km: row.get(6)?,
                text: row.get(7)?,
                evidence: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidField {
        field: format!("entry.{field}"),
        message: message.into(),
    }
}

/// The rate per km for the item's vehicle on `day`, in cents; None when the rules give none.
pub fn rate_on(ruleset: &Ruleset, item_key: &str, day: &str) -> Option<i64> {
    let vehicle = ruleset.items.iter().find(|i| i.key == item_key)?.vehicle?;
    ruleset
        .rates
        .kilometergeld
        .iter()
        .find(|r| r.vehicle == vehicle && r.from.as_str() <= day && day <= r.to.as_str())
        .map(|r| r.per_km.0)
}

/// Adds an amount or a trip to `year`: an item of the matching kind and a date in the year. The command refuses
/// a closed year.
pub fn add(
    db: &Db,
    config_dir: &Path,
    ruleset: &Ruleset,
    year: i32,
    entry: &NewEntry,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let (item_key, day_text, text) = match entry {
        NewEntry::Amount {
            item_key,
            date,
            text,
            ..
        }
        | NewEntry::Trip {
            item_key,
            date,
            text,
            ..
        } => (item_key, date, text),
    };
    let item = ruleset
        .items
        .iter()
        .find(|i| &i.key == item_key && i.visible)
        .ok_or_else(|| invalid("item", "Choose one of the year's items"))?;
    let day = date::parse(day_text).ok_or_else(|| invalid("date", "This isn't a date"))?;
    if day.year() != year {
        return Err(invalid("date", &format!("A date in {year}")));
    }
    let (kind, cents, km, evidence) = match entry {
        NewEntry::Amount {
            amount, evidence, ..
        } => {
            if item.kind != ItemKind::Amount {
                return Err(invalid(
                    "item",
                    "This item takes documents or trips, not amounts",
                ));
            }
            let cents = parse_cents(amount)
                .map(i64::abs)
                .filter(|c| *c > 0)
                .ok_or_else(|| invalid("amount", "This isn't an amount"))?;
            let evidence = match evidence {
                Some(from) => Some(copy_evidence(
                    config_dir,
                    &date::iso(day),
                    text,
                    Path::new(from),
                )?),
                None => None,
            };
            (EntryKind::Amount, Some(cents), None, evidence)
        }
        NewEntry::Trip { km, .. } => {
            if item.kind != ItemKind::Trips {
                return Err(invalid(
                    "item",
                    "This item takes documents or amounts, not trips",
                ));
            }
            let km = km
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|k| *k > 0)
                .ok_or_else(|| invalid("km", "Whole kilometres"))?;
            if rate_on(ruleset, item_key, &date::iso(day)).is_none() {
                return Err(invalid("date", "The rules give no rate for this date"));
            }
            if text.trim().is_empty() {
                return Err(invalid("text", "Say where and why"));
            }
            (EntryKind::Trip, None, Some(km), None)
        }
    };
    let row = EntryRow {
        id: uuid::Uuid::new_v4().to_string(),
        year,
        item_key: item_key.clone(),
        kind,
        date: date::iso(day),
        cents,
        km,
        text: text.trim().to_string(),
        evidence,
        created_at: now,
    };
    let label = match kind {
        EntryKind::Amount => format!("Add Amount to KZ {}", item.kz),
        EntryKind::Trip => format!("Add Trip to KZ {}", item.kz),
    };
    db.write(Op::new(label, vec![Change::insert(ENTRIES, &row)?]), source)
}

fn entry(db: &Db, year: i32, id: &str) -> AppResult<EntryRow> {
    entries(db, year)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| AppError::NotFound(format!("No entry {id}")))
}

/// Copies an evidence PDF to `docs/<year>/evidence/<date>-<text>.pdf`; returns its relative path.
fn copy_evidence(config_dir: &Path, day: &str, text: &str, from: &Path) -> AppResult<String> {
    let bytes = std::fs::read(from)?;
    let name = slug(text);
    let name = if name.is_empty() {
        "amount".into()
    } else {
        name
    };
    let base = format!("{}/{}/evidence/{day}-{name}.pdf", files::FOLDER, &day[..4]);
    files::copy_to(
        config_dir,
        &base,
        &bytes,
        &crate::statements::file_hash(&bytes),
    )
}

/// Attaches the PDF at `from` as an amount's evidence, replacing any earlier one; the old file stays.
pub fn attach(
    db: &Db,
    config_dir: &Path,
    year: i32,
    id: &str,
    from: &Path,
    source: Source,
) -> AppResult<Written> {
    let before = entry(db, year, id)?;
    if before.kind != EntryKind::Amount {
        return Err(AppError::InvalidInput(
            "Only an amount takes evidence".into(),
        ));
    }
    let after = EntryRow {
        evidence: Some(copy_evidence(config_dir, &before.date, &before.text, from)?),
        ..before.clone()
    };
    db.write(
        Op::new(
            "Attach Evidence",
            vec![Change::update(ENTRIES, &before, &after)?],
        ),
        source,
    )
}

/// An amount's evidence file, to open.
pub fn evidence_path(db: &Db, config_dir: &Path, year: i32, id: &str) -> AppResult<PathBuf> {
    let row = entry(db, year, id)?;
    let file = row
        .evidence
        .ok_or_else(|| AppError::NotFound("This amount has no evidence".into()))?;
    Ok(config_dir.join(file))
}

pub fn delete(db: &Db, year: i32, id: &str, source: Source) -> AppResult<Written> {
    let row = entry(db, year, id)?;
    let label = match row.kind {
        EntryKind::Amount => "Delete Amount",
        EntryKind::Trip => "Delete Trip",
    };
    db.write(Op::new(label, vec![Change::delete(ENTRIES, &row)?]), source)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;
    use crate::date::DateOrder;
    use crate::settings::NumberFormat;
    use crate::year::page::{AddKind, AttentionKind, AttentionView, RowKind, YearPage, page};
    use crate::year::tests::{insert, open_year_with, rules};

    const NOW: i64 = 1_700_000_000;

    /// The year's rules plus a required-evidence amount item (727), a trips item without a vehicle (728)
    /// and car rates of €0.50 until June and €0.60 from July on item 724.
    fn entry_rules() -> Value {
        let mut rules = rules();
        rules["items"].as_array_mut().unwrap().extend([
            json!({ "key": "fees", "kz": "727", "name": "Fees", "section": "main", "kind": "amount" }),
            json!({
                "key": "walk", "kz": "728", "name": "Walking", "section": "main", "kind": "trips",
                "evidence": "none"
            }),
        ]);
        rules["items"][3]["vehicle"] = json!("car");
        rules["rates"] = json!({ "kilometergeld": [
            { "vehicle": "car", "perKm": 0.5, "from": "2026-01-01", "to": "2026-06-30" },
            { "vehicle": "car", "perKm": 0.6, "from": "2026-07-01", "to": "2026-12-31" }
        ]});
        rules
    }

    struct Fixture {
        _db_dir: TempDir,
        db: Db,
        config: TempDir,
        ruleset: Ruleset,
    }

    fn fixture() -> Fixture {
        let rules = entry_rules();
        let (db_dir, db) = open_year_with(&rules);
        Fixture {
            _db_dir: db_dir,
            db,
            config: tempfile::tempdir().unwrap(),
            ruleset: serde_json::from_value(rules).unwrap(),
        }
    }

    impl Fixture {
        fn add(&self, entry: &NewEntry) -> AppResult<Written> {
            add(
                &self.db,
                self.config.path(),
                &self.ruleset,
                2026,
                entry,
                NOW,
                Source::Ui,
            )
        }

        fn stored(&self) -> Vec<EntryRow> {
            entries(&self.db, 2026).unwrap()
        }

        fn only(&self) -> EntryRow {
            let mut all = self.stored();
            assert_eq!(all.len(), 1, "{all:?}");
            all.remove(0)
        }

        fn pdf(&self, name: &str, bytes: &[u8]) -> String {
            let path = self.config.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            path.to_string_lossy().into_owned()
        }

        fn year_page(&self) -> YearPage {
            page(
                &self.db,
                2026,
                "",
                NumberFormat::DotDecimal,
                DateOrder::DayFirst,
            )
            .unwrap()
        }

        fn attach(&self, id: &str, from: &str) -> AppResult<Written> {
            attach(
                &self.db,
                self.config.path(),
                2026,
                id,
                Path::new(from),
                Source::Ui,
            )
        }
    }

    fn amount(
        item: &str,
        date: &str,
        amount: &str,
        text: &str,
        evidence: Option<String>,
    ) -> NewEntry {
        NewEntry::Amount {
            item_key: item.into(),
            date: date.into(),
            amount: amount.into(),
            text: text.into(),
            evidence,
        }
    }

    fn trip(item: &str, date: &str, km: &str, text: &str) -> NewEntry {
        NewEntry::Trip {
            item_key: item.into(),
            date: date.into(),
            km: km.into(),
            text: text.into(),
        }
    }

    fn refused_field(f: &Fixture, entry: &NewEntry) -> String {
        match f.add(entry).unwrap_err() {
            AppError::InvalidField { field, .. } => field,
            other => panic!("expected an invalid field, got {other:?}"),
        }
    }

    fn kennzahl<'a>(page: &'a YearPage, kz: &str) -> &'a crate::year::page::KennzahlView {
        page.kennzahlen.iter().find(|k| k.kz == kz).unwrap()
    }

    #[test]
    fn an_amount_stores_the_absolute_cents_the_date_and_the_trimmed_text() {
        let f = fixture();

        let written = f
            .add(&amount(
                "tips",
                "2026-03-04",
                "-12,50",
                "  Copy shop  ",
                None,
            ))
            .unwrap();

        assert_eq!(
            written,
            Written {
                label: "Add Amount to KZ 722".into(),
                tables: vec!["entries".into()],
            }
        );
        let stored = f.only();
        assert_eq!(
            stored,
            EntryRow {
                id: stored.id.clone(),
                year: 2026,
                item_key: "tips".into(),
                kind: EntryKind::Amount,
                date: "2026-03-04".into(),
                cents: Some(1_250),
                km: None,
                text: "Copy shop".into(),
                evidence: None,
                created_at: NOW,
            }
        );
    }

    #[test]
    fn a_trip_stores_its_kilometres_and_is_labelled_with_the_kennzahl() {
        let f = fixture();

        let written = f
            .add(&trip("parking", "2026-11-12", " 12 ", "Client visit"))
            .unwrap();

        assert_eq!(written.label, "Add Trip to KZ 724");
        let stored = f.only();
        assert_eq!(
            (stored.kind, stored.cents, stored.km, stored.text.as_str()),
            (EntryKind::Trip, None, Some(12), "Client visit")
        );
        assert_eq!(stored.evidence, None);
    }

    #[test]
    fn an_entry_is_refused_for_the_field_that_is_wrong() {
        let f = fixture();
        let cases = [
            (amount("nope", "2026-03-04", "5", "x", None), "entry.item"),
            (amount("secret", "2026-03-04", "5", "x", None), "entry.item"),
            (
                amount("employer", "2026-03-04", "5", "x", None),
                "entry.item",
            ),
            (amount("books", "2026-03-04", "5", "x", None), "entry.item"),
            (
                amount("parking", "2026-03-04", "5", "x", None),
                "entry.item",
            ),
            (trip("tips", "2026-03-04", "5", "x"), "entry.item"),
            (trip("books", "2026-03-04", "5", "x"), "entry.item"),
            (amount("tips", "not a date", "5", "x", None), "entry.date"),
            (amount("tips", "2025-12-31", "5", "x", None), "entry.date"),
            (amount("tips", "2027-01-01", "5", "x", None), "entry.date"),
            (trip("parking", "2027-01-01", "5", "x"), "entry.date"),
            (amount("tips", "2026-03-04", "0", "x", None), "entry.amount"),
            (
                amount("tips", "2026-03-04", "0,00", "x", None),
                "entry.amount",
            ),
            (
                amount("tips", "2026-03-04", "abc", "x", None),
                "entry.amount",
            ),
            (amount("tips", "2026-03-04", "", "x", None), "entry.amount"),
            (trip("parking", "2026-03-04", "0", "x"), "entry.km"),
            (trip("parking", "2026-03-04", "-3", "x"), "entry.km"),
            (trip("parking", "2026-03-04", "12.5", "x"), "entry.km"),
            (trip("parking", "2026-03-04", "far", "x"), "entry.km"),
            (trip("parking", "2026-03-04", "5", "   "), "entry.text"),
            (trip("walk", "2026-03-04", "5", "x"), "entry.date"),
        ];

        for (entry, field) in cases {
            assert_eq!(refused_field(&f, &entry), field, "{entry:?}");
        }
        assert!(f.stored().is_empty());
    }

    #[test]
    fn rate_on_gives_the_rate_of_the_items_vehicle_with_inclusive_bounds() {
        let f = fixture();
        for (item, day, expected) in [
            ("parking", "2026-01-01", Some(50)),
            ("parking", "2026-06-30", Some(50)),
            ("parking", "2026-07-01", Some(60)),
            ("parking", "2026-12-31", Some(60)),
            ("parking", "2025-12-31", None),
            ("parking", "2027-01-01", None),
            ("walk", "2026-03-04", None),
            ("books", "2026-03-04", None),
            ("nope", "2026-03-04", None),
        ] {
            assert_eq!(rate_on(&f.ruleset, item, day), expected, "{item} {day}");
        }
    }

    #[test]
    fn an_amount_with_evidence_copies_the_pdf_into_the_years_evidence_folder() {
        let f = fixture();
        let first = f.pdf("first.pdf", b"first receipt");
        let second = f.pdf("second.pdf", b"second receipt");

        f.add(&amount("fees", "2026-03-04", "80", "Exam fee", Some(first)))
            .unwrap();
        f.add(&amount(
            "fees",
            "2026-03-04",
            "80",
            "Exam fee",
            Some(second),
        ))
        .unwrap();

        let stored = f.stored();
        let paths: Vec<Option<&str>> = stored.iter().map(|e| e.evidence.as_deref()).collect();
        assert_eq!(
            paths,
            [
                Some("docs/2026/evidence/2026-03-04-exam-fee.pdf"),
                Some("docs/2026/evidence/2026-03-04-exam-fee-2.pdf")
            ]
        );
        assert_eq!(
            std::fs::read(
                f.config
                    .path()
                    .join("docs/2026/evidence/2026-03-04-exam-fee.pdf")
            )
            .unwrap(),
            b"first receipt"
        );
        assert_eq!(
            std::fs::read(
                f.config
                    .path()
                    .join("docs/2026/evidence/2026-03-04-exam-fee-2.pdf")
            )
            .unwrap(),
            b"second receipt"
        );
    }

    #[test]
    fn attaching_evidence_sets_it_on_the_amount_and_undo_takes_it_away() {
        let f = fixture();
        f.add(&amount("fees", "2026-03-04", "80", "Exam fee", None))
            .unwrap();
        let id = f.only().id;
        let source = f.pdf("receipt.pdf", b"receipt");

        let written = f.attach(&id, &source).unwrap();

        assert_eq!(written.label, "Attach Evidence");
        assert_eq!(
            f.only().evidence.as_deref(),
            Some("docs/2026/evidence/2026-03-04-exam-fee.pdf")
        );
        f.db.undo().unwrap();
        assert_eq!(f.only().evidence, None);
    }

    #[test]
    fn attaching_evidence_to_a_trip_or_an_unknown_entry_is_refused() {
        let f = fixture();
        f.add(&trip("parking", "2026-03-04", "5", "Client visit"))
            .unwrap();
        let id = f.only().id;
        let source = f.pdf("receipt.pdf", b"receipt");

        let on_trip = f.attach(&id, &source).unwrap_err();
        let unknown = f.attach("nope", &source).unwrap_err();

        assert!(matches!(on_trip, AppError::InvalidInput(_)), "{on_trip:?}");
        assert!(matches!(unknown, AppError::NotFound(_)), "{unknown:?}");
        assert_eq!(f.only().evidence, None);
    }

    #[test]
    fn the_evidence_path_points_into_the_config_folder_and_needs_evidence() {
        let f = fixture();
        let source = f.pdf("receipt.pdf", b"receipt");
        f.add(&amount(
            "fees",
            "2026-03-04",
            "80",
            "Exam fee",
            Some(source),
        ))
        .unwrap();
        f.add(&amount("tips", "2026-03-05", "5", "Tip", None))
            .unwrap();
        let stored = f.stored();

        let with = evidence_path(&f.db, f.config.path(), 2026, &stored[0].id).unwrap();
        let without = evidence_path(&f.db, f.config.path(), 2026, &stored[1].id).unwrap_err();
        let unknown = evidence_path(&f.db, f.config.path(), 2026, "nope").unwrap_err();

        assert_eq!(
            with,
            f.config
                .path()
                .join("docs/2026/evidence/2026-03-04-exam-fee.pdf")
        );
        assert!(matches!(without, AppError::NotFound(_)), "{without:?}");
        assert!(matches!(unknown, AppError::NotFound(_)), "{unknown:?}");
    }

    #[test]
    fn deleting_an_entry_is_labelled_by_its_kind_and_undo_brings_it_back() {
        let f = fixture();
        f.add(&amount("tips", "2026-03-05", "5", "Tip", None))
            .unwrap();
        f.add(&trip("parking", "2026-03-06", "5", "Client visit"))
            .unwrap();
        let ids: Vec<String> = f.stored().into_iter().map(|e| e.id).collect();

        let amount_label = delete(&f.db, 2026, &ids[0], Source::Ui).unwrap().label;
        let trip_label = delete(&f.db, 2026, &ids[1], Source::Ui).unwrap().label;

        assert_eq!(
            (amount_label.as_str(), trip_label.as_str()),
            ("Delete Amount", "Delete Trip")
        );
        assert!(f.stored().is_empty());
        f.db.undo().unwrap();
        assert_eq!(f.only().id, ids[1]);
    }

    #[test]
    fn deleting_an_unknown_entry_is_not_found() {
        let f = fixture();

        let err = delete(&f.db, 2026, "nope", Source::Ui).unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn an_amount_counts_in_its_kennzahl_and_sits_under_the_whole_year() {
        let f = fixture();
        f.add(&amount("tips", "2026-03-04", "12,50", "Copy shop", None))
            .unwrap();

        let page = f.year_page();

        assert_eq!(kennzahl(&page, "722").copy, "12,50");
        assert_eq!(page.total, "€12.50");
        let row = &kennzahl(&page, "722").rows[0];
        assert_eq!(
            (
                row.kind,
                row.title.as_str(),
                row.detail.as_str(),
                row.amount.as_str()
            ),
            (
                RowKind::Amount,
                "Copy shop",
                "Amount for the year",
                "€12.50"
            )
        );
        assert_eq!(page.months.len(), 1);
        assert_eq!(page.months[0].label, "Whole year");
        assert_eq!(page.months[0].total, "€12.50");
    }

    #[test]
    fn a_trip_counts_kilometres_times_the_rate_of_its_date_and_sits_in_its_month() {
        let f = fixture();
        f.add(&trip("parking", "2026-11-12", "12", "Client visit"))
            .unwrap();
        f.add(&trip("parking", "2026-03-05", "10", "Site visit"))
            .unwrap();

        let page = f.year_page();

        assert_eq!(kennzahl(&page, "724").copy, "12,20");
        let november = page.months.iter().find(|m| m.label == "November").unwrap();
        assert_eq!(november.total, "€7.20");
        assert_eq!(
            (november.rows[0].kind, november.rows[0].detail.as_str()),
            (RowKind::Trip, "12 Nov · 12 km × €0.60")
        );
        let march = page.months.iter().find(|m| m.label == "March").unwrap();
        assert_eq!(march.rows[0].detail, "5 Mar · 10 km × €0.50");
        assert_eq!(page.total, "€12.20");
    }

    #[test]
    fn an_amount_that_needs_evidence_counts_nothing_and_asks_for_it_until_attached() {
        let f = fixture();
        f.add(&amount("fees", "2026-03-04", "80", "Exam fee", None))
            .unwrap();
        let id = f.only().id;

        let waiting = f.year_page();

        assert_eq!(kennzahl(&waiting, "727").copy, "0,00");
        assert_eq!(waiting.total, "€0.00");
        let row = &kennzahl(&waiting, "727").rows[0];
        assert_eq!(
            (row.attach, row.amount.as_str(), row.detail.as_str()),
            (true, "€0.00", "€80.00 · not counted without evidence")
        );
        assert_eq!(
            waiting.attention,
            [AttentionView {
                kind: AttentionKind::MissingInvoice,
                title: "Exam fee · €80.00".into(),
                detail: "KZ 727 needs evidence before it counts".into(),
                document_id: None,
                entry_id: Some(id.clone()),
            }]
        );
        assert_eq!(waiting.progress, "1 invoice and 2 answers missing");

        let source = f.pdf("receipt.pdf", b"receipt");
        f.attach(&id, &source).unwrap();
        let done = f.year_page();

        assert_eq!(kennzahl(&done, "727").copy, "80,00");
        assert_eq!(done.total, "€80.00");
        let row = &kennzahl(&done, "727").rows[0];
        assert_eq!((row.attach, row.amount.as_str()), (false, "€80.00"));
        assert!(done.attention.is_empty());
    }

    #[test]
    fn an_entry_on_an_item_reported_by_others_never_counts() {
        let f = fixture();
        insert(
            &f.db,
            ENTRIES,
            &EntryRow {
                id: "e-1".into(),
                year: 2026,
                item_key: "employer".into(),
                kind: EntryKind::Amount,
                date: "2026-03-04".into(),
                cents: Some(9_900),
                km: None,
                text: "Reported".into(),
                evidence: None,
                created_at: 1,
            },
        );

        let page = f.year_page();

        assert_eq!(page.total, "€0.00");
        assert!(kennzahl(&page, "726").rows.is_empty());
        assert!(page.attention.is_empty());
    }

    #[test]
    fn an_item_reported_by_others_always_carries_the_reference_note() {
        let f = fixture();

        let page = f.year_page();

        assert_eq!(
            kennzahl(&page, "726").note.as_deref(),
            Some("Reported by others; shown for reference, never counted here")
        );
        assert_eq!(kennzahl(&page, "727").note, None);
    }

    #[test]
    fn each_kennzahl_offers_the_add_that_fits_its_item_and_shows_a_trips_rate() {
        let f = fixture();

        let page = f.year_page();

        let shown: Vec<(&str, Option<AddKind>, Option<&str>, bool)> = page
            .kennzahlen
            .iter()
            .map(|k| (k.kz.as_str(), k.add, k.rate.as_deref(), k.reported))
            .collect();
        assert_eq!(
            shown,
            [
                ("720", Some(AddKind::Document), None, false),
                ("721", Some(AddKind::Document), None, false),
                ("722", Some(AddKind::Amount), None, false),
                ("724", Some(AddKind::Trip), Some("× €0.60 / km"), false),
                ("726", None, None, true),
                ("727", Some(AddKind::Amount), None, false),
                ("728", Some(AddKind::Trip), None, false),
            ]
        );
    }

    #[test]
    fn a_kennzahl_view_reaches_the_page_in_camel_case_with_its_add_kind_and_rate() {
        let f = fixture();

        let page = f.year_page();

        assert_eq!(
            serde_json::to_value(kennzahl(&page, "724")).unwrap(),
            json!({
                "key": "parking",
                "kz": "724",
                "name": "Parking",
                "copy": "0,00",
                "rows": [],
                "note": null,
                "add": "trip",
                "rate": "× €0.60 / km",
                "reported": false
            })
        );
    }

    #[test]
    fn the_page_sends_new_entries_tagged_by_kind_in_camel_case() {
        let amount_json = json!({
            "kind": "amount", "itemKey": "tips", "date": "2026-03-04", "amount": "12,50", "text": "Copy shop"
        });
        let trip_json = json!({
            "kind": "trip", "itemKey": "parking", "date": "2026-03-04", "km": "12", "text": "Client visit"
        });

        assert_eq!(
            serde_json::from_value::<NewEntry>(amount_json).unwrap(),
            amount("tips", "2026-03-04", "12,50", "Copy shop", None)
        );
        assert_eq!(
            serde_json::from_value::<NewEntry>(trip_json).unwrap(),
            trip("parking", "2026-03-04", "12", "Client visit")
        );
    }
}
