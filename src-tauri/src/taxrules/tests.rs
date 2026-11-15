use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::*;
use crate::documents::store::{DOCUMENTS, DocState, DocumentRow};
use crate::test_support::rules::{minimal_rules, rich_rules, rules_text, set};
use crate::test_support::temp_config_dir;

pub(super) fn setup() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

pub(super) fn insert(db: &Db, row: TaxYearRow) -> TaxYearRow {
    let change = Change::insert(taxyears::TABLE, &row).unwrap();
    db.write(Op::new("Add year", vec![change]), Source::App)
        .unwrap();
    row
}

pub(super) fn stored(db: &Db, year: i32) -> TaxYearRow {
    db.read(|c| taxyears::row(c, year)).unwrap().unwrap()
}

/// An open year with `rules`, named after the rules' own ruleset.
pub(super) fn year_row(year: i32, rules: &Value, source: RulesSource) -> TaxYearRow {
    TaxYearRow {
        rules_source: source,
        ..TaxYearRow::new(
            year,
            rules["ruleset"].as_str().unwrap().to_string(),
            rules_text(rules),
            1,
        )
    }
}

pub(super) fn closed(row: TaxYearRow) -> TaxYearRow {
    TaxYearRow {
        state: YearState::Closed,
        closed_at: Some(500),
        ..row
    }
}

/// The year as the app starts it: the built-in rules, untouched.
pub(super) fn builtin_row(year: i32, now: i64) -> TaxYearRow {
    let text = files::bundled(year).unwrap();
    let ruleset = check::read(text, &BTreeSet::new()).unwrap().ruleset;
    TaxYearRow::new(year, ruleset, text.to_string(), now)
}

/// A draft document filed under `key`, so no ruleset may drop that item.
pub(super) fn use_item(db: &Db, key: &str) {
    let row = DocumentRow {
        id: format!("doc-{key}"),
        state: DocState::Draft,
        file: format!("docs/inbox/doc-{key}.pdf"),
        original_name: format!("doc-{key}.pdf"),
        file_hash: crate::statements::file_hash(key.as_bytes()),
        vendor_id: None,
        pay_date: Some("2026-03-03".into()),
        paid_cents: None,
        item_key: Some(key.into()),
        work_share: 100,
        note: String::new(),
        invoice_cents: None,
        invoice_currency: None,
        invoice_number: None,
        invoice_date: None,
        created_at: 1,
        finished_at: None,
        excluded_at: None,
        exclude_reason: None,
        useful_life: None,
    };
    db.write(
        Op::new(
            "Add document",
            vec![Change::insert(DOCUMENTS, &row).unwrap()],
        ),
        Source::Seed,
    )
    .unwrap();
}

fn tax_years_written(label: &str) -> Written {
    Written {
        label: label.into(),
        tables: vec!["tax_years".into()],
    }
}

fn own_rules(year: i32, ruleset: &str, based_on: &str) -> Value {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/ruleset", json!(ruleset));
    set(&mut rules, "/basedOn", json!(based_on));
    rules
}

fn year_summary(years: &[RulesYear]) -> Vec<(i32, YearState, RulesSource)> {
    years.iter().map(|y| (y.year, y.state, y.source)).collect()
}

#[test]
fn used_keys_collects_the_item_keys_recorded_data_refers_to() {
    let (_dir, db) = setup();
    assert_eq!(db.read(used_keys).unwrap(), BTreeSet::new());

    use_item(&db, "books");
    use_item(&db, "travel");

    assert_eq!(
        db.read(used_keys).unwrap(),
        BTreeSet::from(["books".to_string(), "travel".to_string()])
    );
}

#[test]
fn without_tax_years_the_overview_offers_the_newest_built_in_year() {
    let (_dir, db) = setup();

    assert_eq!(
        overview(&db).unwrap(),
        RulesOverview {
            years: vec![],
            next_year: Some(2026),
            next_is_builtin: true,
        }
    );
}

