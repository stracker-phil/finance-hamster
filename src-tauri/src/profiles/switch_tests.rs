use std::path::Path;

use super::*;
use crate::test_support::temp_config_dir;

fn open_first(root: &Path) -> Db {
    let active = crate::profiles::open(root, "Demo").unwrap();
    Db::open(&active.dir).unwrap()
}

fn open_added(root: &Path, name: &str) -> Db {
    let profile = crate::profiles::add(root, name).unwrap();
    Db::open(&crate::profiles::dir(root, &profile.id)).unwrap()
}

fn account_count(db: &Db) -> usize {
    db.read(crate::accounts::rows).unwrap().len()
}

fn rule_ids(db: &Db) -> Vec<String> {
    let mut ids: Vec<String> = db
        .read(crate::rules::store::rules)
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    ids.sort();
    ids
}

fn tax_years(db: &Db) -> Vec<i32> {
    db.read(crate::taxyears::rows)
        .unwrap()
        .into_iter()
        .map(|r| r.year)
        .collect()
}

#[test]
fn a_fresh_profile_gets_the_built_in_rules() {
    let root = temp_config_dir();
    open_first(root.path());
    let db = open_added(root.path(), "Second");

    prepare(root.path(), &db);

    assert_eq!(rule_ids(&db), ["builtin-bank-fees", "builtin-transfers"]);
}

#[test]
fn a_fresh_profile_starts_the_tax_years_that_have_built_in_rules_and_reports_it_once() {
    let root = temp_config_dir();
    open_first(root.path());
    let db = open_added(root.path(), "Second");

    let first_notices = prepare(root.path(), &db);
    let second_notices = prepare(root.path(), &db);

    assert_eq!(tax_years(&db), [2026, 2025]);
    assert_eq!(first_notices, ["Start tax year"]);
    assert_eq!(second_notices, Vec::<String>::new());
}

#[test]
fn the_first_profile_gets_the_demo_data_in_the_debug_build() {
    let root = temp_config_dir();
    let db = open_first(root.path());
    assert_eq!(account_count(&db), 0);

    prepare(root.path(), &db);

    assert!(account_count(&db) > 0);
}

#[test]
fn a_later_profile_stays_without_demo_data() {
    let root = temp_config_dir();
    let first = open_first(root.path());
    let second = open_added(root.path(), "Second");

    prepare(root.path(), &first);
    prepare(root.path(), &second);

    assert!(account_count(&first) > 0);
    assert_eq!(account_count(&second), 0);
}
