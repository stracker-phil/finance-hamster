use serde_json::json;

use super::files;
use super::import::{RulesImport, import, preview};
use super::tests::{closed, insert, setup, stored, use_item, year_row};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::AppError;
use crate::settings::Language;
use crate::taxyears::{self, RulesSource, YearState};
use crate::test_support::rules::{minimal_rules, minimal_text, rules_text, set};

const SCHEMA_PROBLEM: &str = "schemaVersion: this Hamster reads version 1, not 2";
const FROZEN_2026: &str = "Tax year 2026 is closed; its rules are frozen";

fn open_minimal(db: &Db, year: i32) -> taxyears::TaxYearRow {
    insert(
        db,
        year_row(year, &minimal_rules(year), RulesSource::Builtin),
    )
}

fn with_gwg_limit(year: i32, ruleset: &str, limit: f64) -> String {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/ruleset", json!(ruleset));
    set(&mut rules, "/settings/gwgLimit", json!(limit));
    rules_text(&rules)
}

fn dropping_books(year: i32) -> String {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/items/0/key", json!("fees"));
    rules_text(&rules)
}

fn year_exists(db: &Db, year: i32) -> bool {
    db.read(|c| taxyears::row(c, year)).unwrap().is_some()
}

fn hide_category(db: &Db, year: i32, key: &str) {
    let row = crate::year::hidden::HiddenRow::new(year, key);
    let change = Change::insert(crate::year::hidden::HIDDEN, &row).unwrap();
    db.write(Op::new("Hide", vec![change]), Source::Ui).unwrap();
}

fn hidden_in(db: &Db, year: i32) -> Vec<String> {
    db.read(|c| crate::year::hidden::hidden(c, year))
        .unwrap()
        .into_iter()
        .collect()
}

#[test]
fn a_preview_of_unreadable_text_names_no_year_and_lists_problems() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    let mut broken = minimal_rules(2026);
    set(&mut broken, "/schemaVersion", json!(2));

    let not_json = preview(&db, "{".into()).unwrap();
    let wrong_schema = preview(&db, rules_text(&broken)).unwrap();

    assert_eq!(not_json.year, None);
    assert!(!not_json.problems.is_empty());
    assert!(!not_json.replaces);
    assert_eq!(
        wrong_schema,
        RulesImport {
            year: None,
            text: String::new(),
            replaces: false,
            problems: vec![SCHEMA_PROBLEM.into()],
            changes: vec![],
        }
    );
}

#[test]
fn a_preview_for_a_year_that_does_not_exist_yet_starts_it_without_changes() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    let text = minimal_text(2030);

    assert_eq!(
        preview(&db, text.clone()).unwrap(),
        RulesImport {
            year: Some(2030),
            text,
            replaces: false,
            problems: vec![],
            changes: vec![],
        }
    );
}

#[test]
fn a_preview_for_an_open_year_replaces_it_and_lists_what_changes() {
    let (_dir, db) = setup();
    let before = open_minimal(&db, 2026);
    let text = with_gwg_limit(2026, "2026 v1", 1200.0);

    assert_eq!(
        preview(&db, text.clone()).unwrap(),
        RulesImport {
            year: Some(2026),
            text,
            replaces: true,
            problems: vec![],
            changes: vec!["settings.gwgLimit: 1000 → 1200".into()],
        }
    );
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn a_preview_for_a_closed_year_says_its_rules_are_frozen() {
    let (_dir, db) = setup();
    insert(
        &db,
        closed(year_row(2026, &minimal_rules(2026), RulesSource::Builtin)),
    );

    let result = preview(&db, with_gwg_limit(2026, "2026 v1", 1200.0)).unwrap();

    assert_eq!(result.year, Some(2026));
    assert!(result.replaces);
    assert_eq!(result.problems, vec![FROZEN_2026.to_string()]);
}

#[test]
fn a_preview_for_an_existing_year_refuses_to_drop_an_item_recorded_data_uses() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    use_item(&db, "books");

    let result = preview(&db, dropping_books(2026)).unwrap();

    assert_eq!(
        result,
        RulesImport {
            year: Some(2026),
            text: String::new(),
            replaces: true,
            problems: vec![
                "items: \"books\" is missing, but entries use it; keys can't be removed or renamed"
                    .to_string()
            ],
            changes: vec![],
        }
    );
}

#[test]
fn a_preview_for_a_new_year_may_drop_items_other_years_use() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    use_item(&db, "books");

    let result = preview(&db, dropping_books(2027)).unwrap();

    assert_eq!(result.year, Some(2027));
    assert!(!result.replaces);
    assert_eq!(result.problems, Vec::<String>::new());
}

#[test]
fn a_missing_year_before_all_others_is_inserted_with_the_files_rules_as_the_owners_version() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    let text = with_gwg_limit(2020, "2020 v7", 900.0);

    let written = import(&db, &text, 99).unwrap();

    assert_eq!(
        written,
        Written {
            label: "Import the Tax Rules for 2020".into(),
            tables: vec!["tax_years".into()],
        }
    );
    let row = stored(&db, 2020);
    assert_eq!(row.ruleset, "2020 v7");
    assert_eq!(row.rules, text);
    assert_eq!(row.rules_source, RulesSource::Own);
    assert_eq!(row.state, YearState::Open);
    assert_eq!(row.rules_loaded_at, 99);
}