#[test]
fn the_overview_lists_years_newest_first_with_source_origin_and_built_in_availability() {
    let (_dir, db) = setup();
    insert(
        &db,
        closed(year_row(2024, &minimal_rules(2024), RulesSource::Builtin)),
    );
    insert(&db, builtin_row(2025, 1));
    let mut own = rich_rules(2026);
    set(&mut own, "/ruleset", json!("2026 v5"));
    set(&mut own, "/basedOn", json!("2026 v4"));
    insert(&db, year_row(2026, &own, RulesSource::Own));

    assert_eq!(
        overview(&db).unwrap(),
        RulesOverview {
            years: vec![
                RulesYear {
                    year: 2026,
                    state: YearState::Open,
                    source: RulesSource::Own,
                    ruleset: "2026 v5".into(),
                    based_on: Some("2026 v4".into()),
                    builtin_newer: None,
                    problems: vec![],
                    has_builtin: true,
                    unverified: 0,
                },
                RulesYear {
                    year: 2025,
                    state: YearState::Open,
                    source: RulesSource::Builtin,
                    ruleset: "2025 v3".into(),
                    based_on: None,
                    builtin_newer: None,
                    problems: vec![],
                    has_builtin: true,
                    unverified: 0,
                },
                RulesYear {
                    year: 2024,
                    state: YearState::Closed,
                    source: RulesSource::Builtin,
                    ruleset: "2024 v1".into(),
                    based_on: None,
                    builtin_newer: None,
                    problems: vec![],
                    has_builtin: false,
                    unverified: 0,
                },
            ],
            next_year: Some(2027),
            next_is_builtin: false,
        }
    );
}

#[test]
fn the_next_year_is_built_in_while_the_newest_year_is_the_one_before() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));

    let overview = overview(&db).unwrap();

    assert_eq!(overview.next_year, Some(2026));
    assert!(overview.next_is_builtin);
}

#[test]
fn an_own_year_shows_the_built_in_version_only_when_it_started_from_an_older_one() {
    for (based_on, expected) in [("2026 v3", Some("2026 v4")), ("2026 v4", None)] {
        let (_dir, db) = setup();
        insert(
            &db,
            year_row(
                2026,
                &own_rules(2026, "2026 v5", based_on),
                RulesSource::Own,
            ),
        );

        let year = overview(&db).unwrap().years.remove(0);

        assert_eq!(year.builtin_newer.as_deref(), expected, "{based_on}");
        assert_eq!(year.problems, Vec::<String>::new(), "{based_on}");
    }
}

#[test]
fn an_open_built_in_year_with_an_older_version_has_nothing_to_report_as_it_will_move_on() {
    let (_dir, db) = setup();
    insert(
        &db,
        year_row(2026, &minimal_rules(2026), RulesSource::Builtin),
    );

    let year = overview(&db).unwrap().years.remove(0);

    assert_eq!(year.builtin_newer, None);
    assert_eq!(year.problems, Vec::<String>::new());
}

#[test]
fn a_built_in_version_that_drops_a_used_item_is_reported_with_its_problems() {
    let (_dir, db) = setup();
    insert(
        &db,
        year_row(2026, &minimal_rules(2026), RulesSource::Builtin),
    );
    use_item(&db, "books");

    let year = overview(&db).unwrap().years.remove(0);

    assert_eq!(year.builtin_newer.as_deref(), Some("2026 v4"));
    assert_eq!(
        year.problems,
        vec![
            "items: \"books\" is missing, but entries use it; keys can't be removed or renamed"
                .to_string()
        ]
    );
    assert_eq!(year.ruleset, "2026 v1");
}

#[test]
fn a_closed_year_reports_no_newer_built_in_version() {
    let (_dir, db) = setup();
    use_item(&db, "books");
    insert(
        &db,
        closed(year_row(2026, &minimal_rules(2026), RulesSource::Builtin)),
    );

    let year = overview(&db).unwrap().years.remove(0);

    assert_eq!(year.builtin_newer, None);
    assert_eq!(year.problems, Vec::<String>::new());
}

