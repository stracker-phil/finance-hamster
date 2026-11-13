use std::path::Path;

use serde_json::{Value, json};

use super::*;
use crate::store::json_file::create_private_dir;
use crate::test_support::rules::{minimal_rules, minimal_text, rules_text, set};
use crate::test_support::temp_config_dir;

fn setup() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn date(ms: i64) -> String {
    format!("day {ms}")
}

fn put_file(config: &Path, year: i32, text: &str) {
    let folder = files::dir(config);
    create_private_dir(&folder).unwrap();
    std::fs::write(folder.join(files::file_name(year)), text).unwrap();
}

fn file_text(config: &Path, year: i32) -> Option<String> {
    files::read(config, year).unwrap()
}

fn add_row(db: &Db, row: &TaxYearRow) {
    let change = Change::insert(taxyears::TABLE, row).unwrap();
    db.write(Op::new("Add year", vec![change]), Source::App)
        .unwrap();
}

fn open_row(db: &Db, year: i32, text: &str, loaded_at: i64) -> TaxYearRow {
    let row = TaxYearRow::new(year, format!("{year} v1"), text.to_string(), loaded_at);
    add_row(db, &row);
    row
}

fn closed_row(db: &Db, year: i32, text: &str, closed_at: i64) -> TaxYearRow {
    let row = TaxYearRow {
        state: YearState::Closed,
        closed_at: Some(closed_at),
        ..TaxYearRow::new(year, format!("{year} v1"), text.to_string(), 1)
    };
    add_row(db, &row);
    row
}

fn stored(db: &Db, year: i32) -> TaxYearRow {
    db.read(|c| taxyears::row(c, year)).unwrap().unwrap()
}

/// The year's rules as a second version with another Geringwertige-Wirtschaftsgüter limit.
fn v2(year: i32, gwg_limit: f64) -> Value {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/ruleset", json!(format!("{year} v2")));
    set(&mut rules, "/settings/gwgLimit", json!(gwg_limit));
    rules
}

fn invalid_text(year: i32) -> String {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/schemaVersion", json!(2));
    rules_text(&rules)
}

const SCHEMA_PROBLEM: &str = "schemaVersion: this Hamster reads version 1, not 2";

#[test]
fn a_file_without_a_tax_year_row_is_a_draft() {
    let (dir, db) = setup();
    let mut rules = minimal_rules(2027);
    set(&mut rules, "/basedOn", json!("2026 v1"));
    put_file(dir.path(), 2027, &rules_text(&rules));

    assert_eq!(
        overview(&db, dir.path(), &date).unwrap(),
        vec![RulesFile {
            file_name: "rules-2027.json".into(),
            year: 2027,
            state: RulesState::Draft,
            ruleset: Some("2027 v1".into()),
            based_on: Some("2026 v1".into()),
            items: 1,
            unverified: 0,
            problems: vec![],
            edited: false,
            missing: false,
            closed_on: None,
            loaded_on: None,
        }]
    );
}

#[test]
fn an_open_year_whose_file_matches_what_it_applies_is_not_edited() {
    let (dir, db) = setup();
    let text = minimal_text(2026);
    put_file(dir.path(), 2026, &text);
    open_row(&db, 2026, &text, 5);

    assert_eq!(
        overview(&db, dir.path(), &date).unwrap(),
        vec![RulesFile {
            file_name: "rules-2026.json".into(),
            year: 2026,
            state: RulesState::Open,
            ruleset: Some("2026 v1".into()),
            based_on: None,
            items: 1,
            unverified: 0,
            problems: vec![],
            edited: false,
            missing: false,
            closed_on: None,
            loaded_on: Some("day 5".into()),
        }]
    );
}

#[test]
fn years_list_newest_first_with_their_state_from_the_row() {
    let (dir, db) = setup();
    for year in [2024, 2025, 2026, 2027] {
        put_file(dir.path(), year, &minimal_text(year));
    }
    closed_row(&db, 2025, &minimal_text(2025), 500);
    open_row(&db, 2026, &minimal_text(2026), 5);

    let rows = overview(&db, dir.path(), &date).unwrap();

    let shown: Vec<(i32, RulesState)> = rows.iter().map(|r| (r.year, r.state)).collect();
    assert_eq!(
        shown,
        vec![
            (2027, RulesState::Draft),
            (2026, RulesState::Open),
            (2025, RulesState::Closed),
            (2024, RulesState::Draft),
        ]
    );
}

