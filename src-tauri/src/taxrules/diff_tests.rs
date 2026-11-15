use serde_json::{Value, json};

use super::diff::describe;
use super::model::Ruleset;
use crate::test_support::rules::{minimal_rules, set};

fn typed(rules: Value) -> Ruleset {
    serde_json::from_value(rules).unwrap()
}

fn lines(old: Value, new: Value) -> Vec<String> {
    describe(&typed(old), &typed(new))
}

/// The lines for `mutate` applied to the minimal file, with the ruleset bumped so no hint is added; the
/// ruleset's own line is checked and left out.
fn changes(mutate: impl FnOnce(&mut Value)) -> Vec<String> {
    let mut new = minimal_rules(2026);
    mutate(&mut new);
    set(&mut new, "/ruleset", json!("2026 v2"));
    let mut all = lines(minimal_rules(2026), new);
    assert_eq!(all.remove(0), "ruleset: \"2026 v1\" → \"2026 v2\"");
    all
}

fn push_item(rules: &mut Value, key: &str, kz: &str, name: &str) {
    rules["items"].as_array_mut().unwrap().push(json!({
        "key": key, "kz": kz, "name": name, "section": "main", "kind": "documents"
    }));
}

#[test]
fn identical_files_describe_nothing() {
    assert_eq!(
        lines(minimal_rules(2026), minimal_rules(2026)),
        Vec::<String>::new()
    );
}

#[test]
fn a_changed_setting_and_a_bumped_ruleset_read_as_two_lines() {
    let mut new = minimal_rules(2026);
    set(&mut new, "/settings/gwgLimit", json!(1200.00));
    set(&mut new, "/ruleset", json!("2026 v2"));

    assert_eq!(
        lines(minimal_rules(2026), new),
        vec![
            "ruleset: \"2026 v1\" → \"2026 v2\"".to_string(),
            "settings.gwgLimit: 1000 → 1200".to_string(),
        ]
    );
}

#[test]
fn a_change_under_the_same_ruleset_name_adds_a_hint_to_bump_it() {
    let mut new = minimal_rules(2026);
    set(&mut new, "/settings/gwgLimit", json!(1200.00));

    assert_eq!(
        lines(minimal_rules(2026), new),
        vec![
            "settings.gwgLimit: 1000 → 1200".to_string(),
            "The ruleset is still \"2026 v1\"; bump it to tell the versions apart".to_string(),
        ]
    );
}

#[test]
fn only_a_renamed_ruleset_is_one_line_without_a_hint() {
    let mut new = minimal_rules(2026);
    set(&mut new, "/ruleset", json!("2026 v2"));
    assert_eq!(
        lines(minimal_rules(2026), new),
        vec!["ruleset: \"2026 v1\" → \"2026 v2\"".to_string()]
    );
}

#[test]
fn notes_sources_and_based_on_are_not_changes() {
    let mut new = minimal_rules(2026);
    set(&mut new, "/notes", json!("Reworded"));
    set(&mut new, "/basedOn", json!("2025 v9"));
    set(
        &mut new,
        "/sources",
        json!([{ "label": "Example", "url": "https://example.com" }]),
    );
    assert_eq!(lines(minimal_rules(2026), new), Vec::<String>::new());
}

#[test]
fn settings_that_appear_or_change_name_their_old_and_new_value() {
    assert_eq!(
        changes(|r| set(r, "/settings/werbungskostenpauschale", json!(132.00))),
        vec!["settings.werbungskostenpauschale: none → 132".to_string()]
    );
    assert_eq!(
        changes(|r| set(r, "/settings/halfYearFrom", json!("01-01"))),
        vec!["settings.halfYearFrom: \"07-01\" → \"01-01\"".to_string()]
    );
    assert_eq!(
        changes(|r| set(r, "/usefulLife", json!({ "computer": 3 }))),
        vec!["usefulLife.computer: none → 3".to_string()]
    );
}