#[test]
fn unverified_values_of_a_year_are_counted() {
    let (_dir, db) = setup();
    let mut rules = minimal_rules(2026);
    set(&mut rules, "/items/0/verified", json!(false));
    insert(&db, year_row(2026, &rules, RulesSource::Own));

    assert_eq!(overview(&db).unwrap().years[0].unverified, 1);
}

#[test]
fn the_overview_serializes_with_camel_case_names_and_snake_case_words() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2026, 1));

    assert_eq!(
        serde_json::to_value(overview(&db).unwrap()).unwrap(),
        json!({
            "years": [{
                "year": 2026,
                "state": "open",
                "source": "builtin",
                "ruleset": "2026 v4",
                "basedOn": "2025 v3",
                "builtinNewer": null,
                "problems": [],
                "hasBuiltin": true,
                "unverified": 0
            }],
            "nextYear": 2027,
            "nextIsBuiltin": false
        })
    );
}

#[test]
fn an_open_built_in_year_takes_on_the_current_built_in_version_as_one_op() {
    let (_dir, db) = setup();
    let before = insert(
        &db,
        year_row(2026, &minimal_rules(2026), RulesSource::Builtin),
    );

    let written = adopt_builtin(&db, 99).unwrap().unwrap();

    assert_eq!(written, tax_years_written("Update Tax Rules to 2026 v4"));
    assert_eq!(
        stored(&db, 2026),
        TaxYearRow {
            ruleset: "2026 v4".into(),
            rules: files::bundled(2026).unwrap().to_string(),
            rules_loaded_at: 99,
            rules_source: RulesSource::Builtin,
            ..before
        }
    );
}

#[test]
fn an_adopted_version_is_journaled_but_not_on_the_undo_stack() {
    let (_dir, db) = setup();
    insert(
        &db,
        year_row(2026, &minimal_rules(2026), RulesSource::Builtin),
    );
    let journal = || -> i64 {
        db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap()
    };
    let before = journal();
    adopt_builtin(&db, 99).unwrap().unwrap();

    assert!(journal() > before);
    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(stored(&db, 2026).ruleset, "2026 v4");
}

#[test]
fn every_outdated_built_in_year_moves_in_one_op_named_oldest_first() {
    let (_dir, db) = setup();
    for year in [2025, 2026] {
        insert(
            &db,
            year_row(year, &minimal_rules(year), RulesSource::Builtin),
        );
    }

    let written = adopt_builtin(&db, 99).unwrap().unwrap();

    assert_eq!(
        written,
        tax_years_written("Update Tax Rules to 2025 v3, 2026 v4")
    );
    assert_eq!(stored(&db, 2025).rules, files::bundled(2025).unwrap());
    assert_eq!(stored(&db, 2026).rules, files::bundled(2026).unwrap());
}

#[test]
fn a_year_that_already_has_the_built_in_text_is_left_alone() {
    let (_dir, db) = setup();
    let current = insert(&db, builtin_row(2026, 1));

    assert_eq!(adopt_builtin(&db, 99).unwrap(), None);
    assert_eq!(stored(&db, 2026), current);
}

#[test]
fn without_tax_years_there_is_nothing_to_adopt() {
    let (_dir, db) = setup();
    assert_eq!(adopt_builtin(&db, 99).unwrap(), None);
}

#[test]
fn an_own_year_and_a_closed_year_keep_their_rules() {
    let (_dir, db) = setup();
    let own = insert(&db, year_row(2026, &minimal_rules(2026), RulesSource::Own));
    let frozen = insert(
        &db,
        closed(year_row(2025, &minimal_rules(2025), RulesSource::Builtin)),
    );

    assert_eq!(adopt_builtin(&db, 99).unwrap(), None);
    assert_eq!(stored(&db, 2026), own);
    assert_eq!(stored(&db, 2025), frozen);
}

#[test]
fn a_year_without_built_in_rules_is_left_alone() {
    let (_dir, db) = setup();
    let row = insert(
        &db,
        year_row(2030, &minimal_rules(2030), RulesSource::Builtin),
    );

    assert_eq!(adopt_builtin(&db, 99).unwrap(), None);
    assert_eq!(stored(&db, 2030), row);
}