#[test]
fn nothing_to_list_gives_an_empty_overview() {
    let (dir, db) = setup();
    assert_eq!(overview(&db, dir.path(), &date).unwrap(), vec![]);
}

#[test]
fn an_open_year_whose_file_changed_is_edited_and_shows_the_files_ruleset() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 5);
    put_file(dir.path(), 2026, &rules_text(&v2(2026, 1200.0)));

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert_eq!(rows.len(), 1);
    assert!(rows[0].edited);
    assert_eq!(rows[0].ruleset.as_deref(), Some("2026 v2"));
    assert_eq!(rows[0].loaded_on.as_deref(), Some("day 5"));
    assert!(rows[0].problems.is_empty());
}

#[test]
fn a_year_whose_file_is_gone_is_missing_and_keeps_its_applied_ruleset() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 5);

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert_eq!(rows.len(), 1);
    assert!(rows[0].missing);
    assert!(!rows[0].edited);
    assert_eq!(rows[0].state, RulesState::Open);
    assert_eq!(rows[0].ruleset.as_deref(), Some("2026 v1"));
    assert_eq!(rows[0].items, 0);
}

#[test]
fn an_invalid_file_lists_its_problems_and_has_no_item_count() {
    let (dir, db) = setup();
    put_file(dir.path(), 2027, &invalid_text(2027));

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert_eq!(rows[0].problems, vec![SCHEMA_PROBLEM.to_string()]);
    assert_eq!(rows[0].ruleset, None);
    assert_eq!(rows[0].items, 0);
    assert!(!rows[0].missing);
}

#[test]
fn an_invalid_file_of_an_open_year_keeps_the_applied_ruleset_and_counts_as_edited() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 5);
    put_file(dir.path(), 2026, &invalid_text(2026));

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert_eq!(rows[0].problems, vec![SCHEMA_PROBLEM.to_string()]);
    assert_eq!(rows[0].ruleset.as_deref(), Some("2026 v1"));
    assert!(rows[0].edited);
}

#[test]
fn a_file_naming_another_year_than_its_file_name_is_a_problem() {
    let (dir, db) = setup();
    put_file(dir.path(), 2026, &minimal_text(2027));

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert_eq!(
        rows[0].problems,
        vec!["year: the file says 2027, its name says 2026".to_string()]
    );
}

#[test]
fn a_closed_year_is_described_from_its_frozen_rules_whatever_the_file_says() {
    for file in [
        "not json".to_string(),
        rules_text(&v2(2026, 1200.0)),
        invalid_text(2026),
    ] {
        let (dir, db) = setup();
        closed_row(&db, 2026, &minimal_text(2026), 500);
        put_file(dir.path(), 2026, &file);

        let rows = overview(&db, dir.path(), &date).unwrap();

        assert_eq!(
            rows,
            vec![RulesFile {
                file_name: "rules-2026.json".into(),
                year: 2026,
                state: RulesState::Closed,
                ruleset: Some("2026 v1".into()),
                based_on: None,
                items: 1,
                unverified: 0,
                problems: vec![],
                edited: false,
                missing: false,
                closed_on: Some("day 500".into()),
                loaded_on: None,
            }],
            "{file}"
        );
    }
}

#[test]
fn a_closed_year_without_its_file_is_still_described_from_the_frozen_copy() {
    let (dir, db) = setup();
    closed_row(&db, 2026, &minimal_text(2026), 500);

    let rows = overview(&db, dir.path(), &date).unwrap();

    assert!(rows[0].missing);
    assert_eq!(rows[0].ruleset.as_deref(), Some("2026 v1"));
    assert_eq!(rows[0].items, 1);
    assert!(rows[0].problems.is_empty());
}

#[test]
fn unverified_entries_are_counted_for_the_file() {
    let (dir, db) = setup();
    let mut rules = minimal_rules(2027);
    set(&mut rules, "/items/0/verified", json!(false));
    put_file(dir.path(), 2027, &rules_text(&rules));

    assert_eq!(overview(&db, dir.path(), &date).unwrap()[0].unverified, 1);
}

#[test]
fn preview_lists_only_open_years_whose_file_differs_from_what_they_apply() {
    let (dir, db) = setup();
    let same = minimal_text(2024);
    put_file(dir.path(), 2024, &same);
    open_row(&db, 2024, &same, 1);
    put_file(dir.path(), 2025, &rules_text(&v2(2025, 1200.0)));
    closed_row(&db, 2025, &minimal_text(2025), 2);
    put_file(dir.path(), 2027, &rules_text(&v2(2027, 1200.0)));
    open_row(&db, 2028, &minimal_text(2028), 1);

    assert_eq!(preview(&db, dir.path()).unwrap(), vec![]);
}

