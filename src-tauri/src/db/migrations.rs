//! The schema as numbered migrations under `PRAGMA user_version` (ADR 0012). A migration is never edited once
//! released; a change is a new one at the end.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::tr;

/// `MIGRATIONS[n]` brings the schema from version `n` to `n + 1`.
pub const MIGRATIONS: &[&str] = &[
    // 1: the journal, one row per changed field (ADR 0007).
    "CREATE TABLE journal (
        seq INTEGER PRIMARY KEY AUTOINCREMENT,
        op INTEGER NOT NULL,
        at INTEGER NOT NULL,
        label TEXT NOT NULL,
        source TEXT NOT NULL,
        entity TEXT NOT NULL,
        entity_id TEXT NOT NULL,
        field TEXT NOT NULL,
        old TEXT,
        new TEXT
    );
    CREATE INDEX journal_entity ON journal (entity, entity_id);",
    // 2: bank accounts and tax years; a tax year keeps the rules text it applies (ADR 0040).
    "CREATE TABLE accounts (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        bank TEXT NOT NULL,
        iban TEXT NOT NULL UNIQUE,
        own INTEGER NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE tax_years (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL UNIQUE,
        state TEXT NOT NULL,
        ruleset TEXT NOT NULL,
        rules TEXT NOT NULL,
        rules_loaded_at INTEGER NOT NULL,
        closed_at INTEGER
    );",
    // 3: imported statements and their lines, stored once with the layout that read them.
    "CREATE TABLE statements (
        id TEXT PRIMARY KEY,
        account_id TEXT NOT NULL REFERENCES accounts (id),
        number INTEGER NOT NULL,
        year INTEGER NOT NULL,
        from_date TEXT NOT NULL,
        to_date TEXT NOT NULL,
        old_balance INTEGER NOT NULL,
        credits INTEGER NOT NULL,
        debits INTEGER NOT NULL,
        new_balance INTEGER NOT NULL,
        layout TEXT NOT NULL,
        file_hash TEXT NOT NULL UNIQUE,
        file TEXT NOT NULL,
        pages INTEGER NOT NULL,
        imported_at INTEGER NOT NULL,
        UNIQUE (account_id, year, number)
    );
    CREATE TABLE transactions (
        id TEXT PRIMARY KEY,
        statement_id TEXT NOT NULL REFERENCES statements (id),
        account_id TEXT NOT NULL REFERENCES accounts (id),
        page INTEGER NOT NULL,
        row INTEGER NOT NULL,
        value_date TEXT NOT NULL,
        cents INTEGER NOT NULL,
        text TEXT NOT NULL,
        extra TEXT NOT NULL,
        counterparty TEXT,
        counterparty_iban TEXT,
        card_kind TEXT,
        card_date TEXT,
        card_time TEXT,
        merchant TEXT,
        original_cents INTEGER,
        original_currency TEXT,
        fx_fee INTEGER,
        fx_rate TEXT,
        fx_date TEXT,
        transfer INTEGER NOT NULL
    );
    CREATE INDEX transactions_statement ON transactions (statement_id);
    CREATE INDEX transactions_value_date ON transactions (value_date);",
    // 4: rules, and how each line was decided (by a rule or by hand); the lines themselves stay as imported.
    "CREATE TABLE rules (
        id TEXT PRIMARY KEY,
        position INTEGER NOT NULL,
        builtin TEXT,
        contains TEXT NOT NULL,
        amount_cents INTEGER,
        tolerance_cents INTEGER NOT NULL,
        account_id TEXT,
        own_counterparty INTEGER NOT NULL,
        action TEXT NOT NULL,
        item_key TEXT,
        subscription_id TEXT,
        document_id TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE decisions (
        id TEXT PRIMARY KEY REFERENCES transactions (id),
        kind TEXT NOT NULL,
        item_key TEXT,
        subscription_id TEXT,
        document_id TEXT,
        rule_id TEXT,
        decided_at INTEGER NOT NULL
    );
    CREATE INDEX decisions_rule ON decisions (rule_id);",
    // 5: vendors, documents (an invoice PDF plus its facts) and their links to the bank lines that paid them.
    "CREATE TABLE vendors (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        default_item_key TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE documents (
        id TEXT PRIMARY KEY,
        state TEXT NOT NULL,
        file TEXT NOT NULL,
        original_name TEXT NOT NULL,
        file_hash TEXT NOT NULL UNIQUE,
        vendor_id TEXT,
        pay_date TEXT,
        paid_cents INTEGER,
        item_key TEXT,
        work_share INTEGER NOT NULL,
        note TEXT NOT NULL,
        invoice_cents INTEGER,
        invoice_currency TEXT,
        invoice_number TEXT,
        invoice_date TEXT,
        created_at INTEGER NOT NULL,
        finished_at INTEGER,
        excluded_at INTEGER,
        exclude_reason TEXT
    );
    CREATE TABLE document_links (
        id TEXT PRIMARY KEY,
        document_id TEXT NOT NULL REFERENCES documents (id),
        transaction_id TEXT NOT NULL REFERENCES transactions (id),
        created_at INTEGER NOT NULL,
        UNIQUE (document_id, transaction_id)
    );
    CREATE INDEX document_links_transaction ON document_links (transaction_id);",
    // 6: the permanent deletion log: what was deleted from the Archive, when and why.
    "CREATE TABLE deletions (
        id TEXT PRIMARY KEY,
        document_id TEXT NOT NULL,
        title TEXT NOT NULL,
        original_name TEXT NOT NULL,
        file TEXT NOT NULL,
        file_hash TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        excluded_at INTEGER,
        deleted_at INTEGER NOT NULL,
        reason TEXT NOT NULL
    );",
    // 7: the answers to a tax year's questions (Telearbeitstage, …): a count, or cents for an amount.
    "CREATE TABLE answers (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL,
        question TEXT NOT NULL,
        value INTEGER NOT NULL,
        answered_at INTEGER NOT NULL,
        UNIQUE (year, question)
    );",
    // 8: a document's own useful life, overriding its item's asset class.
    "ALTER TABLE documents ADD COLUMN useful_life INTEGER;",
    // 9: amounts and trips entered on the tax year page; a trip's value comes from the rate valid on its date.
    "CREATE TABLE entries (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL,
        item_key TEXT NOT NULL,
        kind TEXT NOT NULL,
        date TEXT NOT NULL,
        cents INTEGER,
        km INTEGER,
        text TEXT NOT NULL,
        evidence TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX entries_year ON entries (year);",
    // 10: subscriptions and their plans; a plan's period decides which plan a charge belongs to.
    "CREATE TABLE subscriptions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        tone INTEGER NOT NULL,
        category TEXT NOT NULL,
        ended_on TEXT,
        reminder TEXT NOT NULL,
        item_key TEXT,
        match_text TEXT NOT NULL,
        tolerance_cents INTEGER,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE plans (
        id TEXT PRIMARY KEY,
        subscription_id TEXT NOT NULL,
        name TEXT NOT NULL,
        cents INTEGER NOT NULL,
        currency TEXT NOT NULL,
        interval TEXT NOT NULL,
        start TEXT NOT NULL,
        end TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX plans_subscription ON plans (subscription_id);",
    // 11: what a tax year counted when it was closed, kept when it is reopened.
    "CREATE TABLE snapshots (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL,
        closed_at INTEGER NOT NULL,
        ruleset TEXT NOT NULL,
        total TEXT NOT NULL,
        kennzahlen TEXT NOT NULL,
        documents TEXT NOT NULL,
        file TEXT NOT NULL
    );
    CREATE INDEX snapshots_year ON snapshots (year);",
    // 12: when each tax year's year check was last completed; nothing per question.
    "CREATE TABLE year_checks (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL UNIQUE,
        completed_at INTEGER NOT NULL
    );",
    // 13: the name other statements print for an account, to tell transfers when they print no IBAN.
    "ALTER TABLE accounts ADD COLUMN statement_name TEXT;",
    // 14: a subscription's tolerance in percent of its plan, rounded up from the old one in cents against the
    // latest plan; tolerance_cents stays for the journal's older ops.
    "ALTER TABLE subscriptions ADD COLUMN tolerance_percent INTEGER;
    UPDATE subscriptions SET tolerance_percent = CASE
        WHEN tolerance_cents IS NULL THEN NULL
        WHEN tolerance_cents = 0 THEN 0
        ELSE COALESCE(
            (SELECT MIN(99, (ABS(subscriptions.tolerance_cents) * 100 + p.cents - 1) / p.cents)
             FROM plans p WHERE p.subscription_id = subscriptions.id AND p.cents > 0
             ORDER BY p.start DESC LIMIT 1),
            0)
    END;",
    // 15: a subscription asks about another amount unless it takes only its exact amount; the tolerance
    // columns stay for the journal's older ops.
    "ALTER TABLE subscriptions ADD COLUMN exact_only INTEGER NOT NULL DEFAULT 0;",
    // 16: whether a tax year follows the built-in rules or keeps the owner's version (ADR 0051).
    "ALTER TABLE tax_years ADD COLUMN rules_source TEXT NOT NULL DEFAULT 'builtin';",
    // 17: categories the owner hides while recording a tax year (ADR 0053).
    "CREATE TABLE hidden_items (
        id TEXT PRIMARY KEY,
        year INTEGER NOT NULL,
        item_key TEXT NOT NULL,
        UNIQUE (year, item_key)
    );",
];

