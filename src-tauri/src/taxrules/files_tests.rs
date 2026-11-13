use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::check;
use super::files::*;
use crate::error::AppError;
use crate::store::json_file::create_private_dir;
use crate::test_support::rules::{minimal_rules, minimal_text, rules_text, set};
use crate::test_support::temp_config_dir;

const SCHEMA: &str = include_str!("../../resources/rules/rules.schema.json");
const README: &str = include_str!("../../resources/rules/README.md");
const SEED: &str = include_str!("../../resources/rules/rules-2026.json");

fn with_rules_folder() -> (tempfile::TempDir, PathBuf) {
    let config = temp_config_dir();
    let folder = dir(config.path());
    create_private_dir(&folder).unwrap();
    (config, folder)
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn only_a_plain_year_file_has_a_year() {
    for (name, expected) in [
        ("rules-2026.json", Some(2026)),
        ("rules-1999.json", Some(1999)),
        ("rules-2026.v1.json", None),
        ("rules-2026.v1-2.json", None),
        ("rules-26.json", None),
        ("rules-20260.json", None),
        ("rules-abcd.json", None),
        ("rules-2026.json.tmp", None),
        ("rules.schema.json", None),
        ("README.md", None),
        ("2026.json", None),
    ] {
        assert_eq!(year_of(name), expected, "{name}");
    }
}

#[test]
fn file_names_follow_the_year() {
    assert_eq!(file_name(2027), "rules-2027.json");
    assert_eq!(year_of(&file_name(2027)), Some(2027));
}

#[test]
fn install_writes_the_schema_the_readme_and_the_seed_into_an_empty_folder() {
    let config = temp_config_dir();
    install(config.path()).unwrap();

    let folder = dir(config.path());
    assert_eq!(
        names_in(&folder),
        vec!["README.md", "rules-2026.json", "rules.schema.json"]
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("rules.schema.json")).unwrap(),
        SCHEMA
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("README.md")).unwrap(),
        README
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2026.json")).unwrap(),
        SEED
    );
}

#[test]
fn install_never_seeds_next_to_an_existing_rules_file() {
    let (config, folder) = with_rules_folder();
    std::fs::write(folder.join("rules-2025.json"), "mine").unwrap();

    install(config.path()).unwrap();

    assert_eq!(
        names_in(&folder),
        vec!["README.md", "rules-2025.json", "rules.schema.json"]
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2025.json")).unwrap(),
        "mine"
    );
}

#[test]
fn install_leaves_an_edited_rules_file_alone_and_seeds_again_only_when_no_year_file_remains() {
    let config = temp_config_dir();
    install(config.path()).unwrap();
    let path = dir(config.path()).join("rules-2026.json");
    std::fs::write(&path, "edited").unwrap();
    install(config.path()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited");

    std::fs::rename(&path, dir(config.path()).join("rules-2026.v1.json")).unwrap();
    install(config.path()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SEED);
}

#[test]
fn install_rewrites_a_changed_schema_and_readme() {
    let config = temp_config_dir();
    install(config.path()).unwrap();
    let folder = dir(config.path());
    std::fs::write(folder.join("rules.schema.json"), "old").unwrap();
    std::fs::write(folder.join("README.md"), "old").unwrap();

    install(config.path()).unwrap();

    assert_eq!(
        std::fs::read_to_string(folder.join("rules.schema.json")).unwrap(),
        SCHEMA
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("README.md")).unwrap(),
        README
    );
}

#[test]
fn list_finds_year_files_by_year_and_ignores_backups_and_other_files() {
    let (config, folder) = with_rules_folder();
    for name in [
        "rules-2026.json",
        "rules-2025.json",
        "rules-2025.v1.json",
        "rules.schema.json",
        "README.md",
    ] {
        std::fs::write(folder.join(name), "{}").unwrap();
    }

    let listed = list(config.path()).unwrap();
    assert_eq!(
        listed.into_iter().collect::<Vec<_>>(),
        vec![
            (2025, folder.join("rules-2025.json")),
            (2026, folder.join("rules-2026.json")),
        ]
    );
}

