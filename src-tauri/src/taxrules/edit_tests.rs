use serde_json::{Value, json};

use super::edit::{self, JsonPreview};
use super::tests::{closed, insert, setup, stored, use_item, year_row};
use crate::db::Written;
use crate::error::AppError;
use crate::settings::NumberFormat;
use crate::taxyears::{RulesSource, TaxYearRow};
use crate::test_support::rules::{minimal_rules, minimal_text, rich_rules, rules_text, set};

const SCHEMA_PROBLEM: &str = "schemaVersion: this Hamster reads version 1, not 2";

fn written(label: &str) -> Written {
    Written {
        label: label.into(),
        tables: vec!["tax_years".into()],
    }
}

fn rules_of(row: &TaxYearRow) -> Value {
    serde_json::from_str(&row.rules).unwrap()
}

fn own_rich(db: &crate::db::Db) -> TaxYearRow {
    insert(db, year_row(2026, &rich_rules(2026), RulesSource::Own))
}

fn builtin_minimal(db: &crate::db::Db) -> TaxYearRow {
    insert(
        db,
        TaxYearRow::new(2026, "2026 v1".into(), minimal_text(2026), 1),
    )
}

fn set_value(db: &crate::db::Db, path: &str, input: &str) -> crate::error::AppResult<Written> {
    edit::set_value(db, 2026, path, input, NumberFormat::DotDecimal, 99)
}

fn invalid_value(input: &str) -> AppError {
    AppError::InvalidInput(format!("\"{input}\" isn't a valid value"))
}

#[test]
fn the_editor_shows_the_rules_as_pretty_json_ending_in_a_newline() {
    let (_dir, db) = setup();
    insert(
        &db,
        TaxYearRow::new(2026, "2026 v1".into(), minimal_rules(2026).to_string(), 1),
    );

    assert_eq!(edit::json(&db, 2026).unwrap(), minimal_text(2026));
}

#[test]
fn the_editor_for_an_unknown_year_is_not_found() {
    let (_dir, db) = setup();

    assert_eq!(
        edit::json(&db, 2030).unwrap_err(),
        AppError::NotFound("Tax year 2030".into())
    );
}

#[test]
fn each_kind_of_value_changes_in_place_and_names_itself_in_the_op_label() {
    for (path, input, pointer, value, label) in [
        (
            "rates/kilometergeld/0/perKm",
            "0.50",
            "/rates/kilometergeld/0/perKm",
            json!(0.5),
            "Change Car, 1 Jan – 31 Dec for 2026 to €0.50",
        ),
        (
            "settings/gwgLimit",
            "1200",
            "/settings/gwgLimit",
            json!(1200.0),
            "Change Purchases above this are spread over their useful life for 2026 to €1,200.00",
        ),
        (
            "usefulLife/computer",
            "5",
            "/usefulLife/computer",
            json!(5),
            "Change Computers for 2026 to 5 years",
        ),
        (
            "items/books/privateShare",
            "40",
            "/items/0/privateShare",
            json!(40),
            "Change Private share, preset for new documents for 2026 to 40 %",
        ),
        (
            "items/books/estimate/0/amount",
            "250",
            "/items/0/estimate/0/amount",
            json!(250.0),
            "Change Counted at most for 2026 to €250.00",
        ),
        (
            "items/books/estimate/1/min",
            "7",
            "/items/0/estimate/1/min",
            json!(7),
            "Change Needed at least for 2026 to 7 days",
        ),
        (
            "items/books/estimate/2/perUnit",
            "4.50",
            "/items/0/estimate/2/perUnit",
            json!(4.5),
            "Change The tax office subtracts per days for 2026 to €4.50",
        ),
        (
            "items/books/estimate/2/maxUnits",
            "90",
            "/items/0/estimate/2/maxUnits",
            json!(90),
            "Change Counted at most for 2026 to 90 days",
        ),
    ] {
        let (_dir, db) = setup();
        let before = own_rich(&db);

        let result = set_value(&db, path, input).unwrap();

        let mut expected = rich_rules(2026);
        *expected.pointer_mut(pointer).unwrap() = value;
        assert_eq!(result, written(label), "{path}");
        let after = stored(&db, 2026);
        assert_eq!(rules_of(&after), expected, "{path}");
        assert_eq!(after.rules_source, RulesSource::Own, "{path}");
        assert_eq!(after.rules_loaded_at, 99, "{path}");
        assert_eq!(after.ruleset, before.ruleset, "{path}");
    }
}