#[test]
fn a_built_in_version_that_drops_an_item_recorded_data_uses_is_not_taken_on() {
    let (_dir, db) = setup();
    let before = insert(
        &db,
        year_row(2026, &minimal_rules(2026), RulesSource::Builtin),
    );
    use_item(&db, "books");

    assert_eq!(adopt_builtin(&db, 99).unwrap(), None);
    assert_eq!(stored(&db, 2026), before);
    assert_eq!(db.undo().unwrap(), None);
}

#[test]
fn set_own_makes_the_text_the_owners_version_after_the_checks() {
    let (_dir, db) = setup();
    let before = insert(&db, builtin_row(2026, 1));
    let mut rules = minimal_rules(2026);
    set(&mut rules, "/ruleset", json!("2026 v9"));

    let written = set_own(&db, 2026, rules_text(&rules), "Edit rules".into(), 99).unwrap();

    assert_eq!(written, tax_years_written("Edit rules"));
    assert_eq!(
        stored(&db, 2026),
        TaxYearRow {
            ruleset: "2026 v9".into(),
            rules: rules_text(&rules),
            rules_loaded_at: 99,
            rules_source: RulesSource::Own,
            ..before
        }
    );
}

#[test]
fn set_own_refuses_rules_that_fail_the_checks_or_name_another_year() {
    let (_dir, db) = setup();
    let before = insert(&db, builtin_row(2026, 1));
    let mut broken = minimal_rules(2026);
    set(&mut broken, "/schemaVersion", json!(2));

    for (text, problem) in [
        (
            rules_text(&broken),
            "schemaVersion: this Hamster reads version 1, not 2",
        ),
        (
            rules_text(&minimal_rules(2027)),
            "year: the rules say 2027, not 2026",
        ),
    ] {
        let err = set_own(&db, 2026, text, "Edit".into(), 99).unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidInput(format!("These rules can't be used: {problem}"))
        );
    }
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn set_own_refuses_rules_that_drop_a_used_item() {
    let (_dir, db) = setup();
    insert(&db, year_row(2026, &minimal_rules(2026), RulesSource::Own));
    use_item(&db, "books");
    let mut rules = minimal_rules(2026);
    set(&mut rules, "/items/0/key", json!("fees"));

    let err = set_own(&db, 2026, rules_text(&rules), "Edit".into(), 99).unwrap_err();

    assert!(
        matches!(&err, AppError::InvalidInput(m) if m.contains("\"books\" is missing")),
        "{err:?}"
    );
}

#[test]
fn use_builtin_returns_an_own_year_to_the_built_in_rules() {
    let (_dir, db) = setup();
    let before = insert(
        &db,
        year_row(
            2026,
            &own_rules(2026, "2026 v5", "2026 v3"),
            RulesSource::Own,
        ),
    );

    let written = use_builtin(&db, 2026, 99).unwrap();

    assert_eq!(
        written,
        tax_years_written("Use the Built-in Tax Rules for 2026")
    );
    assert_eq!(
        stored(&db, 2026),
        TaxYearRow {
            ruleset: "2026 v4".into(),
            rules: files::bundled(2026).unwrap().to_string(),
            rules_loaded_at: 99,
            rules_source: RulesSource::Builtin,
            ..before.clone()
        }
    );

    db.undo().unwrap().unwrap();
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn use_builtin_is_refused_for_a_year_without_built_in_rules() {
    let (_dir, db) = setup();
    let before = insert(&db, year_row(2027, &minimal_rules(2027), RulesSource::Own));

    let err = use_builtin(&db, 2027, 99).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput("Hamster has no built-in rules for 2027".into())
    );
    assert_eq!(stored(&db, 2027), before);
}

#[test]
fn use_builtin_is_refused_for_a_closed_or_unknown_year() {
    let (_dir, db) = setup();
    insert(
        &db,
        closed(year_row(2026, &minimal_rules(2026), RulesSource::Own)),
    );

    assert_eq!(
        use_builtin(&db, 2026, 99).unwrap_err(),
        AppError::InvalidInput("Tax year 2026 is closed; its rules are frozen".into())
    );
    assert_eq!(
        use_builtin(&db, 2030, 99).unwrap_err(),
        AppError::NotFound("Tax year 2030".into())
    );
}

