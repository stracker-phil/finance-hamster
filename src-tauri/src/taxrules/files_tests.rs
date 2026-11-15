use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::check;
use super::files::*;
use crate::error::AppError;
use crate::test_support::rules::{minimal_rules, minimal_text, rules_text, set};

const SEED: &str = include_str!("../../resources/rules/rules-2026.json");
const SEED_2025: &str = include_str!("../../resources/rules/rules-2025.json");

#[test]
fn the_built_in_rules_are_the_checked_files_of_the_resources_folder() {
    assert_eq!(bundled(2025), Some(SEED_2025));
    assert_eq!(bundled(2026), Some(SEED));
}

#[test]
fn a_year_without_built_in_rules_has_none() {
    for year in [2024, 2027, 0] {
        assert_eq!(bundled(year), None, "{year}");
    }
}

#[test]
fn the_built_in_years_come_oldest_first() {
    assert_eq!(bundled_years().collect::<Vec<_>>(), vec![2025, 2026]);
}

#[test]
fn every_built_in_text_passes_the_checks_and_names_its_own_year() {
    for year in bundled_years() {
        let ruleset = check::read(bundled(year).unwrap(), &BTreeSet::new()).unwrap();
        assert_eq!(ruleset.year, year, "{year}");
    }
}

#[test]
fn next_year_of_the_seed_moves_the_year_the_ruleset_and_the_kilometergeld_dates() {
    let text = next_year_text(SEED).unwrap();
    let next: Value = serde_json::from_str(&text).unwrap();

    let mut expected: Value = serde_json::from_str(SEED).unwrap();
    set(&mut expected, "/year", json!(2027));
    set(&mut expected, "/ruleset", json!("2027 v1"));
    set(&mut expected, "/basedOn", json!("2026 v4"));
    set(
        &mut expected,
        "/rates/kilometergeld/0/from",
        json!("2027-01-01"),
    );
    set(
        &mut expected,
        "/rates/kilometergeld/0/to",
        json!("2027-12-31"),
    );
    assert_eq!(next, expected);
    assert!(text.ends_with("}\n"));
}

#[test]
fn next_year_places_based_on_right_after_the_year() {
    let next: Value = serde_json::from_str(&next_year_text(SEED).unwrap()).unwrap();
    let keys: Vec<&str> = next
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let year = keys.iter().position(|k| *k == "year").unwrap();
    assert_eq!(keys[year + 1], "basedOn");
    assert_eq!(keys.iter().filter(|k| **k == "basedOn").count(), 1);
}

#[test]
fn next_year_adds_based_on_when_the_old_file_had_none() {
    let next: Value = serde_json::from_str(&next_year_text(&minimal_text(2026)).unwrap()).unwrap();
    assert_eq!(next["basedOn"], json!("2026 v1"));
    assert_eq!(next["ruleset"], json!("2027 v1"));
    assert_eq!(next["year"], json!(2027));
}

#[test]
fn next_year_of_the_seed_passes_the_checks() {
    let text = next_year_text(SEED).unwrap();
    let ruleset = check::read(&text, &BTreeSet::new()).unwrap();
    assert_eq!(ruleset.year, 2027);
    assert_eq!(ruleset.based_on.as_deref(), Some("2026 v4"));
    assert_eq!(ruleset.items.len(), 12);
}

#[test]
fn next_year_moves_a_leap_day_to_the_last_day_of_february() {
    let mut rules = minimal_rules(2024);
    set(
        &mut rules,
        "/rates",
        json!({ "kilometergeld": [
            { "vehicle": "car", "perKm": 0.42, "from": "2024-02-29", "to": "2024-12-31" }
        ]}),
    );

    let next: Value = serde_json::from_str(&next_year_text(&rules_text(&rules)).unwrap()).unwrap();

    assert_eq!(
        next["rates"]["kilometergeld"][0]["from"],
        json!("2025-02-28")
    );
    assert_eq!(next["rates"]["kilometergeld"][0]["to"], json!("2025-12-31"));
}

#[test]
fn next_year_leaves_other_values_as_they_are() {
    let mut rules = minimal_rules(2026);
    set(&mut rules, "/settings/gwgLimit", json!(1200.00));
    set(&mut rules, "/notes", json!("Keep"));

    let next: Value = serde_json::from_str(&next_year_text(&rules_text(&rules)).unwrap()).unwrap();

    assert_eq!(next["settings"], rules["settings"]);
    assert_eq!(next["notes"], json!("Keep"));
    assert_eq!(next["items"], rules["items"]);
    assert_eq!(next["sections"], rules["sections"]);
}

#[test]
fn next_year_refuses_text_that_is_not_a_rules_object() {
    for text in ["[]", "{}", r#"{"year":"2026"}"#] {
        let err = next_year_text(text).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)), "{text}: {err:?}");
    }
}