#[test]
fn a_changed_kilometergeld_rate_is_named_by_its_position() {
    let with_rate = |per_km: f64| {
        let mut rules = minimal_rules(2026);
        set(
            &mut rules,
            "/rates",
            json!({ "kilometergeld": [
                { "vehicle": "car", "perKm": per_km, "from": "2026-01-01", "to": "2026-12-31" }
            ]}),
        );
        rules
    };
    assert_eq!(
        lines(with_rate(0.5), with_rate(0.6)),
        vec![
            "rates.kilometergeld[0].perKm: 0.5 → 0.6".to_string(),
            "The ruleset is still \"2026 v1\"; bump it to tell the versions apart".to_string(),
        ]
    );
}

#[test]
fn a_change_to_only_a_german_hint_is_reported() {
    let with_hint = |hint: &str| {
        let mut rules = minimal_rules(2026);
        set(
            &mut rules,
            "/items/0/ask",
            json!({
                "order": 1, "question": "Did you buy books?", "hint": "Receipts",
                "de": { "question": "Hast du Bücher gekauft?", "hint": hint }
            }),
        );
        rules
    };
    let mut new = with_hint("Belege und Rechnungen");
    set(&mut new, "/ruleset", json!("2026 v2"));

    let mut all = lines(with_hint("Belege"), new);

    assert_eq!(all.remove(0), "ruleset: \"2026 v1\" → \"2026 v2\"");
    assert_eq!(
        all,
        vec!["items.books.ask.de.hint: \"Belege\" → \"Belege und Rechnungen\"".to_string()]
    );
}

#[test]
fn an_added_german_name_is_reported_under_the_item() {
    assert_eq!(
        changes(|r| set(r, "/items/0/de", json!({ "name": "Bücher" }))),
        vec!["items.books.de: none → {\"name\":\"Bücher\"}".to_string()]
    );
}

#[test]
fn an_added_item_is_listed_with_its_kz_and_name() {
    assert_eq!(
        changes(|r| push_item(r, "fees", "724", "Fees")),
        vec!["items: \"fees\" (KZ 724 Fees) added".to_string()]
    );
}

#[test]
fn a_removed_item_is_listed_with_its_kz_and_name() {
    assert_eq!(
        changes(|r| r["items"] = json!([])),
        vec!["items: \"books\" (KZ 720 Books) removed".to_string()]
    );
}

#[test]
fn a_replaced_item_lists_the_removal_before_the_addition() {
    assert_eq!(
        changes(|r| {
            r["items"] = json!([]);
            push_item(r, "fees", "724", "Fees");
        }),
        vec![
            "items: \"books\" (KZ 720 Books) removed".to_string(),
            "items: \"fees\" (KZ 724 Fees) added".to_string(),
        ]
    );
}

#[test]
fn a_changed_item_field_is_named_by_the_item_key_and_the_field() {
    assert_eq!(
        changes(|r| set(r, "/items/0/kz", json!("721"))),
        vec!["items.books.kz: \"720\" → \"721\"".to_string()]
    );
    assert_eq!(
        changes(|r| set(r, "/items/0/privateShare", json!(40))),
        vec!["items.books.privateShare: none → 40".to_string()]
    );
    assert_eq!(
        changes(|r| set(r, "/items/0/visible", json!(false))),
        vec!["items.books.visible: true → false".to_string()]
    );
}

#[test]
fn items_are_matched_by_key_not_by_position() {
    let mut old = minimal_rules(2026);
    push_item(&mut old, "fees", "724", "Fees");
    let mut new = minimal_rules(2026);
    push_item(&mut new, "fees", "724", "Fees");
    new["items"].as_array_mut().unwrap().reverse();
    assert_eq!(lines(old, new), Vec::<String>::new());
}

#[test]
fn added_questions_and_sections_are_listed_by_their_label_and_name() {
    assert_eq!(
        changes(|r| set(
            r,
            "/questions",
            json!([{ "id": "days", "label": "Days", "type": "integer" }])
        )),
        vec!["questions: \"days\" (Days) added".to_string()]
    );
    assert_eq!(
        changes(|r| r["sections"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "id": "other", "name": "Other" }))),
        vec!["sections: \"other\" (Other) added".to_string()]
    );
}
