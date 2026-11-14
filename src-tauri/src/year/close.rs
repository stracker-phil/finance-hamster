//! Closing a tax year: drafts and undated documents block it, missing invoices and unanswered questions only
//! warn. Closing stores a snapshot of what the year counted with a summary PDF in `snapshots/`; afterwards the
//! year is read-only until Reopen, which keeps the snapshot.

use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::page::{self, AttentionKind, RowKind, YearPage};
use super::report;
use crate::date::DateOrder;
use crate::db::{Change, Db, Op, Source, Written};
use crate::documents::store as documents;
use crate::error::{AppError, AppResult};
use crate::settings::NumberFormat;
use crate::store::json_file::create_private_dir;
use crate::taxyears::{self, TaxYearRow, YearState};

pub const SNAPSHOTS: &str = "snapshots";
const FOLDER: &str = "snapshots";

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct CloseCheck {
    /// What has to be done first: drafts, undated documents.
    pub blockers: Vec<String>,
    /// What is allowed but worth knowing: missing invoices, unanswered questions.
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SnapshotKennzahl {
    pub kz: String,
    pub name: String,
    pub copy: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SnapshotDocument {
    pub id: String,
    pub title: String,
    pub file: String,
    pub hash: String,
    pub kz: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SnapshotRow {
    pub id: String,
    pub year: i32,
    pub closed_at: i64,
    pub ruleset: String,
    pub total: String,
    /// `SnapshotKennzahl`s as JSON.
    pub kennzahlen: String,
    /// `SnapshotDocument`s as JSON, depreciation slices among them.
    pub documents: String,
    /// The summary PDF, relative to the data folder.
    pub file: String,
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

pub fn check(page: &YearPage) -> CloseCheck {
    let count = |kind| page.attention.iter().filter(|a| a.kind == kind).count();
    let mut blockers = Vec::new();
    let drafts = count(AttentionKind::Draft);
    if drafts > 0 {
        blockers.push(format!("{} to finish", plural(drafts, "draft", "drafts")));
    }
    let undated = count(AttentionKind::Undated);
    if undated > 0 {
        blockers.push(format!(
            "{} without a payment date",
            plural(undated, "document", "documents")
        ));
    }
    let mut warnings = Vec::new();
    let missing = count(AttentionKind::MissingInvoice);
    if missing > 0 {
        warnings.push(format!(
            "{} without the invoice won't count",
            plural(missing, "payment", "payments")
        ));
    }
    let open = page.questions.iter().filter(|q| !q.answered).count();
    if open > 0 {
        warnings.push(format!(
            "{} unanswered",
            plural(open, "question", "questions")
        ));
    }
    CloseCheck { blockers, warnings }
}

fn year_row(db: &Db, year: i32) -> AppResult<TaxYearRow> {
    db.read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(format!("No tax year {year}")))
}

/// The first free `<year>-summary[-n].pdf` in `snapshots/`.
fn free_name(config_dir: &Path, year: i32) -> String {
    let mut name = format!("{FOLDER}/{year}-summary.pdf");
    let mut n = 2;
    while config_dir.join(&name).exists() {
        name = format!("{FOLDER}/{year}-summary-{n}.pdf");
        n += 1;
    }
    name
}

/// Writes the summary PDF; returns its path relative to the data folder.
fn write_summary(config_dir: &Path, page: &YearPage, stamp: &str) -> AppResult<String> {
    let file = free_name(config_dir, page.year);
    let path = config_dir.join(&file);
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    std::fs::write(&path, report::summary_pdf(page, stamp))?;
    Ok(file)
}

fn snapshot_kennzahlen(page: &YearPage) -> Vec<SnapshotKennzahl> {
    page.kennzahlen
        .iter()
        .filter(|k| !k.reported)
        .map(|k| SnapshotKennzahl {
            kz: k.kz.clone(),
            name: k.name.clone(),
            copy: k.copy.clone(),
        })
        .collect()
}

fn snapshot_documents(db: &Db, page: &YearPage) -> AppResult<Vec<SnapshotDocument>> {
    let all = db.read(documents::documents)?;
    Ok(page
        .kennzahlen
        .iter()
        .flat_map(|kz| kz.rows.iter().map(move |row| (kz, row)))
        .filter(|(_, row)| row.kind == RowKind::Document)
        .filter_map(|(kz, row)| {
            let doc = all.iter().find(|d| d.id == row.id)?;
            Some(SnapshotDocument {
                id: doc.id.clone(),
                title: row.title.clone(),
                file: doc.file.clone(),
                hash: doc.file_hash.clone(),
                kz: kz.kz.clone(),
                amount: row.amount.clone(),
            })
        })
        .collect())
}

/// Closes an open year: writes its summary PDF and the snapshot row in one op; `stamp` dates the summary.
#[allow(clippy::too_many_arguments)]
pub fn close(
    db: &Db,
    config_dir: &Path,
    year: i32,
    number: NumberFormat,
    order: DateOrder,
    stamp: &str,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let before = year_row(db, year)?;
    if before.state == YearState::Closed {
        return Err(AppError::InvalidInput(format!(
            "Tax year {year} is closed already"
        )));
    }
    let page = page::page(db, year, "", number, order)?;
    let blockers = check(&page).blockers;
    if !blockers.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Before closing: {}",
            blockers.join(", ")
        )));
    }
    let file = write_summary(config_dir, &page, stamp)?;
    let snapshot = SnapshotRow {
        id: uuid::Uuid::new_v4().to_string(),
        year,
        closed_at: now,
        ruleset: before.ruleset.clone(),
        total: page.total.clone(),
        kennzahlen: serde_json::to_string(&snapshot_kennzahlen(&page))?,
        documents: serde_json::to_string(&snapshot_documents(db, &page)?)?,
        file,
    };
    let after = TaxYearRow {
        state: YearState::Closed,
        closed_at: Some(now),
        ..before.clone()
    };
    let label = if crate::yearcheck::close_needs_run(db, year)? {
        format!("Close Tax Year {year} without year check")
    } else {
        format!("Close Tax Year {year}")
    };
    db.write(
        Op::new(
            label,
            vec![
                Change::insert(SNAPSHOTS, &snapshot)?,
                Change::update(taxyears::TABLE, &before, &after)?,
            ],
        ),
        source,
    )
}