#[test]
fn preview_describes_the_changes_between_the_applied_rules_and_the_file() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2026, &rules_text(&v2(2026, 1200.0)));

    assert_eq!(
        preview(&db, dir.path()).unwrap(),
        vec![ReloadPreview {
            year: 2026,
            from: "2026 v1".into(),
            to: Some("2026 v2".into()),
            changes: vec![
                "ruleset: \"2026 v1\" → \"2026 v2\"".into(),
                "settings.gwgLimit: 1000 → 1200".into(),
            ],
            problems: vec![],
        }]
    );
}

#[test]
fn preview_of_an_invalid_file_has_problems_and_no_target() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2026, &invalid_text(2026));

    assert_eq!(
        preview(&db, dir.path()).unwrap(),
        vec![ReloadPreview {
            year: 2026,
            from: "2026 v1".into(),
            to: None,
            changes: vec![],
            problems: vec![SCHEMA_PROBLEM.into()],
        }]
    );
}

#[test]
fn preview_of_a_file_naming_another_year_is_a_problem() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2026, &minimal_text(2027));

    let previews = preview(&db, dir.path()).unwrap();

    assert_eq!(
        previews[0].problems,
        vec!["year: the file says 2027, its name says 2026".to_string()]
    );
    assert_eq!(previews[0].to, None);
}

#[test]
fn a_whitespace_only_edit_previews_as_a_formatting_change() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2026, &minimal_rules(2026).to_string());

    assert_eq!(
        preview(&db, dir.path()).unwrap(),
        vec![ReloadPreview {
            year: 2026,
            from: "2026 v1".into(),
            to: Some("2026 v1".into()),
            changes: vec!["Only the formatting changed".into()],
            problems: vec![],
        }]
    );
}

#[test]
fn unreadable_applied_rules_are_replaced_by_the_file() {
    let (dir, db) = setup();
    open_row(&db, 2026, "garbage", 1);
    put_file(dir.path(), 2026, &minimal_text(2026));

    let previews = preview(&db, dir.path()).unwrap();

    assert_eq!(
        previews[0].changes,
        vec!["The applied rules can't be read; the file replaces them".to_string()]
    );
    assert_eq!(previews[0].to.as_deref(), Some("2026 v1"));
}

#[test]
fn previews_come_oldest_year_first() {
    let (dir, db) = setup();
    for year in [2026, 2025] {
        open_row(&db, year, &minimal_text(year), 1);
        put_file(dir.path(), year, &rules_text(&v2(year, 1200.0)));
    }

    let years: Vec<i32> = preview(&db, dir.path())
        .unwrap()
        .iter()
        .map(|p| p.year)
        .collect();

    assert_eq!(years, vec![2025, 2026]);
}

#[test]
fn apply_loads_the_file_into_the_year_as_one_op() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    let new_text = rules_text(&v2(2026, 1200.0));
    put_file(dir.path(), 2026, &new_text);

    let written = apply(&db, dir.path(), 99).unwrap().unwrap();

    assert_eq!(
        written,
        Written {
            label: "Reload Tax Rules".into(),
            tables: vec!["tax_years".into()]
        }
    );
    assert_eq!(
        stored(&db, 2026),
        TaxYearRow {
            id: "2026".into(),
            year: 2026,
            state: YearState::Open,
            ruleset: "2026 v2".into(),
            rules: new_text,
            rules_loaded_at: 99,
            closed_at: None,
        }
    );
    assert_eq!(preview(&db, dir.path()).unwrap(), vec![]);
}

#[test]
fn undoing_a_reload_restores_the_old_rules() {
    let (dir, db) = setup();
    let before = open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2026, &rules_text(&v2(2026, 1200.0)));
    apply(&db, dir.path(), 99).unwrap();

    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.tables, vec!["tax_years".to_string()]);
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn every_pending_year_reloads_in_one_op_and_one_undo() {
    let (dir, db) = setup();
    let first = open_row(&db, 2025, &minimal_text(2025), 1);
    let second = open_row(&db, 2026, &minimal_text(2026), 1);
    for year in [2025, 2026] {
        put_file(dir.path(), year, &rules_text(&v2(year, 1200.0)));
    }

    apply(&db, dir.path(), 99).unwrap().unwrap();
    assert_eq!(stored(&db, 2025).ruleset, "2025 v2");
    assert_eq!(stored(&db, 2026).ruleset, "2026 v2");

    db.undo().unwrap().unwrap();
    assert_eq!(stored(&db, 2025), first);
    assert_eq!(stored(&db, 2026), second);
    assert_eq!(db.undo().unwrap(), None);
}

