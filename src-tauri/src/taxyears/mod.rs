//! Tax years: one row per year with its state and the rules text it applies. A year starts from its rules
//! file once the calendar reaches it; closing it freezes that text (ADR 0040).

pub mod commands;

use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::AppResult;
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

const COLUMNS: &str = "id, year, state, ruleset, rules, rules_loaded_at, closed_at";

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

/// Puts the rules folder in place, then starts the years the calendar has reached.
pub fn start_on_launch(
    db: &Db,
    config_dir: &Path,
    now: chrono::DateTime<chrono::Local>,
) -> AppResult<Option<Written>> {
    use chrono::Datelike;
    files::install(config_dir)?;
    start_from_rules(db, config_dir, now.year(), now.timestamp_millis())
}

/// Starts every year up to `current_year` that has a valid rules file but no row yet; an invalid file waits
/// for the owner (Settings → Tax Rules shows why).
pub fn start_from_rules(
    db: &Db,
    config_dir: &Path,
    current_year: i32,
    now: i64,
) -> AppResult<Option<Written>> {
    let existing: Vec<i32> = db.read(rows)?.iter().map(|r| r.year).collect();
    let mut changes = Vec::new();
    for year in files::list(config_dir)?.into_keys() {
        if year > current_year || existing.contains(&year) {
            continue;
        }
        let Some(text) = files::read(config_dir, year)? else {
            continue;
        };
        match check::read(&text, &BTreeSet::new()) {
            Ok(ruleset) if ruleset.year == year => {
                let row = TaxYearRow::new(year, ruleset.ruleset, text, now);
                changes.push(Change::insert(TABLE, &row)?);
            }
            Ok(_) => log::warn!("{} names another year", files::file_name(year)),
            Err(problems) => log::warn!(
                "{} isn't valid: {}",
                files::file_name(year),
                problems.join("; ")
            ),
        }
    }
    if changes.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        db.write(Op::new("Start tax year", changes), Source::App)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::json_file::create_private_dir;
    use crate::test_support::rules::{minimal_rules, minimal_text, set};
    use crate::test_support::temp_config_dir;
    use serde_json::json;

    fn setup() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    fn put_file(config: &Path, year: i32, text: &str) {
        let folder = files::dir(config);
        create_private_dir(&folder).unwrap();
        std::fs::write(folder.join(files::file_name(year)), text).unwrap();
    }

    fn years(db: &Db) -> Vec<(i32, YearState)> {
        db.read(rows)
            .unwrap()
            .iter()
            .map(|r| (r.year, r.state))
            .collect()
    }

    #[test]
    fn every_valid_file_up_to_the_current_year_starts_an_open_year() {
        let (dir, db) = setup();
        for year in [2025, 2026, 2027] {
            put_file(dir.path(), year, &minimal_text(year));
        }

        let written = start_from_rules(&db, dir.path(), 2026, 1000)
            .unwrap()
            .unwrap();

        assert_eq!(
            written,
            Written {
                label: "Start tax year".into(),
                tables: vec!["tax_years".into()]
            }
        );
        assert_eq!(
            db.read(rows).unwrap(),
            vec![
                TaxYearRow {
                    id: "2026".into(),
                    year: 2026,
                    state: YearState::Open,
                    ruleset: "2026 v1".into(),
                    rules: minimal_text(2026),
                    rules_loaded_at: 1000,
                    closed_at: None,
                },
                TaxYearRow {
                    id: "2025".into(),
                    year: 2025,
                    state: YearState::Open,
                    ruleset: "2025 v1".into(),
                    rules: minimal_text(2025),
                    rules_loaded_at: 1000,
                    closed_at: None,
                },
            ]
        );
    }

    #[test]
    fn starting_years_is_not_undoable() {
        let (dir, db) = setup();
        put_file(dir.path(), 2026, &minimal_text(2026));
        start_from_rules(&db, dir.path(), 2026, 1000).unwrap();

        assert_eq!(db.undo().unwrap(), None);
        assert_eq!(years(&db), vec![(2026, YearState::Open)]);
    }

    #[test]
    fn a_future_year_stays_a_draft() {
        let (dir, db) = setup();
        put_file(dir.path(), 2027, &minimal_text(2027));

        assert_eq!(start_from_rules(&db, dir.path(), 2026, 1000).unwrap(), None);
        assert_eq!(years(&db), vec![]);
    }

    #[test]
    fn a_year_that_has_a_row_is_left_as_it_is() {
        let (dir, db) = setup();
        put_file(dir.path(), 2026, &minimal_text(2026));
        start_from_rules(&db, dir.path(), 2026, 1000).unwrap();
        let before = db.read(rows).unwrap();
        let mut changed = minimal_rules(2026);
        set(&mut changed, "/settings/gwgLimit", json!(1200.00));
        put_file(dir.path(), 2026, &changed.to_string());

        assert_eq!(start_from_rules(&db, dir.path(), 2030, 2000).unwrap(), None);
        assert_eq!(db.read(rows).unwrap(), before);
    }

    #[test]
    fn a_new_file_starts_its_year_while_existing_rows_stay() {
        let (dir, db) = setup();
        put_file(dir.path(), 2025, &minimal_text(2025));
        start_from_rules(&db, dir.path(), 2025, 1000).unwrap();
        put_file(dir.path(), 2026, &minimal_text(2026));

        start_from_rules(&db, dir.path(), 2026, 2000)
            .unwrap()
            .unwrap();

        let loaded: Vec<(i32, i64)> = db
            .read(rows)
            .unwrap()
            .iter()
            .map(|r| (r.year, r.rules_loaded_at))
            .collect();
        assert_eq!(loaded, vec![(2026, 2000), (2025, 1000)]);
    }

    #[test]
    fn an_invalid_file_or_one_naming_another_year_waits_for_the_owner() {
        let (dir, db) = setup();
        put_file(dir.path(), 2023, "not json");
        let mut broken = minimal_rules(2024);
        set(&mut broken, "/schemaVersion", json!(2));
        put_file(dir.path(), 2024, &broken.to_string());
        put_file(dir.path(), 2025, &minimal_text(2026));
        put_file(dir.path(), 2026, &minimal_text(2026));

        start_from_rules(&db, dir.path(), 2026, 1000)
            .unwrap()
            .unwrap();

        assert_eq!(years(&db), vec![(2026, YearState::Open)]);
    }

    #[test]
    fn without_a_rules_folder_nothing_starts() {
        let (dir, db) = setup();
        assert_eq!(start_from_rules(&db, dir.path(), 2026, 1000).unwrap(), None);
    }

    #[test]
    fn the_shipped_seed_starts_its_year() {
        let (dir, db) = setup();
        files::install(dir.path()).unwrap();

        start_from_rules(&db, dir.path(), 2026, 1000)
            .unwrap()
            .unwrap();

        assert_eq!(
            list(&db).unwrap(),
            vec![TaxYear {
                year: 2026,
                state: YearState::Open,
                ruleset: "2026 v1".into()
            }]
        );
    }

    #[test]
    fn years_list_newest_first_and_a_single_year_can_be_read() {
        let (dir, db) = setup();
        for year in [2024, 2026, 2025] {
            put_file(dir.path(), year, &minimal_text(year));
        }
        start_from_rules(&db, dir.path(), 2026, 1000).unwrap();

        let listed: Vec<i32> = list(&db).unwrap().iter().map(|y| y.year).collect();
        assert_eq!(listed, vec![2026, 2025, 2024]);

        let row_2025 = db.read(|c| row(c, 2025)).unwrap().unwrap();
        assert_eq!(row_2025.ruleset, "2025 v1");
        assert_eq!(db.read(|c| row(c, 2030)).unwrap(), None);
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
