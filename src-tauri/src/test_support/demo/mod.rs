//! The debug build's made-up data (ADR 0022): seeded on the first launch with an empty data folder, and again
//! after `hamster dev seed --yes`. Each feature adds its seeder to `SEEDERS`; every row goes through
//! `Db::write`, so the demo journal reads like the owner's.

use std::path::Path;

use crate::accounts::{self, AccountRow};
use crate::db::{Change, Db, Op, Source};
use crate::error::AppResult;

mod statements;

type Seeder = fn() -> AppResult<Option<Op>>;

/// The ops that build the demo data set, in order.
const SEEDERS: &[Seeder] = &[demo_accounts, demo_rules];

/// Made-up accounts at a made-up bank code; the IBANs' check digits are valid.
pub const DEMO_ACCOUNTS: [(&str, &str); 3] = [
    ("Private", "AT241999900000000001"),
    ("Household", "AT941999900000000002"),
    ("Business", "AT671999900000000003"),
];

fn demo_accounts() -> AppResult<Option<Op>> {
    let demo = DEMO_ACCOUNTS;
    let changes = demo
        .iter()
        .enumerate()
        .map(|(i, (name, iban))| {
            let row = AccountRow {
                id: format!("demo-account-{}", i + 1),
                name: (*name).into(),
                bank: "Example Sparkasse".into(),
                iban: (*iban).into(),
                own: true,
                created_at: i as i64,
            };
            Change::insert(accounts::TABLE, &row)
        })
        .collect::<AppResult<_>>()?;
    Ok(Some(Op::new("Add the demo accounts", changes)))
}

/// The built-in rules and a few of the owner's, so some demo lines arrive decided and some wait for Review.
fn demo_rules() -> AppResult<Option<Op>> {
    use crate::rules::store::{Action, RULES, RuleRow};
    let rule = |n: i64, token: &str, action: Action, item_key: Option<&str>| RuleRow {
        id: format!("demo-rule-{n}"),
        position: n,
        builtin: None,
        contains: serde_json::json!([token]).to_string(),
        amount_cents: None,
        tolerance_cents: 0,
        account_id: None,
        own_counterparty: false,
        action,
        item_key: item_key.map(Into::into),
        subscription_id: None,
        document_id: None,
        created_at: n,
    };
    let mut rows = crate::rules::builtin_rules(0);
    rows.extend([
        rule(1, "NORDWIND", Action::Ignore, None),
        rule(2, "HAUSVERWALTUNG", Action::Ignore, None),
        rule(3, "TANKSTELLE", Action::Ignore, None),
        rule(4, "SEITENWEISE", Action::Anv, Some("literature")),
    ]);
    let changes = rows
        .iter()
        .map(|r| Change::insert(RULES, r))
        .collect::<AppResult<_>>()?;
    Ok(Some(Op::new("Add the demo rules", changes)))
}

/// Seeds the demo data when the database has never been written to; returns how many ops and statements.
pub fn seed_if_empty(db: &Db, config_dir: &Path, today: chrono::NaiveDate) -> AppResult<usize> {
    let written: i64 =
        db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))?;
    if written > 0 {
        return Ok(0);
    }
    let mut count = 0;
    for seeder in SEEDERS {
        if let Some(op) = seeder()? {
            db.write(op, Source::Seed)?;
            count += 1;
        }
    }
    count += statements::seed(db, config_dir, today)?;
    Ok(count)
}