/// Opens a closed year again; its snapshots stay.
pub fn reopen(db: &Db, year: i32, source: Source) -> AppResult<Written> {
    let before = year_row(db, year)?;
    if before.state == YearState::Open {
        return Err(AppError::InvalidInput(format!("Tax year {year} is open")));
    }
    let after = TaxYearRow {
        state: YearState::Open,
        closed_at: None,
        ..before.clone()
    };
    db.write(
        Op::new(
            format!("Reopen Tax Year {year}"),
            vec![Change::update(taxyears::TABLE, &before, &after)?],
        ),
        source,
    )
}

/// The newest snapshot's summary PDF of a year.
pub fn snapshot_file(db: &Db, config_dir: &Path, year: i32) -> AppResult<PathBuf> {
    let file: Option<String> = db.read(|c| {
        Ok(c.query_row(
            &format!("SELECT file FROM {SNAPSHOTS} WHERE year = ? ORDER BY closed_at DESC LIMIT 1"),
            [year],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    file.map(|f| config_dir.join(f))
        .ok_or_else(|| AppError::NotFound(format!("Tax year {year} has no snapshot")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::store::DocState;
    use crate::platform::pdf::Pdf;
    use crate::statements::file_hash;
    use crate::year::page::{AttentionView, QuestionView};
    use crate::year::tests::{add_document, open_year};

    const STAMP: &str = "Closed 13 Nov 2026";

    fn bare_page(attention: &[AttentionKind], questions: &[bool]) -> YearPage {
        YearPage {
            year: 2026,
            ruleset: "2026 v1".into(),
            state: YearState::Open,
            total: "€0.00".into(),
            progress: String::new(),
            progress_percent: 0,
            pauschale_note: None,
            questions: questions
                .iter()
                .enumerate()
                .map(|(i, answered)| QuestionView {
                    id: format!("q{i}"),
                    label: format!("Question {i}"),
                    help: None,
                    unit: None,
                    value: String::new(),
                    answered: *answered,
                })
                .collect(),
            attention: attention
                .iter()
                .enumerate()
                .map(|(i, kind)| AttentionView {
                    kind: *kind,
                    title: format!("Entry {i}"),
                    detail: String::new(),
                    document_id: None,
                    entry_id: None,
                    line_id: None,
                })
                .collect(),
            kennzahlen: Vec::new(),
            months: Vec::new(),
            archived: 0,
        }
    }

    fn finished(db: &Db, id: &str, item: &str, date: &str, cents: i64, share: i64) {
        add_document(db, id, |d| {
            d.item_key = Some(item.into());
            d.pay_date = Some(date.into());
            d.paid_cents = Some(cents);
            d.work_share = share;
        });
    }

    fn close_2026(db: &Db, dir: &Path, now: i64) -> AppResult<Written> {
        close(
            db,
            dir,
            2026,
            NumberFormat::DotDecimal,
            DateOrder::DayFirst,
            STAMP,
            now,
            Source::Ui,
        )
    }

    fn year_page(db: &Db) -> YearPage {
        page::page(db, 2026, "", NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap()
    }

    fn year_state(db: &Db) -> (YearState, Option<i64>) {
        let row = db.read(|c| taxyears::row(c, 2026)).unwrap().unwrap();
        (row.state, row.closed_at)
    }

    fn snapshots(db: &Db) -> Vec<SnapshotRow> {
        db.read(|c| {
            let mut query = c.prepare(&format!(
                "SELECT id, year, closed_at, ruleset, total, kennzahlen, documents, file
                 FROM {SNAPSHOTS} ORDER BY closed_at"
            ))?;
            let rows = query.query_map([], |r| {
                Ok(SnapshotRow {
                    id: r.get(0)?,
                    year: r.get(1)?,
                    closed_at: r.get(2)?,
                    ruleset: r.get(3)?,
                    total: r.get(4)?,
                    kennzahlen: r.get(5)?,
                    documents: r.get(6)?,
                    file: r.get(7)?,
                })
            })?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap()
    }

    fn two_documents_year() -> (tempfile::TempDir, Db) {
        let (dir, db) = open_year();
        finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
        finished(&db, "doc-b", "phone", "2026-11-02", 12_000, 60);
        (dir, db)
    }

    #[test]
    fn a_year_without_attention_has_neither_blockers_nor_warnings() {
        assert_eq!(
            check(&bare_page(&[], &[true])),
            CloseCheck {
                blockers: vec![],
                warnings: vec![]
            }
        );
    }

    #[test]
    fn drafts_and_undated_documents_block_and_missing_invoices_and_open_questions_warn() {
        use AttentionKind::{Draft, MissingInvoice, Undated};
        let page = bare_page(
            &[Draft, Undated, MissingInvoice, Draft, MissingInvoice],
            &[false, true, false],
        );

        assert_eq!(
            check(&page),
            CloseCheck {
                blockers: vec![
                    "2 drafts to finish".into(),
                    "1 document without a payment date".into()
                ],
                warnings: vec![
                    "2 payments without the invoice won't count".into(),
                    "2 questions unanswered".into()
                ],
            }
        );
    }

    #[test]
    fn counts_of_one_use_the_singular() {
        use AttentionKind::{Draft, MissingInvoice, Undated};
        let page = bare_page(&[Draft, Undated, MissingInvoice], &[false]);

        assert_eq!(
            check(&page),
            CloseCheck {
                blockers: vec![
                    "1 draft to finish".into(),
                    "1 document without a payment date".into()
                ],
                warnings: vec![
                    "1 payment without the invoice won't count".into(),
                    "1 question unanswered".into()
                ],
            }
        );
    }

    #[test]
    fn several_undated_documents_use_the_plural() {
        let page = bare_page(&[AttentionKind::Undated, AttentionKind::Undated], &[]);

        assert_eq!(
            check(&page).blockers,
            vec!["2 documents without a payment date".to_string()]
        );
    }

    #[test]
    fn the_check_serializes_in_camel_case() {
        assert_eq!(
            serde_json::to_value(check(&bare_page(&[AttentionKind::Draft], &[false]))).unwrap(),
            serde_json::json!({
                "blockers": ["1 draft to finish"],
                "warnings": ["1 question unanswered"]
            })
        );
    }

    #[test]
    fn closing_marks_the_year_closed_with_the_time_and_labels_the_op() {
        let (dir, db) = two_documents_year();

        let written = close_2026(&db, dir.path(), 500).unwrap();

        assert_eq!(written.label, "Close Tax Year 2026");
        assert_eq!(year_state(&db), (YearState::Closed, Some(500)));
    }

    #[test]
    fn closing_labels_the_op_without_year_check_unless_one_ran_after_31_dec() {
        let mut asking = crate::year::tests::rules();
        crate::test_support::rules::set(
            &mut asking,
            "/items/0/ask",
            serde_json::json!({ "order": 1, "question": "Did you buy books?" }),
        );
        let cases = [
            (None, "Close Tax Year 2026 without year check"),
            (
                Some((2026, 11, 14)),
                "Close Tax Year 2026 without year check",
            ),
            (Some((2027, 1, 9)), "Close Tax Year 2026"),
        ];
        for (ran, label) in cases {
            let (dir, db) = crate::year::tests::open_year_with(&asking);
            if let Some((y, m, d)) = ran {
                let at = crate::test_support::rules::local_millis(y, m, d);
                crate::yearcheck::complete(&db, 2026, at, Source::Ui).unwrap();
            }

            let written = close_2026(&db, dir.path(), 500).unwrap();

            assert_eq!(written.label, label, "{ran:?}");
        }
    }

    #[test]
    fn closing_stores_a_snapshot_of_what_the_year_counted() {
        let (dir, db) = two_documents_year();

        close_2026(&db, dir.path(), 500).unwrap();

        let stored = snapshots(&db);
        assert_eq!(stored.len(), 1);
        let snapshot = &stored[0];
        assert_eq!(snapshot.year, 2026);
        assert_eq!(snapshot.closed_at, 500);
        assert_eq!(snapshot.ruleset, "2026 v1");
        assert_eq!(snapshot.total, "€118.90");
        assert_eq!(snapshot.file, "snapshots/2026-summary.pdf");
        assert_eq!(
            serde_json::from_str::<Vec<SnapshotKennzahl>>(&snapshot.kennzahlen).unwrap(),
            [
                ("720", "Books", "46,90"),
                ("721", "Phone", "72,00"),
                ("722", "Tips", "0,00"),
                ("724", "Parking", "0,00"),
            ]
            .map(|(kz, name, copy)| SnapshotKennzahl {
                kz: kz.into(),
                name: name.into(),
                copy: copy.into()
            })
        );
        assert_eq!(
            serde_json::from_str::<Vec<SnapshotDocument>>(&snapshot.documents).unwrap(),
            [("doc-a", "720", "€46.90"), ("doc-b", "721", "€72.00")].map(|(id, kz, amount)| {
                SnapshotDocument {
                    id: id.into(),
                    title: id.into(),
                    file: format!("docs/inbox/{id}.pdf"),
                    hash: file_hash(id.as_bytes()),
                    kz: kz.into(),
                    amount: amount.into(),
                }
            })
        );
    }

    #[test]
    fn closing_writes_the_summary_pdf_the_page_would_export() {
        let (dir, db) = two_documents_year();
        let page = year_page(&db);

        close_2026(&db, dir.path(), 500).unwrap();

        let bytes = std::fs::read(dir.path().join("snapshots/2026-summary.pdf")).unwrap();
        assert_eq!(bytes, report::summary_pdf(&page, STAMP));
        assert_eq!(Pdf::from_bytes(&bytes).unwrap().page_count(), 1);
    }

    #[test]
    fn unanswered_questions_and_missing_invoices_do_not_stop_a_close() {
        let (dir, db) = open_year();
        let before = year_page(&db);
        assert_eq!(check(&before).warnings, ["2 questions unanswered"]);

        close_2026(&db, dir.path(), 500).unwrap();

        assert_eq!(year_state(&db).0, YearState::Closed);
    }

    #[test]
    fn a_closed_year_cannot_be_closed_again() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();

        let err = close_2026(&db, dir.path(), 600).unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidInput("Tax year 2026 is closed already".into())
        );
        assert_eq!(snapshots(&db).len(), 1);
        assert_eq!(year_state(&db), (YearState::Closed, Some(500)));
    }

    #[test]
    fn blockers_refuse_the_close_and_leave_everything_untouched() {
        let (dir, db) = open_year();
        add_document(&db, "doc-d", |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
            d.pay_date = Some("2026-03-03".into());
        });
        add_document(&db, "doc-u", |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
        });

        let err = close_2026(&db, dir.path(), 500).unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidInput(
                "Before closing: 1 draft to finish, 1 document without a payment date".into()
            )
        );
        assert_eq!(year_state(&db), (YearState::Open, None));
        assert!(snapshots(&db).is_empty());
        assert!(!dir.path().join("snapshots").exists());
    }

    #[test]
    fn a_year_that_does_not_exist_is_not_found_to_close_or_reopen() {
        let (dir, db) = open_year();

        let closing = close(
            &db,
            dir.path(),
            2031,
            NumberFormat::DotDecimal,
            DateOrder::DayFirst,
            STAMP,
            1,
            Source::Ui,
        );

        assert!(matches!(closing, Err(AppError::NotFound(_))), "{closing:?}");
        assert!(matches!(
            reopen(&db, 2031, Source::Ui),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn the_summary_of_a_year_closed_again_is_numbered() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();
        reopen(&db, 2026, Source::Ui).unwrap();

        close_2026(&db, dir.path(), 600).unwrap();
        reopen(&db, 2026, Source::Ui).unwrap();
        close_2026(&db, dir.path(), 700).unwrap();

        let files: Vec<String> = snapshots(&db).into_iter().map(|s| s.file).collect();
        assert_eq!(
            files,
            [
                "snapshots/2026-summary.pdf",
                "snapshots/2026-summary-2.pdf",
                "snapshots/2026-summary-3.pdf"
            ]
        );
        for file in &files {
            assert!(dir.path().join(file).exists(), "{file}");
        }
    }

    #[test]
    fn undoing_a_close_reopens_the_year_and_drops_the_snapshot_but_keeps_the_file() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(year_state(&db), (YearState::Open, None));
        assert!(snapshots(&db).is_empty());
        assert!(dir.path().join("snapshots/2026-summary.pdf").exists());
    }

    #[test]
    fn reopening_opens_the_year_again_and_keeps_its_snapshot() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();

        let written = reopen(&db, 2026, Source::Ui).unwrap();

        assert_eq!(written.label, "Reopen Tax Year 2026");
        assert_eq!(year_state(&db), (YearState::Open, None));
        assert_eq!(snapshots(&db).len(), 1);
    }

    #[test]
    fn an_open_year_cannot_be_reopened() {
        let (_dir, db) = open_year();

        assert_eq!(
            reopen(&db, 2026, Source::Ui).unwrap_err(),
            AppError::InvalidInput("Tax year 2026 is open".into())
        );
    }

    #[test]
    fn undoing_a_reopen_closes_the_year_again_with_its_time() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();
        reopen(&db, 2026, Source::Ui).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(year_state(&db), (YearState::Closed, Some(500)));
    }

    #[test]
    fn the_snapshot_file_is_the_newest_summary_of_the_year() {
        let (dir, db) = two_documents_year();
        close_2026(&db, dir.path(), 500).unwrap();
        reopen(&db, 2026, Source::Ui).unwrap();
        close_2026(&db, dir.path(), 600).unwrap();

        assert_eq!(
            snapshot_file(&db, dir.path(), 2026).unwrap(),
            dir.path().join("snapshots/2026-summary-2.pdf")
        );
    }

    #[test]
    fn a_year_without_a_snapshot_has_no_snapshot_file() {
        let (dir, db) = two_documents_year();

        let err = snapshot_file(&db, dir.path(), 2026).unwrap_err();

        assert_eq!(
            err,
            AppError::NotFound("Tax year 2026 has no snapshot".into())
        );
    }
}