#[test]
fn a_first_edit_makes_a_built_in_year_the_owners_version_named_after_the_old_ruleset() {
    let (_dir, db) = setup();
    insert(&db, year_row(2026, &rich_rules(2026), RulesSource::Builtin));

    set_value(&db, "settings/gwgLimit", "1200").unwrap();

    let after = stored(&db, 2026);
    let mut expected = rich_rules(2026);
    set(&mut expected, "/ruleset", json!("2026 v1 (own)"));
    set(&mut expected, "/basedOn", json!("2026 v1"));
    set(&mut expected, "/settings/gwgLimit", json!(1200.0));
    assert_eq!(after.rules_source, RulesSource::Own);
    assert_eq!(after.ruleset, "2026 v1 (own)");
    assert_eq!(rules_of(&after), expected);
}

#[test]
fn a_value_edit_can_be_undone() {
    let (_dir, db) = setup();
    let before = own_rich(&db);
    set_value(&db, "settings/gwgLimit", "1200").unwrap();

    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.tables, vec!["tax_years".to_string()]);
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn a_value_that_is_not_a_fitting_number_is_refused_and_changes_nothing() {
    for (path, input) in [
        ("settings/gwgLimit", "abc"),
        ("settings/gwgLimit", "-5"),
        ("settings/gwgLimit", ""),
        ("usefulLife/computer", "3.5"),
        ("usefulLife/computer", "-1"),
        ("items/books/privateShare", "101"),
        ("items/books/privateShare", "-1"),
        ("items/books/privateShare", "half"),
    ] {
        let (_dir, db) = setup();
        let before = own_rich(&db);

        let err = set_value(&db, path, input).unwrap_err();

        assert_eq!(err, invalid_value(input), "{path} {input}");
        assert_eq!(stored(&db, 2026), before, "{path} {input}");
    }
}

#[test]
fn a_private_share_of_zero_and_of_a_hundred_percent_are_accepted() {
    for (input, expected) in [("0", 0), ("100", 100)] {
        let (_dir, db) = setup();
        own_rich(&db);

        set_value(&db, "items/books/privateShare", input).unwrap();

        assert_eq!(
            rules_of(&stored(&db, 2026))["items"][0]["privateShare"],
            json!(expected),
            "{input}"
        );
    }
}

#[test]
fn a_path_the_view_does_not_offer_is_refused() {
    for path in [
        "settings/nope",
        "ruleset",
        "items/ghost/privateShare",
        "items/notes/privateShare",
        "items/plain/privateShare",
        "rates/kilometergeld/3/perKm",
    ] {
        let (_dir, db) = setup();
        let before = own_rich(&db);

        let err = set_value(&db, path, "1").unwrap_err();

        assert_eq!(
            err,
            AppError::InvalidInput(format!("\"{path}\" can't be edited here")),
            "{path}"
        );
        assert_eq!(stored(&db, 2026), before, "{path}");
    }
}

#[test]
fn a_closed_years_value_cannot_be_changed() {
    let (_dir, db) = setup();
    let frozen = insert(
        &db,
        closed(year_row(2026, &rich_rules(2026), RulesSource::Builtin)),
    );

    let err = set_value(&db, "settings/gwgLimit", "1200").unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput("Tax year 2026 is closed; its rules are frozen".into())
    );
    assert_eq!(stored(&db, 2026), frozen);
}