/// What `hamster dev seed --yes` removes: the database, the journal and every stored PDF, so the next launch
/// seeds afresh. Only ever the debug build's own folder.
pub fn reset(config_dir: &Path) -> AppResult<()> {
    for name in [
        crate::db::FILE_NAME,
        "hamster.sqlite-wal",
        "hamster.sqlite-shm",
        crate::db::journal::FILE_NAME,
    ] {
        let path = config_dir.join(name);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    for folder in ["docs", "statements", "snapshots"] {
        let path = config_dir.join(folder);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Change;
    use crate::rules::store::Action;
    use crate::test_support::temp_config_dir;

    fn journal_rows(db: &Db) -> i64 {
        db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap()
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn statement_numbers(db: &Db, account_id: &str) -> Vec<u32> {
        let mut numbers: Vec<u32> = db
            .read(|c| crate::statements::store::statements_of(c, account_id))
            .unwrap()
            .into_iter()
            .map(|s| s.number)
            .collect();
        numbers.sort_unstable();
        numbers
    }

    /// Kind, item and rule of the decision of every line whose text or detail lines contain `needle`.
    type Outcome = Option<(Action, Option<String>, Option<String>)>;

    fn outcomes_of_lines_containing(db: &Db, needle: &str) -> Vec<Outcome> {
        let decisions = db.read(crate::rules::store::decisions).unwrap();
        let lines: Vec<_> = db
            .read(crate::statements::store::all_transactions)
            .unwrap()
            .into_iter()
            .filter(|l| format!("{} {}", l.text, l.extra).contains(needle))
            .collect();
        assert!(!lines.is_empty(), "no line with {needle}");
        lines
            .iter()
            .map(|l| {
                decisions
                    .get(&l.id)
                    .map(|d| (d.kind, d.item_key.clone(), d.rule_id.clone()))
            })
            .collect()
    }

    fn all_decided_as(db: &Db, needle: &str, kind: Action, item: Option<&str>, rule: &str) {
        let expected = Some((kind, item.map(String::from), Some(rule.to_string())));
        for outcome in outcomes_of_lines_containing(db, needle) {
            assert_eq!(outcome, expected, "{needle}");
        }
    }

    #[test]
    fn seeding_an_empty_database_writes_the_accounts_op_the_rules_op_and_one_op_per_demo_statement()
    {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        assert_eq!(seed_if_empty(&db, dir.path(), today()).unwrap(), 2 + 24);

        let accounts = accounts::list(&db).unwrap();
        let shown: Vec<(&str, &str, bool)> = accounts
            .iter()
            .map(|a| (a.name.as_str(), a.iban_masked.as_str(), a.own))
            .collect();
        assert_eq!(
            shown,
            vec![
                ("Private", "AT•• •••• 0001", true),
                ("Household", "AT•• •••• 0002", true),
                ("Business", "AT•• •••• 0003", true),
            ]
        );
        assert_eq!(db.undo().unwrap(), None);
    }

    #[test]
    fn the_seed_has_the_built_in_rules_and_four_demo_rules_in_running_order() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();

        let ids: Vec<String> = db
            .read(crate::rules::store::rules)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();

        assert_eq!(
            ids,
            [
                "builtin-transfers",
                "builtin-bank-fees",
                "demo-rule-1",
                "demo-rule-2",
                "demo-rule-3",
                "demo-rule-4"
            ]
        );
    }

    #[test]
    fn seeded_lines_arrive_decided_by_the_demo_rules_and_the_rest_wait_for_review() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();

        all_decided_as(&db, "NORDWIND", Action::Ignore, None, "demo-rule-1");
        all_decided_as(&db, "HAUSVERWALTUNG", Action::Ignore, None, "demo-rule-2");
        all_decided_as(&db, "TANKSTELLE", Action::Ignore, None, "demo-rule-3");
        all_decided_as(
            &db,
            "SEITENWEISE",
            Action::Anv,
            Some("literature"),
            "demo-rule-4",
        );
        for waiting in ["STREAMFLIX", "LUMEN AI", "CAFE"] {
            for outcome in outcomes_of_lines_containing(&db, waiting) {
                assert_eq!(outcome, None, "{waiting}");
            }
        }
    }

    #[test]
    fn seeded_transfers_to_household_are_ignored_by_the_built_in_rule() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();

        all_decided_as(&db, "HOUSEHOLD", Action::Ignore, None, "builtin-transfers");
        assert_eq!(outcomes_of_lines_containing(&db, "HOUSEHOLD").len(), 11);
    }

    #[test]
    fn every_account_gets_statements_and_household_lacks_february() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();

        assert_eq!(
            statement_numbers(&db, "demo-account-1"),
            (1..=11).collect::<Vec<u32>>()
        );
        assert_eq!(
            statement_numbers(&db, "demo-account-2"),
            vec![1, 3, 4, 5, 6, 7, 8, 9, 10, 11]
        );
        assert_eq!(statement_numbers(&db, "demo-account-3"), vec![9, 10, 11]);
    }

    #[test]
    fn the_current_months_statement_ends_three_days_before_today() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();

        let newest = db
            .read(|c| crate::statements::store::statements_of(c, "demo-account-1"))
            .unwrap()
            .remove(0);

        assert_eq!((newest.number, newest.year), (11, 2026));
        assert_eq!(newest.to_date, "2026-11-10");
        assert_eq!(newest.file, "statements/2026/private-011.pdf");
        assert!(dir.path().join(&newest.file).exists());
    }

    #[test]
    fn seeding_twice_changes_nothing_the_second_time() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        seed_if_empty(&db, dir.path(), today()).unwrap();
        let before = journal_rows(&db);

        assert_eq!(seed_if_empty(&db, dir.path(), today()).unwrap(), 0);
        assert_eq!(journal_rows(&db), before);
        assert_eq!(accounts::list(&db).unwrap().len(), 3);
        assert_eq!(statement_numbers(&db, "demo-account-1").len(), 11);
    }

    #[test]
    fn the_demo_ibans_have_valid_check_digits() {
        let ops = SEEDERS.iter().filter_map(|seeder| seeder().unwrap());
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        for op in ops {
            db.write(op, Source::Seed).unwrap();
        }
        let ibans: Vec<String> = db
            .read(|conn| {
                let mut query = conn.prepare("SELECT iban FROM accounts ORDER BY created_at")?;
                let rows = query.query_map([], |r| r.get(0))?;
                Ok(rows.collect::<Result<_, _>>()?)
            })
            .unwrap();
        for iban in ibans {
            assert_eq!(
                accounts::normalize_iban(&iban),
                Some(iban.clone()),
                "{iban}"
            );
        }
    }

    #[test]
    fn a_database_that_was_written_to_is_not_seeded() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.read(|conn| {
            conn.execute_batch("CREATE TABLE things (id TEXT PRIMARY KEY)")?;
            Ok(())
        })
        .unwrap();
        let op = Op::new(
            "Add",
            vec![Change::insert("things", &serde_json::json!({ "id": "a" })).unwrap()],
        );
        db.write(op, Source::Ui).unwrap();
        let before = journal_rows(&db);

        assert_eq!(seed_if_empty(&db, dir.path(), today()).unwrap(), 0);
        assert_eq!(journal_rows(&db), before);
    }

    #[test]
    fn reset_removes_the_data_and_keeps_everything_else() {
        let dir = temp_config_dir();
        let root = dir.path();
        for name in [
            "hamster.sqlite",
            "hamster.sqlite-wal",
            "hamster.sqlite-shm",
            "journal.jsonl",
            "settings.json",
        ] {
            std::fs::write(root.join(name), "x").unwrap();
        }
        for folder in ["docs", "statements", "snapshots", "rules"] {
            std::fs::create_dir_all(root.join(folder).join("2026")).unwrap();
            std::fs::write(root.join(folder).join("2026").join("file.pdf"), "x").unwrap();
        }

        reset(root).unwrap();

        for gone in [
            "hamster.sqlite",
            "hamster.sqlite-wal",
            "hamster.sqlite-shm",
            "journal.jsonl",
            "docs",
            "statements",
            "snapshots",
        ] {
            assert!(!root.join(gone).exists(), "{gone}");
        }
        assert!(root.join("settings.json").exists());
        assert!(root.join("rules").join("2026").join("file.pdf").exists());
    }

    #[test]
    fn reset_of_an_empty_folder_succeeds() {
        let dir = temp_config_dir();
        reset(dir.path()).unwrap();
    }
}
