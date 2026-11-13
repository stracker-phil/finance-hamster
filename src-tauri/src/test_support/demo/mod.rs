//! The debug build's made-up data (ADR 0022): seeded on the first launch with an empty data folder, and again
//! after `hamster dev seed --yes`. Each feature adds its seeder to `SEEDERS`; every row goes through
//! `Db::write`, so the demo journal reads like the owner's.

use std::path::Path;

use crate::db::{Db, Op, Source};
use crate::error::AppResult;

type Seeder = fn() -> AppResult<Option<Op>>;

/// The ops that build the demo data set, in order.
const SEEDERS: &[Seeder] = &[];

/// Seeds the demo data when the database has never been written to.
pub fn seed_if_empty(db: &Db) -> AppResult<usize> {
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
    use crate::test_support::temp_config_dir;

    fn journal_rows(db: &Db) -> i64 {
        db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap()
    }

    #[test]
    fn seeding_an_empty_database_writes_nothing_without_seeders() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        assert_eq!(seed_if_empty(&db).unwrap(), 0);
        assert_eq!(journal_rows(&db), 0);
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

        assert_eq!(seed_if_empty(&db).unwrap(), 0);
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