#[test]
fn list_without_a_rules_folder_is_empty() {
    let config = temp_config_dir();
    assert!(list(config.path()).unwrap().is_empty());
}

#[test]
fn read_returns_the_text_or_none_when_the_file_is_missing() {
    let (config, folder) = with_rules_folder();
    std::fs::write(folder.join("rules-2026.json"), "text").unwrap();

    assert_eq!(read(config.path(), 2026).unwrap(), Some("text".to_string()));
    assert_eq!(read(config.path(), 2027).unwrap(), None);
}

#[test]
fn replace_without_an_old_file_writes_it_and_returns_no_backup() {
    let (config, folder) = with_rules_folder();

    assert_eq!(replace(config.path(), 2026, "new", None).unwrap(), None);

    assert_eq!(names_in(&folder), vec!["rules-2026.json"]);
    assert_eq!(read(config.path(), 2026).unwrap(), Some("new".to_string()));
}

#[test]
fn replace_keeps_the_old_file_under_the_version_of_its_ruleset() {
    let (config, folder) = with_rules_folder();
    std::fs::write(folder.join("rules-2026.json"), "old").unwrap();

    let backup = replace(config.path(), 2026, "new", Some("2026 v3")).unwrap();

    assert_eq!(backup, Some("rules-2026.v3.json".to_string()));
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2026.v3.json")).unwrap(),
        "old"
    );
    assert_eq!(read(config.path(), 2026).unwrap(), Some("new".to_string()));
}

#[test]
fn replace_numbers_a_backup_whose_name_is_taken() {
    let (config, folder) = with_rules_folder();
    std::fs::write(folder.join("rules-2026.json"), "first").unwrap();
    replace(config.path(), 2026, "second", Some("2026 v1")).unwrap();

    let backup = replace(config.path(), 2026, "third", Some("2026 v1")).unwrap();

    assert_eq!(backup, Some("rules-2026.v1-2.json".to_string()));
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2026.v1.json")).unwrap(),
        "first"
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2026.v1-2.json")).unwrap(),
        "second"
    );
    assert_eq!(
        read(config.path(), 2026).unwrap(),
        Some("third".to_string())
    );

    let fourth = replace(config.path(), 2026, "fourth", Some("2026 v1")).unwrap();
    assert_eq!(fourth, Some("rules-2026.v1-3.json".to_string()));
}

#[test]
fn replace_names_the_backup_v0_when_the_old_ruleset_is_unknown() {
    let (config, folder) = with_rules_folder();
    std::fs::write(folder.join("rules-2026.json"), "old").unwrap();

    let backup = replace(config.path(), 2026, "new", None).unwrap();

    assert_eq!(backup, Some("rules-2026.v0.json".to_string()));
}

#[test]
fn create_writes_a_new_file_and_refuses_to_overwrite() {
    let (config, folder) = with_rules_folder();
    create(config.path(), 2027, "first").unwrap();
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2027.json")).unwrap(),
        "first"
    );

    let err = create(config.path(), 2027, "second").unwrap_err();
    assert_eq!(
        err,
        AppError::InvalidInput("rules-2027.json already exists".into())
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("rules-2027.json")).unwrap(),
        "first"
    );
}

#[test]
fn next_year_of_the_seed_moves_the_year_the_ruleset_and_the_kilometergeld_dates() {
    let text = next_year_text(SEED).unwrap();
    let next: Value = serde_json::from_str(&text).unwrap();

    let mut expected: Value = serde_json::from_str(SEED).unwrap();
    set(&mut expected, "/year", json!(2027));
    set(&mut expected, "/ruleset", json!("2027 v1"));
    set(&mut expected, "/basedOn", json!("2026 v1"));
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
    assert_eq!(ruleset.based_on.as_deref(), Some("2026 v1"));
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