#[test]
fn changing_a_value_of_an_unknown_year_is_not_found() {
    let (_dir, db) = setup();

    let err = edit::set_value(
        &db,
        2030,
        "settings/gwgLimit",
        "1",
        NumberFormat::DotDecimal,
        99,
    )
    .unwrap_err();

    assert_eq!(err, AppError::NotFound("Tax year 2030".into()));
}

#[test]
fn an_amount_typed_with_a_decimal_comma_is_read_and_shown_in_the_chosen_format() {
    let (_dir, db) = setup();
    own_rich(&db);

    let result = edit::set_value(
        &db,
        2026,
        "settings/gwgLimit",
        "1.234,50",
        NumberFormat::CommaDecimal,
        99,
    )
    .unwrap();

    assert_eq!(
        result,
        written(
            "Change Purchases above this are spread over their useful life for 2026 to €1.234,50"
        )
    );
    assert_eq!(
        rules_of(&stored(&db, 2026))["settings"]["gwgLimit"],
        json!(1234.5)
    );
}

#[test]
fn a_preview_lists_what_saving_would_change_without_saving() {
    let (_dir, db) = setup();
    let before = builtin_minimal(&db);
    let mut next = minimal_rules(2026);
    set(&mut next, "/settings/gwgLimit", json!(1200.00));

    let preview = edit::preview_json(&db, 2026, &rules_text(&next)).unwrap();

    assert_eq!(
        preview,
        JsonPreview {
            problems: vec![],
            changes: vec!["settings.gwgLimit: 1000 → 1200".into()],
        }
    );
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn a_preview_of_the_same_rules_has_no_changes() {
    let (_dir, db) = setup();
    builtin_minimal(&db);

    assert_eq!(
        edit::preview_json(&db, 2026, &minimal_text(2026)).unwrap(),
        JsonPreview {
            problems: vec![],
            changes: vec![],
        }
    );
}

#[test]
fn a_preview_of_rules_that_fail_the_checks_lists_the_problems_and_no_changes() {
    let (_dir, db) = setup();
    builtin_minimal(&db);
    let mut broken = minimal_rules(2026);
    set(&mut broken, "/schemaVersion", json!(2));

    assert_eq!(
        edit::preview_json(&db, 2026, &rules_text(&broken)).unwrap(),
        JsonPreview {
            problems: vec![SCHEMA_PROBLEM.into()],
            changes: vec![],
        }
    );
}

#[test]
fn a_preview_of_text_that_is_not_json_has_one_problem() {
    let (_dir, db) = setup();
    builtin_minimal(&db);

    let preview = edit::preview_json(&db, 2026, "{").unwrap();

    assert_eq!(preview.problems.len(), 1);
    assert_eq!(preview.changes, Vec::<String>::new());
}

#[test]
fn a_preview_of_rules_for_another_year_is_a_problem_beside_the_changes() {
    let (_dir, db) = setup();
    builtin_minimal(&db);

    assert_eq!(
        edit::preview_json(&db, 2026, &minimal_text(2027)).unwrap(),
        JsonPreview {
            problems: vec!["year: the rules say 2027, not 2026".into()],
            changes: vec!["ruleset: \"2026 v1\" → \"2027 v1\"".into()],
        }
    );
}

#[test]
fn a_preview_of_rules_that_drop_a_used_item_names_it() {
    let (_dir, db) = setup();
    builtin_minimal(&db);
    use_item(&db, "books");
    let mut next = minimal_rules(2026);
    set(&mut next, "/items/0/key", json!("fees"));

    assert_eq!(
        edit::preview_json(&db, 2026, &rules_text(&next))
            .unwrap()
            .problems,
        vec![
            "items: \"books\" is missing, but entries use it; keys can't be removed or renamed"
                .to_string()
        ]
    );
}

#[test]
fn a_preview_for_an_unknown_year_is_not_found() {
    let (_dir, db) = setup();

    assert_eq!(
        edit::preview_json(&db, 2030, &minimal_text(2030)).unwrap_err(),
        AppError::NotFound("Tax year 2030".into())
    );
}

#[test]
fn saving_json_with_a_new_ruleset_name_makes_the_year_the_owners_version_under_that_name() {
    let (_dir, db) = setup();
    let before = builtin_minimal(&db);
    let mut next = minimal_rules(2026);
    set(&mut next, "/ruleset", json!("2026 v2"));
    set(&mut next, "/settings/gwgLimit", json!(1200.00));

    let result = edit::set_json(&db, 2026, &rules_text(&next), 99).unwrap();

    assert_eq!(result, written("Edit the Tax Rules for 2026"));
    let after = stored(&db, 2026);
    assert_eq!(after.rules_source, RulesSource::Own);
    assert_eq!(after.ruleset, "2026 v2");
    assert_eq!(after.rules_loaded_at, 99);
    assert_eq!(rules_of(&after), next);

    db.undo().unwrap().unwrap();
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn saving_json_under_the_same_ruleset_name_names_the_owners_version_after_it() {
    let (_dir, db) = setup();
    builtin_minimal(&db);
    let mut next = minimal_rules(2026);
    set(&mut next, "/settings/gwgLimit", json!(1200.00));

    edit::set_json(&db, 2026, &rules_text(&next), 99).unwrap();

    let after = stored(&db, 2026);
    set(&mut next, "/ruleset", json!("2026 v1 (own)"));
    set(&mut next, "/basedOn", json!("2026 v1"));
    assert_eq!(after.rules_source, RulesSource::Own);
    assert_eq!(after.ruleset, "2026 v1 (own)");
    assert_eq!(rules_of(&after), next);
}

#[test]
fn saving_json_in_an_own_year_keeps_its_name_and_origin() {
    let (_dir, db) = setup();
    let mut own = minimal_rules(2026);
    set(&mut own, "/ruleset", json!("2026 v5"));
    set(&mut own, "/basedOn", json!("2026 v4"));
    insert(&db, year_row(2026, &own, RulesSource::Own));
    let mut next = own.clone();
    set(&mut next, "/settings/gwgLimit", json!(1200.00));

    edit::set_json(&db, 2026, &rules_text(&next), 99).unwrap();

    let after = stored(&db, 2026);
    assert_eq!(after.ruleset, "2026 v5");
    assert_eq!(rules_of(&after), next);
}

#[test]
fn saving_rules_with_problems_is_refused_and_changes_nothing() {
    let (_dir, db) = setup();
    let before = builtin_minimal(&db);
    let mut broken = minimal_rules(2026);
    set(&mut broken, "/schemaVersion", json!(2));

    for (text, problem) in [
        (
            rules_text(&broken),
            format!("These rules can't be used: {SCHEMA_PROBLEM}"),
        ),
        (
            minimal_text(2027),
            "These rules can't be used: year: the rules say 2027, not 2026".to_string(),
        ),
    ] {
        let err = edit::set_json(&db, 2026, &text, 99).unwrap_err();

        assert_eq!(err, AppError::InvalidInput(problem));
    }
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn saving_text_that_is_not_json_is_refused() {
    let (_dir, db) = setup();
    let before = builtin_minimal(&db);

    let err = edit::set_json(&db, 2026, "{", 99).unwrap_err();

    assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
    assert_eq!(stored(&db, 2026), before);
}

#[test]
fn saving_json_for_a_closed_or_unknown_year_is_refused() {
    let (_dir, db) = setup();
    let frozen = insert(
        &db,
        closed(year_row(2026, &minimal_rules(2026), RulesSource::Builtin)),
    );

    assert_eq!(
        edit::set_json(&db, 2026, &minimal_text(2026), 99).unwrap_err(),
        AppError::InvalidInput("Tax year 2026 is closed; its rules are frozen".into())
    );
    assert_eq!(stored(&db, 2026), frozen);
    assert_eq!(
        edit::set_json(&db, 2030, &minimal_text(2030), 99).unwrap_err(),
        AppError::NotFound("Tax year 2030".into())
    );
}