#[test]
fn use_builtin_is_refused_when_recorded_data_uses_an_item_the_built_in_rules_lack() {
    let (_dir, db) = setup();
    let before = insert(&db, year_row(2026, &minimal_rules(2026), RulesSource::Own));
    use_item(&db, "books");

    let err = use_builtin(&db, 2026, 99).unwrap_err();

    assert!(
        matches!(&err, AppError::InvalidInput(m) if m.starts_with("These rules can't be used: ")),
        "{err:?}"
    );
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn start_next_takes_the_built_in_rules_of_the_next_year_when_the_app_has_them() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));

    let written = start_next(&db, 99).unwrap();

    assert_eq!(written, tax_years_written("Start Tax Year 2026"));
    assert_eq!(stored(&db, 2026), builtin_row(2026, 99));
}

#[test]
fn start_next_copies_the_newest_year_as_the_owners_version_without_built_in_rules() {
    let (_dir, db) = setup();
    let newest = insert(&db, builtin_row(2026, 1));

    let written = start_next(&db, 99).unwrap();

    assert_eq!(written, tax_years_written("Start Tax Year 2027"));
    let text = files::next_year_text(&newest.rules).unwrap();
    assert_eq!(
        stored(&db, 2027),
        TaxYearRow {
            rules_source: RulesSource::Own,
            ..TaxYearRow::new(2027, "2027 v1".into(), text, 99)
        }
    );
    let copy = check::read(&stored(&db, 2027).rules, &BTreeSet::new()).unwrap();
    assert_eq!(copy.based_on.as_deref(), Some("2026 v4"));
}

#[test]
fn start_next_continues_from_the_newest_year_only() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));
    insert(&db, builtin_row(2026, 1));

    start_next(&db, 99).unwrap();

    assert_eq!(
        year_summary(&overview(&db).unwrap().years),
        vec![
            (2027, YearState::Open, RulesSource::Own),
            (2026, YearState::Open, RulesSource::Builtin),
            (2025, YearState::Open, RulesSource::Builtin),
        ]
    );
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
fn start_next_carries_the_newest_years_hidden_categories_over() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));
    hide_category(&db, 2025, "union-dues");
    hide_category(&db, 2025, "books");
    hide_category(&db, 2024, "other");

    let written = start_next(&db, 99).unwrap();

    let mut tables = written.tables;
    tables.sort();
    assert_eq!(tables, ["hidden_items", "tax_years"]);
    assert_eq!(hidden_in(&db, 2026), ["books", "union-dues"]);
    assert_eq!(hidden_in(&db, 2025), ["books", "union-dues"]);
}

#[test]
fn start_next_hides_nothing_when_the_newest_year_hid_nothing() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));

    let written = start_next(&db, 99).unwrap();

    assert_eq!(written, tax_years_written("Start Tax Year 2026"));
    assert!(hidden_in(&db, 2026).is_empty());
}

#[test]
fn undoing_start_next_takes_the_carried_categories_back_too() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2025, 1));
    hide_category(&db, 2025, "books");
    start_next(&db, 99).unwrap();

    db.undo().unwrap().unwrap();

    assert!(hidden_in(&db, 2026).is_empty());
    assert_eq!(hidden_in(&db, 2025), ["books"]);
}

#[test]
fn start_next_can_be_undone() {
    let (_dir, db) = setup();
    insert(&db, builtin_row(2026, 1));
    start_next(&db, 99).unwrap();

    db.undo().unwrap().unwrap();

    assert_eq!(db.read(|c| taxyears::row(c, 2027)).unwrap(), None);
}

#[test]
fn start_next_without_any_tax_year_is_refused() {
    let (_dir, db) = setup();

    assert_eq!(
        start_next(&db, 99).unwrap_err(),
        AppError::InvalidInput("There is no tax year to continue from".into())
    );
}
