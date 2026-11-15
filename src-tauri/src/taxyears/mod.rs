//! Tax years: one row per year with its state and the rules text it applies. A year starts from its built-in
//! rules once the calendar reaches it; closing it freezes that text (ADR 0051).

pub mod commands;

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::taxrules::{check, files};

pub const TABLE: &str = "tax_years";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum YearState {
    Open,
    Closed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TaxYearRow {
    pub id: String,
    pub year: i32,
    pub state: YearState,
    /// The applied ruleset's name, e.g. `2026 v1`.
    pub ruleset: String,
    /// The applied rules file, verbatim.
    pub rules: String,
    pub rules_loaded_at: i64,
    pub closed_at: Option<i64>,
    /// Older journal entries predate it; their years used the shipped rules.
    #[serde(default)]
    pub rules_source: RulesSource,
}

/// Where a year's rules come from: the app's built-in copy, or the owner's version edited in Settings.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum RulesSource {
    #[default]
    Builtin,
    Own,
}

impl TaxYearRow {
    pub fn new(year: i32, ruleset: String, rules: String, now: i64) -> Self {
        Self {
            id: year.to_string(),
            year,
            state: YearState::Open,
            ruleset,
            rules,
            rules_loaded_at: now,
            closed_at: None,
            rules_source: RulesSource::Builtin,
        }
    }
}

/// What the sidebar lists.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct TaxYear {
    pub year: i32,
    pub state: YearState,
    pub ruleset: String,
}

const COLUMNS: &str = "id, year, state, ruleset, rules, rules_loaded_at, closed_at, rules_source";

fn from_sql(row: &rusqlite::Row) -> rusqlite::Result<TaxYearRow> {
    let state: String = row.get(2)?;
    Ok(TaxYearRow {
        id: row.get(0)?,
        year: row.get(1)?,
        state: if state == "closed" {
            YearState::Closed
        } else {
            YearState::Open
        },
        ruleset: row.get(3)?,
        rules: row.get(4)?,
        rules_loaded_at: row.get(5)?,
        closed_at: row.get(6)?,
        rules_source: if row.get::<_, String>(7)? == "own" {
            RulesSource::Own
        } else {
            RulesSource::Builtin
        },
    })
}

/// Every tax year, newest first.
pub fn rows(conn: &Connection) -> AppResult<Vec<TaxYearRow>> {
    let mut query = conn.prepare(&format!("SELECT {COLUMNS} FROM {TABLE} ORDER BY year DESC"))?;
    let rows = query.query_map([], from_sql)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn row(conn: &Connection, year: i32) -> AppResult<Option<TaxYearRow>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM {TABLE} WHERE year = ?"),
            [year],
            from_sql,
        )
        .optional()?)
}

/// The closed tax years, for the bulk re-deciders that leave their lines alone.
pub fn closed_years(conn: &Connection) -> AppResult<std::collections::HashSet<i32>> {
    Ok(rows(conn)?
        .into_iter()
        .filter(|r| r.state == YearState::Closed)
        .map(|r| r.year)
        .collect())
}

/// Whether a `yyyy-mm-dd` date falls in one of `closed`.
pub fn in_closed(closed: &std::collections::HashSet<i32>, date: &str) -> bool {
    date.get(..4)
        .and_then(|y| y.parse::<i32>().ok())
        .is_some_and(|y| closed.contains(&y))
}

/// Refuses a change that would touch a closed tax year: a payment date in it, before or after the change.
pub fn refuse_closed(db: &Db, dates: &[Option<&str>]) -> AppResult<()> {
    for year in dates
        .iter()
        .flatten()
        .filter_map(|d| d.get(..4)?.parse::<i32>().ok())
    {
        if db
            .read(|c| row(c, year))?
            .is_some_and(|r| r.state == YearState::Closed)
        {
            return Err(AppError::InvalidField {
                field: "document.payDate".into(),
                message: crate::tr!(
                    "Tax year {year} is closed; reopen it to change its documents",
                    "Das Steuerjahr {year} ist abgeschlossen; öffne es wieder, um seine Belege zu ändern"
                ),
            });
        }
    }
    Ok(())
}

pub fn list(db: &Db) -> AppResult<Vec<TaxYear>> {
    let rows = db.read(rows)?;
    Ok(rows
        .into_iter()
        .map(|r| TaxYear {
            year: r.year,
            state: r.state,
            ruleset: r.ruleset,
        })
        .collect())
}

