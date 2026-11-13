//! The schema as numbered migrations under `PRAGMA user_version` (ADR 0012). A migration is never edited once
//! released; a change is a new one at the end.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

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
];

pub fn version(conn: &Connection) -> AppResult<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version as usize)
}

/// Brings the file to the newest schema; a file from a newer Hamster is refused untouched.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current = version(conn)?;
    if current > MIGRATIONS.len() {
        return Err(AppError::InvalidInput(format!(
            "The data file is from a newer Hamster (schema {current}, this version knows {})",
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
        assert_eq!(tables(&conn), vec!["journal".to_string()]);
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