#[test]
fn a_whitespace_only_edit_is_loaded_too() {
    let (dir, db) = setup();
    open_row(&db, 2026, &minimal_text(2026), 1);
    let compact = minimal_rules(2026).to_string();
    put_file(dir.path(), 2026, &compact);

    apply(&db, dir.path(), 99).unwrap().unwrap();

    let row = stored(&db, 2026);
    assert_eq!(row.rules, compact);
    assert_eq!(row.ruleset, "2026 v1");
    assert_eq!(row.rules_loaded_at, 99);
}

#[test]
fn apply_is_refused_while_any_file_has_a_problem_and_changes_no_year() {
    let (dir, db) = setup();
    let fine = open_row(&db, 2025, &minimal_text(2025), 1);
    let broken = open_row(&db, 2026, &minimal_text(2026), 1);
    put_file(dir.path(), 2025, &rules_text(&v2(2025, 1200.0)));
    put_file(dir.path(), 2026, &invalid_text(2026));

    let err = apply(&db, dir.path(), 99).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput(format!("rules-2026.json can't be loaded: {SCHEMA_PROBLEM}"))
    );
    assert_eq!(stored(&db, 2025), fine);
    assert_eq!(stored(&db, 2026), broken);
}

#[test]
fn apply_with_nothing_changed_writes_nothing() {
    let (dir, db) = setup();
    let text = minimal_text(2026);
    put_file(dir.path(), 2026, &text);
    open_row(&db, 2026, &text, 1);

    assert_eq!(apply(&db, dir.path(), 99).unwrap(), None);
    assert_eq!(stored(&db, 2026).rules_loaded_at, 1);
    assert_eq!(db.undo().unwrap(), None);
}

#[test]
fn apply_leaves_closed_years_and_drafts_alone() {
    let (dir, db) = setup();
    let closed = closed_row(&db, 2025, &minimal_text(2025), 2);
    put_file(dir.path(), 2025, &rules_text(&v2(2025, 1200.0)));
    put_file(dir.path(), 2027, &rules_text(&v2(2027, 1200.0)));

    assert_eq!(apply(&db, dir.path(), 99).unwrap(), None);
    assert_eq!(stored(&db, 2025), closed);
    assert_eq!(db.read(|c| taxyears::row(c, 2027)).unwrap(), None);
}

#[test]
fn importing_a_file_with_problems_is_refused_and_writes_nothing() {
    let (dir, db) = setup();

    let err = import(&db, dir.path(), &invalid_text(2026)).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput(format!("The file can't be used: {SCHEMA_PROBLEM}"))
    );
    assert!(files::list(dir.path()).unwrap().is_empty());
}

#[test]
fn importing_text_that_is_not_a_rules_file_is_refused() {
    let (dir, db) = setup();
    for text in ["", "not json", "[]", r#"{"year":2026}"#] {
        let err = import(&db, dir.path(), text).unwrap_err();
        assert!(
            matches!(&err, AppError::InvalidInput(m) if m.starts_with("The file can't be used: ")),
            "{text}: {err:?}"
        );
    }
}

#[test]
fn importing_into_a_closed_year_is_refused_and_keeps_the_file() {
    let (dir, db) = setup();
    closed_row(&db, 2026, &minimal_text(2026), 2);
    put_file(dir.path(), 2026, &minimal_text(2026));

    let err = import(&db, dir.path(), &rules_text(&v2(2026, 1200.0))).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput("Tax year 2026 is closed; its rules are frozen".into())
    );
    assert_eq!(file_text(dir.path(), 2026), Some(minimal_text(2026)));
}

#[test]
fn importing_a_new_year_writes_its_file_without_a_backup_or_a_reload() {
    let (dir, db) = setup();
    create_private_dir(&files::dir(dir.path())).unwrap();
    let text = minimal_text(2027);

    let imported = import(&db, dir.path(), &text).unwrap();

    assert_eq!(
        imported,
        Imported {
            file_name: "rules-2027.json".into(),
            year: 2027,
            backup: None,
            needs_reload: false,
        }
    );
    assert_eq!(file_text(dir.path(), 2027), Some(text));
}