/// Starts the years the calendar has reached that have built-in rules, then moves open years that follow the
/// built-in rules to a newer built-in version.
pub fn start_on_launch(db: &Db, now: chrono::DateTime<chrono::Local>) -> AppResult<Vec<Written>> {
    use chrono::Datelike;
    let millis = now.timestamp_millis();
    let mut written: Vec<Written> = start_builtin(db, now.year(), millis)?.into_iter().collect();
    written.extend(crate::taxrules::adopt_builtin(db, millis)?);
    Ok(written)
}

/// Starts every year up to `current_year` that has built-in rules but no row yet.
pub fn start_builtin(db: &Db, current_year: i32, now: i64) -> AppResult<Option<Written>> {
    let existing: Vec<i32> = db.read(rows)?.iter().map(|r| r.year).collect();
    let mut changes = Vec::new();
    for year in files::bundled_years() {
        if year > current_year || existing.contains(&year) {
            continue;
        }
        let Some(text) = files::bundled(year) else {
            continue;
        };
        match check::read(text, &BTreeSet::new()) {
            Ok(ruleset) if ruleset.year == year => {
                let row = TaxYearRow::new(year, ruleset.ruleset, text.to_string(), now);
                changes.push(Change::insert(TABLE, &row)?);
            }
            Ok(_) => log::warn!("The built-in rules for {year} name another year"),
            Err(problems) => log::warn!(
                "The built-in rules for {year} aren't valid: {}",
                problems.join("; ")
            ),
        }
    }
    if changes.is_empty() {
        return Ok(None);
    }
    Ok(Some(db.write(
        Op::new(crate::tr!("Start tax year", "Steuerjahr beginnen"), changes),
        Source::App,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rules::minimal_text;
    use crate::test_support::temp_config_dir;
    use chrono::TimeZone;

    fn setup() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    fn builtin_row(year: i32, now: i64) -> TaxYearRow {
        let text = files::bundled(year).unwrap();
        let ruleset = check::read(text, &BTreeSet::new()).unwrap().ruleset;
        TaxYearRow::new(year, ruleset, text.to_string(), now)
    }

    fn years(db: &Db) -> Vec<(i32, YearState)> {
        db.read(rows)
            .unwrap()
            .iter()
            .map(|r| (r.year, r.state))
            .collect()
    }

    #[test]
    fn every_built_in_year_up_to_the_current_year_starts_as_an_open_built_in_year() {
        let (_dir, db) = setup();

        let written = start_builtin(&db, 2026, 1000).unwrap().unwrap();

        assert_eq!(
            written,
            Written {
                label: "Start tax year".into(),
                tables: vec!["tax_years".into()]
            }
        );
        assert_eq!(
            db.read(rows).unwrap(),
            vec![builtin_row(2026, 1000), builtin_row(2025, 1000)]
        );
        assert_eq!(
            list(&db).unwrap(),
            vec![
                TaxYear {
                    year: 2026,
                    state: YearState::Open,
                    ruleset: "2026 v4".into()
                },
                TaxYear {
                    year: 2025,
                    state: YearState::Open,
                    ruleset: "2025 v3".into()
                }
            ]
        );
    }

    #[test]
    fn starting_years_is_not_undoable() {
        let (_dir, db) = setup();
        start_builtin(&db, 2026, 1000).unwrap();

        assert_eq!(db.undo().unwrap(), None);
        assert_eq!(
            years(&db),
            vec![(2026, YearState::Open), (2025, YearState::Open)]
        );
    }

    #[test]
    fn a_year_the_calendar_has_not_reached_is_not_started() {
        let (_dir, db) = setup();

        start_builtin(&db, 2025, 1000).unwrap().unwrap();

        assert_eq!(years(&db), vec![(2025, YearState::Open)]);
    }

    #[test]
    fn before_the_first_built_in_year_nothing_starts() {
        let (_dir, db) = setup();

        assert_eq!(start_builtin(&db, 2024, 1000).unwrap(), None);
        assert_eq!(years(&db), vec![]);
    }

    #[test]
    fn a_year_that_has_a_row_is_never_started_twice() {
        let (_dir, db) = setup();
        start_builtin(&db, 2026, 1000).unwrap();
        let before = db.read(rows).unwrap();

        assert_eq!(start_builtin(&db, 2030, 2000).unwrap(), None);
        assert_eq!(db.read(rows).unwrap(), before);
    }

    #[test]
    fn a_newly_reached_year_starts_while_existing_rows_stay() {
        let (_dir, db) = setup();
        start_builtin(&db, 2025, 1000).unwrap();

        start_builtin(&db, 2026, 2000).unwrap().unwrap();

        let loaded: Vec<(i32, i64)> = db
            .read(rows)
            .unwrap()
            .iter()
            .map(|r| (r.year, r.rules_loaded_at))
            .collect();
        assert_eq!(loaded, vec![(2026, 2000), (2025, 1000)]);
    }

    #[test]
    fn launching_in_a_year_starts_its_built_in_years_and_has_nothing_to_adopt() {
        let (_dir, db) = setup();
        let now = chrono::Local
            .with_ymd_and_hms(2026, 11, 15, 12, 0, 0)
            .unwrap();

        let written = start_on_launch(&db, now).unwrap();

        assert_eq!(
            written,
            vec![Written {
                label: "Start tax year".into(),
                tables: vec!["tax_years".into()]
            }]
        );
        assert_eq!(
            years(&db),
            vec![(2026, YearState::Open), (2025, YearState::Open)]
        );
    }

    #[test]
    fn launching_also_moves_an_open_built_in_year_to_a_newer_built_in_version() {
        let (_dir, db) = setup();
        let old = TaxYearRow::new(2026, "2026 v1".into(), minimal_text(2026), 1);
        db.write(
            Op::new("Add year", vec![Change::insert(TABLE, &old).unwrap()]),
            Source::App,
        )
        .unwrap();
        let now = chrono::Local
            .with_ymd_and_hms(2026, 11, 15, 12, 0, 0)
            .unwrap();

        let written = start_on_launch(&db, now).unwrap();

        let labels: Vec<&str> = written.iter().map(|w| w.label.as_str()).collect();
        assert_eq!(labels, ["Start tax year", "Update Tax Rules to 2026 v4"]);
        let row_2026 = db.read(|c| row(c, 2026)).unwrap().unwrap();
        assert_eq!(row_2026.ruleset, "2026 v4");
        assert_eq!(row_2026.rules, files::bundled(2026).unwrap());
    }

    #[test]
    fn years_list_newest_first_and_a_single_year_can_be_read() {
        let (_dir, db) = setup();
        start_builtin(&db, 2026, 1000).unwrap();

        let listed: Vec<i32> = list(&db).unwrap().iter().map(|y| y.year).collect();
        assert_eq!(listed, vec![2026, 2025]);

        let row_2025 = db.read(|c| row(c, 2025)).unwrap().unwrap();
        assert_eq!(row_2025.ruleset, "2025 v3");
        assert_eq!(db.read(|c| row(c, 2030)).unwrap(), None);
    }

    #[test]
    fn a_year_row_reads_its_rules_source_and_older_journal_rows_default_to_built_in() {
        let (_dir, db) = setup();
        let own = TaxYearRow {
            rules_source: RulesSource::Own,
            ..TaxYearRow::new(2026, "2026 v4 (own)".into(), minimal_text(2026), 1)
        };
        db.write(
            Op::new("Add year", vec![Change::insert(TABLE, &own).unwrap()]),
            Source::App,
        )
        .unwrap();

        assert_eq!(db.read(|c| row(c, 2026)).unwrap(), Some(own));

        let mut old_json =
            serde_json::to_value(TaxYearRow::new(2025, "x".into(), "{}".into(), 1)).unwrap();
        old_json.as_object_mut().unwrap().remove("rules_source");
        let old: TaxYearRow = serde_json::from_value(old_json).unwrap();
        assert_eq!(old.rules_source, RulesSource::Builtin);
    }

    fn add_years(db: &Db, states: &[(i32, YearState)]) {
        let changes = states
            .iter()
            .map(|(year, state)| {
                let row = TaxYearRow {
                    state: *state,
                    closed_at: (*state == YearState::Closed).then_some(5),
                    ..TaxYearRow::new(*year, format!("{year} v1"), minimal_text(*year), 1)
                };
                Change::insert(TABLE, &row).unwrap()
            })
            .collect();
        db.write(Op::new("Add years", changes), Source::App)
            .unwrap();
    }

    #[test]
    fn a_date_in_a_closed_year_is_refused_as_a_payment_date_field() {
        let (_dir, db) = setup();
        add_years(&db, &[(2025, YearState::Closed), (2026, YearState::Open)]);

        let err = refuse_closed(&db, &[Some("2025-12-31")]).unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidField {
                field: "document.payDate".into(),
                message: "Tax year 2025 is closed; reopen it to change its documents".into(),
            }
        );
    }

    #[test]
    fn any_one_date_in_a_closed_year_refuses_the_whole_change() {
        let (_dir, db) = setup();
        add_years(&db, &[(2025, YearState::Closed), (2026, YearState::Open)]);

        for dates in [
            [Some("2026-03-03"), Some("2025-01-01")],
            [Some("2025-01-01"), Some("2026-03-03")],
            [None, Some("2025-06-06")],
        ] {
            assert!(refuse_closed(&db, &dates).is_err(), "{dates:?}");
        }
    }

    #[test]
    fn dates_in_open_or_unknown_years_and_missing_dates_pass() {
        let (_dir, db) = setup();
        add_years(&db, &[(2025, YearState::Closed), (2026, YearState::Open)]);

        for dates in [
            vec![Some("2026-03-03")],
            vec![Some("2031-03-03")],
            vec![None],
            vec![None, None],
            vec![],
            vec![Some("n/a")],
        ] {
            assert_eq!(refuse_closed(&db, &dates), Ok(()), "{dates:?}");
        }
    }

    #[test]
    fn reopening_a_year_lets_its_dates_pass_again() {
        let (_dir, db) = setup();
        add_years(&db, &[(2025, YearState::Closed)]);
        let closed = db.read(|c| row(c, 2025)).unwrap().unwrap();
        let open = TaxYearRow {
            state: YearState::Open,
            closed_at: None,
            ..closed.clone()
        };
        db.write(
            Op::new(
                "Reopen",
                vec![Change::update(TABLE, &closed, &open).unwrap()],
            ),
            Source::App,
        )
        .unwrap();

        assert_eq!(refuse_closed(&db, &[Some("2025-05-05")]), Ok(()));
    }

    #[test]
    fn closed_years_are_the_years_whose_state_is_closed() {
        let (_dir, db) = setup();
        add_years(
            &db,
            &[
                (2024, YearState::Closed),
                (2025, YearState::Closed),
                (2026, YearState::Open),
            ],
        );

        assert_eq!(
            db.read(closed_years).unwrap(),
            std::collections::HashSet::from([2024, 2025])
        );
    }

    #[test]
    fn no_year_is_closed_without_tax_years_or_while_all_are_open() {
        let (_dir, db) = setup();
        assert!(db.read(closed_years).unwrap().is_empty());

        add_years(&db, &[(2026, YearState::Open)]);

        assert!(db.read(closed_years).unwrap().is_empty());
    }

    #[test]
    fn a_date_is_in_a_closed_year_by_its_first_four_digits() {
        let closed = std::collections::HashSet::from([2025]);
        for (date, expected) in [
            ("2025-01-01", true),
            ("2025-12-31", true),
            ("2024-12-31", false),
            ("2026-01-01", false),
            ("2025", true),
            ("202", false),
            ("n/a", false),
            ("", false),
        ] {
            assert_eq!(in_closed(&closed, date), expected, "{date:?}");
        }
    }

    #[test]
    fn no_date_is_in_a_closed_year_when_none_is_closed() {
        assert!(!in_closed(&std::collections::HashSet::new(), "2025-06-06"));
    }

    #[test]
    fn a_closed_year_lists_as_closed() {
        let (_dir, db) = setup();
        let closed = TaxYearRow {
            state: YearState::Closed,
            closed_at: Some(5),
            ..TaxYearRow::new(2025, "2025 v1".into(), minimal_text(2025), 1)
        };
        let change = Change::insert(TABLE, &closed).unwrap();
        db.write(Op::new("Add year", vec![change]), Source::App)
            .unwrap();

        assert_eq!(
            list(&db).unwrap(),
            vec![TaxYear {
                year: 2025,
                state: YearState::Closed,
                ruleset: "2025 v1".into()
            }]
        );
        assert_eq!(db.read(|c| row(c, 2025)).unwrap(), Some(closed));
    }
}