pub fn version(conn: &Connection) -> AppResult<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version as usize)
}

/// Brings the file to the newest schema; a file from a newer Hamster is refused untouched.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current = version(conn)?;
    if current > MIGRATIONS.len() {
        return Err(AppError::InvalidInput(tr!(
            "The data file is from a newer Hamster (schema {current}, this version knows {})",
            "Die Datendatei stammt von einem neueren Hamster (Schema {current}, diese Version kennt {})",
            MIGRATIONS.len()
        )));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    fn tables(conn: &Connection) -> Vec<String> {
        let mut query = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
        query
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn a_fresh_file_reaches_the_newest_version() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("fresh.sqlite")).unwrap();
        assert_eq!(version(&conn).unwrap(), 0);
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    fn all_tables() -> Vec<String> {
        [
            "accounts",
            "answers",
            "decisions",
            "deletions",
            "document_links",
            "documents",
            "entries",
            "hidden_items",
            "journal",
            "plans",
            "rules",
            "snapshots",
            "statements",
            "subscriptions",
            "tax_years",
            "transactions",
            "vendors",
            "year_checks",
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn a_file_at_version_sixteen_gains_the_hidden_categories_table_and_keeps_its_tax_years() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v16.sqlite")).unwrap();
        for sql in &MIGRATIONS[..16] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 16i64).unwrap();
        conn.execute(
            "INSERT INTO tax_years (id, year, state, ruleset, rules, rules_loaded_at) \
             VALUES (2026, 2026, 'open', '2026 v1', '{}', 1)",
            [],
        )
        .unwrap();
        assert!(!tables(&conn).contains(&"hidden_items".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
        let years: i64 = conn
            .query_row("SELECT COUNT(*) FROM tax_years", [], |r| r.get(0))
            .unwrap();
        let hidden: i64 = conn
            .query_row("SELECT COUNT(*) FROM hidden_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!((years, hidden), (1, 0));
    }

    #[test]
    fn a_file_at_version_fifteen_gains_the_built_in_rules_source_for_every_tax_year() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v15.sqlite")).unwrap();
        for sql in &MIGRATIONS[..15] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 15i64).unwrap();
        for (year, state, closed_at) in [(2025, "closed", Some(5)), (2026, "open", None)] {
            conn.execute(
                "INSERT INTO tax_years (id, year, state, ruleset, rules, rules_loaded_at, closed_at) \
                 VALUES (?1, ?1, ?2, ?3, '{}', 1, ?4)",
                rusqlite::params![year, state, format!("{year} v1"), closed_at],
            )
            .unwrap();
        }

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        let mut query = conn
            .prepare("SELECT year, rules_source FROM tax_years ORDER BY year")
            .unwrap();
        let sources: Vec<(i64, String)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            sources,
            [(2025, "builtin".to_string()), (2026, "builtin".to_string())]
        );
    }

    #[test]
    fn a_file_at_version_fourteen_gains_exact_only_off_for_every_subscription() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v14.sqlite")).unwrap();
        for sql in &MIGRATIONS[..14] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 14i64).unwrap();
        for (id, tolerance) in [
            ("sub-exact", Some(0)),
            ("sub-any", None),
            ("sub-wide", Some(15)),
        ] {
            conn.execute(
                "INSERT INTO subscriptions (id, name, tone, category, reminder, match_text, tolerance_percent, created_at) \
                 VALUES (?1, ?1, 1, 'Media', 'off', 'TEXT', ?2, 1)",
                rusqlite::params![id, tolerance],
            )
            .unwrap();
        }

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        let mut query = conn
            .prepare("SELECT id, exact_only FROM subscriptions ORDER BY id")
            .unwrap();
        let flags: Vec<(String, i64)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            flags,
            [
                ("sub-any".to_string(), 0),
                ("sub-exact".to_string(), 0),
                ("sub-wide".to_string(), 0),
            ]
        );
    }

    #[test]
    fn a_file_at_version_thirteen_converts_subscription_tolerances_to_percent_of_the_plan() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v13.sqlite")).unwrap();
        for sql in &MIGRATIONS[..13] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 13i64).unwrap();
        for (id, tolerance) in [
            ("sub-tolerant", Some(150)),
            ("sub-any", None),
            ("sub-exact", Some(0)),
            ("sub-huge", Some(5_000)),
            ("sub-no-plan", Some(150)),
        ] {
            conn.execute(
                "INSERT INTO subscriptions (id, name, tone, category, reminder, match_text, tolerance_cents, created_at) \
                 VALUES (?1, ?1, 1, 'Media', 'off', 'TEXT', ?2, 1)",
                rusqlite::params![id, tolerance],
            )
            .unwrap();
            if id != "sub-no-plan" {
                conn.execute(
                    "INSERT INTO plans (id, subscription_id, name, cents, currency, interval, start, created_at) \
                     VALUES (?1, ?2, 'Standard', 1000, 'EUR', 'monthly', '2026-01-01', 1)",
                    rusqlite::params![format!("plan-{id}"), id],
                )
                .unwrap();
            }
        }

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        let mut query = conn
            .prepare("SELECT id, tolerance_percent FROM subscriptions ORDER BY id")
            .unwrap();
        let converted: Vec<(String, Option<i64>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            converted,
            [
                ("sub-any".to_string(), None),
                ("sub-exact".to_string(), Some(0)),
                ("sub-huge".to_string(), Some(99)),
                ("sub-no-plan".to_string(), Some(0)),
                ("sub-tolerant".to_string(), Some(15)),
            ]
        );
    }

    #[test]
    fn a_file_at_version_twelve_gains_the_statement_name_and_keeps_its_accounts() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v12.sqlite")).unwrap();
        for sql in &MIGRATIONS[..12] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 12i64).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, bank, iban, own, created_at) \
             VALUES ('account-1', 'Private', 'Example Sparkasse', 'AT241999900000000001', 1, 1)",
            [],
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        let (name, statement_name): (String, Option<String>) = conn
            .query_row(
                "SELECT name, statement_name FROM accounts WHERE id = 'account-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((name.as_str(), statement_name), ("Private", None));
    }

    #[test]
    fn a_file_at_version_nine_gains_subscriptions_and_plans_and_keeps_its_entries() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v9.sqlite")).unwrap();
        for sql in &MIGRATIONS[..9] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 9i64).unwrap();
        assert!(!tables(&conn).contains(&"subscriptions".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
        assert!(tables(&conn).contains(&"entries".to_string()));
    }

    #[test]
    fn a_file_at_version_eight_gains_the_entries_table_and_keeps_its_documents() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v8.sqlite")).unwrap();
        for sql in &MIGRATIONS[..8] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 8i64).unwrap();
        conn.execute(
            "INSERT INTO documents (id, state, file, original_name, file_hash, work_share, note, created_at) \
             VALUES ('doc-1', 'draft', 'docs/inbox/a.pdf', 'a.pdf', 'hash', 100, '', 1)",
            [],
        )
        .unwrap();
        assert!(!tables(&conn).contains(&"entries".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
        let documents: i64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(documents, 1);
    }

    fn document_columns(conn: &Connection) -> Vec<String> {
        let mut query = conn.prepare("PRAGMA table_info(documents)").unwrap();
        query
            .query_map([], |r| r.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn a_file_at_version_seven_gains_the_useful_life_column_and_keeps_its_documents() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v7.sqlite")).unwrap();
        for sql in &MIGRATIONS[..7] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 7i64).unwrap();
        conn.execute(
            "INSERT INTO documents (id, state, file, original_name, file_hash, work_share, note, created_at) \
             VALUES ('doc-1', 'draft', 'docs/inbox/a.pdf', 'a.pdf', 'hash', 100, '', 1)",
            [],
        )
        .unwrap();
        assert!(!document_columns(&conn).contains(&"useful_life".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
        assert!(document_columns(&conn).contains(&"useful_life".to_string()));
        let life: Option<i64> = conn
            .query_row(
                "SELECT useful_life FROM documents WHERE id = 'doc-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(life, None);
    }

    #[test]
    fn a_file_at_version_six_gains_the_answers_and_keeps_its_deletion_log() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v6.sqlite")).unwrap();
        for sql in &MIGRATIONS[..6] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 6i64).unwrap();
        assert!(!tables(&conn).contains(&"answers".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    #[test]
    fn a_file_at_version_five_gains_the_deletion_log_and_keeps_its_documents() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v5.sqlite")).unwrap();
        for sql in &MIGRATIONS[..5] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 5i64).unwrap();
        assert!(!tables(&conn).contains(&"deletions".to_string()));

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    #[test]
    fn a_file_at_version_four_gains_vendors_documents_and_their_links() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v4.sqlite")).unwrap();
        for sql in &MIGRATIONS[..4] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 4i64).unwrap();
        assert_eq!(
            tables(&conn),
            [
                "accounts",
                "decisions",
                "journal",
                "rules",
                "statements",
                "tax_years",
                "transactions"
            ]
            .map(String::from)
            .to_vec()
        );

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    #[test]
    fn a_file_at_version_two_gains_statements_and_transactions() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v2.sqlite")).unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute_batch(MIGRATIONS[1]).unwrap();
        conn.pragma_update(None, "user_version", 2i64).unwrap();
        assert_eq!(
            tables(&conn),
            ["accounts", "journal", "tax_years"]
                .map(String::from)
                .to_vec()
        );

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    #[test]
    fn a_file_at_version_three_gains_rules_and_decisions() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v3.sqlite")).unwrap();
        for sql in &MIGRATIONS[..3] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 3i64).unwrap();
        assert_eq!(
            tables(&conn),
            [
                "accounts",
                "journal",
                "statements",
                "tax_years",
                "transactions"
            ]
            .map(String::from)
            .to_vec()
        );

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
    }

    #[test]
    fn a_file_at_version_one_gains_every_later_table_and_keeps_its_journal() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("v1.sqlite")).unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1i64).unwrap();
        conn.execute(
            "INSERT INTO journal (op, at, label, source, entity, entity_id, field) VALUES (1, 1, 'Add', 'ui', 'things', 'a', 'id')",
            [],
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        assert_eq!(version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(tables(&conn), all_tables());
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn migrating_again_changes_nothing() {
        let dir = temp_config_dir();
        let path = dir.path().join("again.sqlite");
        let mut conn = Connection::open(&path).unwrap();
        migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO journal (op, at, label, source, entity, entity_id, field) VALUES (1, 1, 'Add', 'ui', 'things', 'a', 'id')",
            [],
        )
        .unwrap();
        drop(conn);

        let mut reopened = Connection::open(&path).unwrap();
        migrate(&mut reopened).unwrap();
        assert_eq!(version(&reopened).unwrap(), MIGRATIONS.len());
        let rows: i64 = reopened
            .query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_file_from_a_newer_hamster_is_refused_and_left_unchanged() {
        let dir = temp_config_dir();
        let mut conn = Connection::open(dir.path().join("newer.sqlite")).unwrap();
        let newer = MIGRATIONS.len() as i64 + 1;
        conn.pragma_update(None, "user_version", newer).unwrap();

        let err = migrate(&mut conn).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        assert_eq!(version(&conn).unwrap(), newer as usize);
        assert_eq!(tables(&conn), Vec::<String>::new());
    }
}