#[test]
fn a_missing_year_after_a_gap_is_inserted_and_leaves_the_gap_empty() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    let text = with_gwg_limit(2028, "2028 v7", 1100.0);

    import(&db, &text, 99).unwrap();

    let row = stored(&db, 2028);
    assert_eq!(row.ruleset, "2028 v7");
    assert_eq!(row.rules, text);
    assert_eq!(row.rules_source, RulesSource::Own);
    assert!(!year_exists(&db, 2027));
}

#[test]
fn the_import_label_is_german_in_the_german_interface() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);

    let written = crate::i18n::with_lang(Language::De, || {
        import(&db, &minimal_text(2028), 99).unwrap()
    });

    assert_eq!(written.label, "Steuerregeln für 2028 importieren");
}

#[test]
fn a_new_year_carries_over_the_hidden_categories_of_the_year_before() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    hide_category(&db, 2026, "books");
    hide_category(&db, 2025, "other");

    let written = import(&db, &minimal_text(2027), 99).unwrap();

    let mut tables = written.tables;
    tables.sort();
    assert_eq!(tables, ["hidden_items", "tax_years"]);
    assert_eq!(hidden_in(&db, 2027), ["books"]);
    assert_eq!(hidden_in(&db, 2026), ["books"]);
}

#[test]
fn a_new_year_after_a_gap_has_no_hidden_categories_to_carry() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    hide_category(&db, 2026, "books");

    import(&db, &minimal_text(2028), 99).unwrap();

    assert!(hidden_in(&db, 2028).is_empty());
}

#[test]
fn an_open_year_gets_its_rules_replaced_as_the_owners_version() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    let text = with_gwg_limit(2026, "2026 v7", 1200.0);

    let written = import(&db, &text, 99).unwrap();

    assert_eq!(
        written,
        Written {
            label: "Import the Tax Rules for 2026".into(),
            tables: vec!["tax_years".into()],
        }
    );
    let row = stored(&db, 2026);
    assert_eq!(row.ruleset, "2026 v7");
    assert_eq!(row.rules, text);
    assert_eq!(row.rules_source, RulesSource::Own);
    assert_eq!(row.rules_loaded_at, 99);
    assert_eq!(row.state, YearState::Open);
}

#[test]
fn a_file_identical_to_the_built_in_rules_follows_them_again() {
    let bundled = files::bundled(2026).unwrap();

    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    import(&db, bundled, 99).unwrap();
    let replaced = stored(&db, 2026);

    let (_dir, db) = setup();
    open_minimal(&db, 2025);
    import(&db, bundled, 99).unwrap();
    let inserted = stored(&db, 2026);

    for (row, case) in [(replaced, "replaced"), (inserted, "inserted")] {
        assert_eq!(row.rules, bundled, "{case}");
        assert_eq!(row.rules_source, RulesSource::Builtin, "{case}");
    }
}

#[test]
fn a_closed_year_refuses_the_import_and_changes_nothing() {
    let (_dir, db) = setup();
    let frozen = insert(
        &db,
        closed(year_row(2026, &minimal_rules(2026), RulesSource::Builtin)),
    );

    let err = import(&db, &with_gwg_limit(2026, "2026 v7", 1200.0), 99).unwrap_err();

    assert_eq!(err, AppError::InvalidInput(FROZEN_2026.into()));
    assert_eq!(stored(&db, 2026), frozen);
}

#[test]
fn invalid_text_is_refused_and_changes_nothing() {
    let (_dir, db) = setup();
    let before = open_minimal(&db, 2026);
    let mut broken = minimal_rules(2026);
    set(&mut broken, "/schemaVersion", json!(2));

    let err = import(&db, &rules_text(&broken), 99).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput(format!("These rules can't be used: {SCHEMA_PROBLEM}"))
    );
    assert!(matches!(
        import(&db, "{", 99).unwrap_err(),
        AppError::InvalidInput(_)
    ));
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn an_existing_year_cannot_drop_an_item_recorded_data_uses() {
    let (_dir, db) = setup();
    let before = open_minimal(&db, 2026);
    use_item(&db, "books");

    let err = import(&db, &dropping_books(2026), 99).unwrap_err();

    assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn undoing_the_import_of_a_new_year_removes_it_again() {
    let (_dir, db) = setup();
    open_minimal(&db, 2026);
    hide_category(&db, 2026, "books");
    import(&db, &minimal_text(2027), 99).unwrap();

    db.undo().unwrap().unwrap();

    assert!(!year_exists(&db, 2027));
    assert!(hidden_in(&db, 2027).is_empty());
    assert_eq!(hidden_in(&db, 2026), ["books"]);
}

#[test]
fn undoing_the_import_into_an_open_year_brings_the_previous_rules_back() {
    let (_dir, db) = setup();
    let before = open_minimal(&db, 2026);
    import(&db, &with_gwg_limit(2026, "2026 v7", 1200.0), 99).unwrap();

    db.undo().unwrap().unwrap();

    assert_eq!(stored(&db, 2026), before);
}