#[test]
fn importing_over_a_draft_keeps_the_old_file_under_its_version() {
    let (dir, db) = setup();
    put_file(dir.path(), 2027, &minimal_text(2027));
    let new_text = rules_text(&v2(2027, 1200.0));

    let imported = import(&db, dir.path(), &new_text).unwrap();

    assert_eq!(imported.backup.as_deref(), Some("rules-2027.v1.json"));
    assert!(!imported.needs_reload);
    assert_eq!(file_text(dir.path(), 2027), Some(new_text));
    let backup = files::dir(dir.path()).join("rules-2027.v1.json");
    assert_eq!(std::fs::read_to_string(backup).unwrap(), minimal_text(2027));
}

#[test]
fn importing_into_an_open_year_needs_a_reload_and_leaves_the_applied_rules_alone() {
    let (dir, db) = setup();
    let applied = open_row(&db, 2026, &minimal_text(2026), 1);
    let mut old = minimal_rules(2026);
    set(&mut old, "/ruleset", json!("2026 v3"));
    put_file(dir.path(), 2026, &rules_text(&old));
    let new_text = rules_text(&v2(2026, 1200.0));

    let imported = import(&db, dir.path(), &new_text).unwrap();

    assert_eq!(
        imported,
        Imported {
            file_name: "rules-2026.json".into(),
            year: 2026,
            backup: Some("rules-2026.v3.json".into()),
            needs_reload: true,
        }
    );
    assert_eq!(stored(&db, 2026), applied);
    assert_eq!(
        preview(&db, dir.path()).unwrap()[0].to.as_deref(),
        Some("2026 v2")
    );
}

#[test]
fn a_backup_of_an_unreadable_old_file_is_named_v0() {
    let (dir, db) = setup();
    put_file(dir.path(), 2026, "not json");

    let imported = import(&db, dir.path(), &minimal_text(2026)).unwrap();

    assert_eq!(imported.backup.as_deref(), Some("rules-2026.v0.json"));
}

#[test]
fn duplicate_copies_the_newest_open_year_to_the_next_year() {
    let (dir, db) = setup();
    for year in [2025, 2026] {
        put_file(dir.path(), year, &minimal_text(year));
        open_row(&db, year, &minimal_text(year), 1);
    }

    let name = duplicate(&db, dir.path()).unwrap();

    assert_eq!(name, "rules-2027.json");
    let text = file_text(dir.path(), 2027).unwrap();
    let ruleset = check::read(&text, &Default::default()).unwrap();
    assert_eq!(ruleset.year, 2027);
    assert_eq!(ruleset.ruleset, "2027 v1");
    assert_eq!(ruleset.based_on.as_deref(), Some("2026 v1"));
    assert_eq!(file_text(dir.path(), 2028), None);
}

#[test]
fn duplicate_copies_the_applied_rules_when_the_file_is_gone() {
    let (dir, db) = setup();
    create_private_dir(&files::dir(dir.path())).unwrap();
    open_row(&db, 2026, &minimal_text(2026), 1);

    assert_eq!(duplicate(&db, dir.path()).unwrap(), "rules-2027.json");
    assert_eq!(
        check::read(&file_text(dir.path(), 2027).unwrap(), &Default::default())
            .unwrap()
            .based_on
            .as_deref(),
        Some("2026 v1")
    );
}

#[test]
fn duplicate_refuses_when_the_next_years_file_exists() {
    let (dir, db) = setup();
    put_file(dir.path(), 2026, &minimal_text(2026));
    put_file(dir.path(), 2027, "mine");
    open_row(&db, 2026, &minimal_text(2026), 1);

    let err = duplicate(&db, dir.path()).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput("rules-2027.json already exists".into())
    );
    assert_eq!(file_text(dir.path(), 2027), Some("mine".to_string()));
}

#[test]
fn duplicate_without_an_open_year_is_refused() {
    let (dir, db) = setup();
    let expected = AppError::InvalidInput("There is no open tax year to copy".into());
    assert_eq!(duplicate(&db, dir.path()).unwrap_err(), expected);

    closed_row(&db, 2026, &minimal_text(2026), 2);
    assert_eq!(duplicate(&db, dir.path()).unwrap_err(), expected);
}
