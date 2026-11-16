use serde_json::json;

use super::*;

mod frozen;
use crate::test_support::temp_config_dir;

type ThingRow = (String, Option<String>, Option<i64>, Option<i64>);

fn open() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.read(|conn| {
        conn.execute_batch(
            "CREATE TABLE things (id TEXT PRIMARY KEY, name TEXT, cents INTEGER, done INTEGER)",
        )?;
        Ok(())
    })
    .unwrap();
    (dir, db)
}

fn thing(id: &str, name: &str, cents: i64) -> serde_json::Value {
    json!({ "id": id, "name": name, "cents": cents, "done": false })
}

fn insert(id: &str, name: &str, cents: i64) -> Op {
    Op::new(
        format!("Add {name}"),
        vec![Change::insert("things", &thing(id, name, cents)).unwrap()],
    )
}

fn rename(id: &str, from: &str, to: &str) -> Op {
    Op::new(
        format!("Rename {from}"),
        vec![Change::update("things", &thing(id, from, 100), &thing(id, to, 100)).unwrap()],
    )
}

fn things(db: &Db) -> Vec<ThingRow> {
    db.read(|conn| {
        let mut query = conn.prepare("SELECT id, name, cents, done FROM things ORDER BY id")?;
        let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
    .unwrap()
}

fn journal_rows(db: &Db) -> i64 {
    db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
        .unwrap()
}

fn row(id: &str, name: &str, cents: i64) -> ThingRow {
    (id.into(), Some(name.into()), Some(cents), Some(0))
}

fn journal_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(journal::FILE_NAME))
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn the_database_reports_the_folder_it_was_opened_in() {
    let (dir, db) = open();

    assert_eq!(db.dir(), dir.path());
}

#[test]
fn an_op_keeps_its_label_and_changes() {
    let change = Change::insert("things", &thing("a", "Rent", 100)).unwrap();
    let op = Op::new("Add Rent", vec![change.clone()]);
    assert_eq!(op.label, "Add Rent");
    assert_eq!(op.changes, vec![change]);
}

#[test]
fn an_inverse_op_reverses_the_order_and_inverts_each_change() {
    let first = Change::insert("things", &thing("a", "Rent", 100)).unwrap();
    let second =
        Change::update("things", &thing("a", "Rent", 100), &thing("a", "Flat", 100)).unwrap();
    let op = Op::new("Add and rename", vec![first.clone(), second.clone()]);
    assert_eq!(
        op.inverse(),
        Op::new("Add and rename", vec![second.inverse(), first.inverse()])
    );
}

#[test]
fn an_op_lists_each_touched_table_once_in_order() {
    let op = Op::new(
        "Mixed",
        vec![
            Change::insert("zebras", &json!({ "id": "1" })).unwrap(),
            Change::insert("apples", &json!({ "id": "1" })).unwrap(),
            Change::delete("zebras", &json!({ "id": "2" })).unwrap(),
        ],
    );
    assert_eq!(
        op.tables(),
        vec!["apples".to_string(), "zebras".to_string()]
    );
}

#[test]
fn sources_are_named_for_the_journal() {
    for (source, name) in [
        (Source::Ui, "ui"),
        (Source::Cli, "cli"),
        (Source::Import, "import"),
        (Source::Seed, "seed"),
        (Source::App, "app"),
        (Source::Undo, "undo"),
        (Source::Redo, "redo"),
    ] {
        assert_eq!(source.as_str(), name);
    }
}

#[test]
fn a_write_inserts_the_row_and_reports_label_and_tables() {
    let (_dir, db) = open();
    let written = db.write(insert("a", "Rent", 1250), Source::Ui).unwrap();
    assert_eq!(
        written,
        Written {
            label: "Add Rent".into(),
            tables: vec!["things".into()]
        }
    );
    assert_eq!(things(&db), vec![row("a", "Rent", 1250)]);
}

#[test]
fn a_write_updates_only_the_changed_fields() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    assert_eq!(things(&db), vec![row("a", "Flat", 100)]);
}

#[test]
fn a_write_deletes_the_row() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let delete = Op::new(
        "Delete Rent",
        vec![Change::delete("things", &thing("a", "Rent", 100)).unwrap()],
    );
    db.write(delete, Source::Ui).unwrap();
    assert_eq!(things(&db), vec![]);
}

#[test]
fn booleans_store_as_integers_and_nested_json_as_text() {
    let (_dir, db) = open();
    let record = json!({ "id": "a", "name": { "tags": [1, 2] }, "cents": null, "done": true });
    let op = Op::new("Add", vec![Change::insert("things", &record).unwrap()]);
    db.write(op, Source::Ui).unwrap();
    assert_eq!(
        things(&db),
        vec![(
            "a".to_string(),
            Some(r#"{"tags":[1,2]}"#.to_string()),
            None,
            Some(1)
        )]
    );
}

#[test]
fn updating_a_missing_row_is_not_found_and_leaves_everything_unchanged() {
    let (dir, db) = open();
    let op = Op::new(
        "Add and rename",
        vec![
            Change::insert("things", &thing("a", "Rent", 100)).unwrap(),
            Change::update("things", &thing("zz", "Old", 1), &thing("zz", "New", 1)).unwrap(),
        ],
    );
    let err = db.write(op, Source::Ui).unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    assert_eq!(things(&db), vec![]);
    assert_eq!(journal_rows(&db), 0);
    assert!(!dir.path().join(journal::FILE_NAME).exists());
}

#[test]
fn deleting_a_missing_row_is_not_found() {
    let (_dir, db) = open();
    let op = Op::new(
        "Delete",
        vec![Change::delete("things", &thing("zz", "Old", 1)).unwrap()],
    );
    assert!(matches!(
        db.write(op, Source::Ui).unwrap_err(),
        AppError::NotFound(_)
    ));
}

#[test]
fn a_duplicate_id_fails_and_leaves_the_table_and_journal_unchanged() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let journal_before = journal_rows(&db);
    let op = Op::new(
        "Add twice",
        vec![
            Change::insert("things", &thing("b", "Other", 5)).unwrap(),
            Change::insert("things", &thing("a", "Again", 9)).unwrap(),
        ],
    );
    assert!(db.write(op, Source::Ui).is_err());
    assert_eq!(things(&db), vec![row("a", "Rent", 100)]);
    assert_eq!(journal_rows(&db), journal_before);
}

#[test]
fn a_failed_write_is_not_undoable() {
    let (_dir, db) = open();
    let op = Op::new(
        "Rename missing",
        vec![Change::update("things", &thing("zz", "Old", 1), &thing("zz", "New", 1)).unwrap()],
    );
    assert!(db.write(op, Source::Ui).is_err());
    assert_eq!(db.undo().unwrap(), None);
}

#[test]
fn an_op_without_changes_is_invalid_input() {
    let (_dir, db) = open();
    let err = db
        .write(Op::new("Nothing", vec![]), Source::Ui)
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
}

#[test]
fn names_that_are_not_plain_identifiers_are_rejected() {
    let (_dir, db) = open();
    let bad_table = Op::new(
        "Bad table",
        vec![Change::insert("things; DROP TABLE things", &json!({ "id": "a" })).unwrap()],
    );
    let bad_column = Op::new(
        "Bad column",
        vec![
            Change::insert(
                "things",
                &json!({ "id": "a", "name) VALUES ('x'); --": "x" }),
            )
            .unwrap(),
        ],
    );
    let leading_digit = Op::new(
        "Digit table",
        vec![Change::delete("1things", &json!({ "id": "a" })).unwrap()],
    );
    for op in [bad_table, bad_column, leading_digit] {
        let label = op.label.clone();
        let err = db.write(op, Source::Ui).unwrap_err();
        assert!(matches!(err, AppError::Internal(_)), "{label}: {err:?}");
    }
    assert_eq!(things(&db), vec![]);
}

#[test]
fn a_record_that_is_not_an_object_cannot_become_a_change() {
    let err = Change::insert("things", &json!(["a", "b"])).unwrap_err();
    assert!(matches!(err, AppError::Internal(_)), "{err:?}");
}

#[test]
fn undo_restores_the_previous_state_and_returns_the_label() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();

    let undone = db.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Rename Rent");
    assert_eq!(undone.tables, vec!["things".to_string()]);
    assert_eq!(things(&db), vec![row("a", "Rent", 100)]);

    db.undo().unwrap().unwrap();
    assert_eq!(things(&db), vec![]);
}

#[test]
fn redo_applies_the_undone_change_again() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Cli).unwrap();
    db.undo().unwrap();

    let redone = db.redo().unwrap().unwrap();
    assert_eq!(redone.label, "Rename Rent");
    assert_eq!(things(&db), vec![row("a", "Flat", 100)]);
    assert_eq!(db.redo().unwrap(), None);
}

#[test]
fn undo_and_redo_with_nothing_to_do_return_none() {
    let (_dir, db) = open();
    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(db.redo().unwrap(), None);
}

#[test]
fn a_new_write_clears_redo() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.undo().unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    assert_eq!(db.redo().unwrap(), None);
    assert_eq!(things(&db), vec![row("b", "Gym", 200)]);
}

#[test]
fn the_undo_history_keeps_only_the_newest_hundred_changes() {
    let (_dir, db) = open();
    for n in 0..101 {
        db.write(insert(&format!("r{n:03}"), "Rent", 100), Source::Ui)
            .unwrap();
    }
    for _ in 0..100 {
        assert!(db.undo().unwrap().is_some());
    }
    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(things(&db), vec![row("r000", "Rent", 100)]);
}

#[test]
fn a_new_write_after_undo_drops_every_undone_change_from_redo() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    db.undo().unwrap();
    db.undo().unwrap();
    db.write(insert("c", "Bike", 300), Source::Ui).unwrap();
    assert_eq!(db.redo().unwrap(), None);
    assert_eq!(things(&db), vec![row("c", "Bike", 300)]);
}

#[test]
fn an_undo_whose_row_was_deleted_meanwhile_fails_and_changes_nothing() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let delete = Op::new(
        "Remove Rent",
        vec![Change::delete("things", &thing("a", "Rent", 100)).unwrap()],
    );
    db.write(delete, Source::App).unwrap();
    let journal_before = journal_rows(&db);

    let err = db.undo().unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    assert_eq!(things(&db), vec![]);
    assert_eq!(journal_rows(&db), journal_before);
}

#[test]
fn a_write_still_succeeds_when_the_journal_file_cannot_be_appended_and_the_next_write_catches_up() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, db) = open();
    let path = dir.path().join(journal::FILE_NAME);
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();

    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    assert_eq!(things(&db), vec![row("a", "Flat", 100)]);
    let behind = db.read(|conn| journal::compare(conn, &path)).unwrap();
    assert_eq!(behind.rows, 5);
    assert_eq!(behind.behind(), 1);
    assert_eq!(journal_lines(dir.path()).len(), 4);

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    let mirror = db.read(|conn| journal::compare(conn, &path)).unwrap();
    assert_eq!(
        mirror,
        journal::Mirror {
            rows: 9,
            matching: 9,
            edited: vec![]
        }
    );
    assert_eq!(journal_lines(dir.path()).len(), 9);
}

#[test]
fn null_fields_mirrored_to_the_journal_file_compare_as_matching() {
    let (dir, db) = open();
    let path = dir.path().join(journal::FILE_NAME);
    let without_name = json!({ "id": "a", "name": null, "cents": null, "done": false });
    let with_name = json!({ "id": "a", "name": "x", "cents": null, "done": false });
    db.write(
        Op::new(
            "Add",
            vec![Change::insert("things", &without_name).unwrap()],
        ),
        Source::Ui,
    )
    .unwrap();
    db.write(
        Op::new(
            "Name it",
            vec![Change::update("things", &without_name, &with_name).unwrap()],
        ),
        Source::Ui,
    )
    .unwrap();
    db.write(
        Op::new(
            "Clear name",
            vec![Change::update("things", &with_name, &without_name).unwrap()],
        ),
        Source::Ui,
    )
    .unwrap();

    let mirror = db.read(|conn| journal::compare(conn, &path)).unwrap();

    assert_eq!(mirror.matching, mirror.rows);
    assert_eq!(mirror.edited, Vec::<i64>::new());
    assert_eq!(mirror.behind(), 0);
    assert!(mirror.rows > 0);
}

#[test]
fn imports_and_cli_writes_are_undoable() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Import).unwrap();
    db.write(insert("b", "Gym", 200), Source::Cli).unwrap();
    db.undo().unwrap();
    db.undo().unwrap();
    assert_eq!(things(&db), vec![]);
}

#[test]
fn seed_writes_are_not_undoable() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Seed).unwrap();
    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(things(&db), vec![row("a", "Rent", 100)]);
}

#[test]
fn app_writes_are_journaled_but_not_undoable() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::App).unwrap();
    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(things(&db), vec![row("a", "Rent", 100)]);
    let sources: Vec<String> = db
        .read(|conn| {
            let mut query = conn.prepare("SELECT DISTINCT source FROM journal")?;
            let rows = query.query_map([], |r| r.get(0))?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap();
    assert_eq!(sources, vec!["app".to_string()]);
}

#[test]
fn undo_and_redo_are_journaled_as_new_ops() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.undo().unwrap();
    db.redo().unwrap();
    let ops: Vec<(i64, String)> = db
        .read(|conn| {
            let mut query = conn.prepare("SELECT DISTINCT op, source FROM journal ORDER BY op")?;
            let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap();
    assert_eq!(
        ops,
        vec![
            (1, "ui".to_string()),
            (2, "undo".to_string()),
            (3, "redo".to_string())
        ]
    );
}

#[test]
fn an_op_followed_by_its_inverse_leaves_the_table_unchanged() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    let before = things(&db);

    let op = Op::new(
        "Reshuffle",
        vec![
            Change::update("things", &thing("a", "Rent", 100), &thing("a", "Flat", 150)).unwrap(),
            Change::delete("things", &thing("b", "Gym", 200)).unwrap(),
            Change::insert("things", &thing("c", "Bike", 300)).unwrap(),
        ],
    );
    db.write(op.clone(), Source::Ui).unwrap();
    assert_eq!(
        things(&db),
        vec![row("a", "Flat", 150), row("c", "Bike", 300)]
    );

    db.write(op.inverse(), Source::Seed).unwrap();
    assert_eq!(things(&db), before);
}

#[test]
fn the_journal_has_one_row_per_field_sharing_an_increasing_op_number() {
    let (_dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Cli).unwrap();
    let entries = db.read(|conn| journal::entries_after(conn, 0)).unwrap();
    let summary: Vec<(i64, &str, &str, &str)> = entries
        .iter()
        .map(|e| (e.op, e.source.as_str(), e.label.as_str(), e.field.as_str()))
        .collect();
    assert_eq!(
        summary,
        vec![
            (1, "ui", "Add Rent", "id"),
            (1, "ui", "Add Rent", "name"),
            (1, "ui", "Add Rent", "cents"),
            (1, "ui", "Add Rent", "done"),
            (2, "cli", "Rename Rent", "name"),
        ]
    );
    assert_eq!(entries[4].old, Some(json!("Rent")));
    assert_eq!(entries[4].new, Some(json!("Flat")));
    assert!(
        entries
            .iter()
            .all(|e| e.at > 0 && e.entity == "things" && e.id == "a")
    );
}

#[test]
fn a_write_appends_its_rows_to_the_journal_file_as_json_lines() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    assert_eq!(lines.len(), 5);

    let mut last: serde_json::Value = serde_json::from_str(&lines[4]).unwrap();
    assert!(last["at"].as_i64().unwrap() > 0);
    last.as_object_mut().unwrap().remove("at");
    assert_eq!(
        last,
        json!({
            "seq": 5, "op": 2, "label": "Rename Rent", "source": "ui",
            "entity": "things", "id": "a", "field": "name", "old": "Rent", "new": "Flat"
        })
    );
}

#[test]
fn the_journal_file_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let mode = std::fs::metadata(dir.path().join(journal::FILE_NAME))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn reopening_without_the_journal_file_restores_every_line() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    drop(db);

    std::fs::remove_file(dir.path().join(journal::FILE_NAME)).unwrap();
    Db::open(dir.path()).unwrap();
    assert_eq!(journal_lines(dir.path()), lines);
}

#[test]
fn reopening_with_a_shorter_journal_file_appends_only_the_missing_lines() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    drop(db);

    let kept = format!("{}\n{}\n", lines[0], lines[1]);
    std::fs::write(dir.path().join(journal::FILE_NAME), &kept).unwrap();
    Db::open(dir.path()).unwrap();
    assert_eq!(journal_lines(dir.path()), lines);
}

#[test]
fn reopening_with_a_journal_file_that_is_ahead_keeps_its_content() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    drop(db);

    let path = dir.path().join(journal::FILE_NAME);
    let ahead = format!(
        "{}\n{}\n",
        std::fs::read_to_string(&path).unwrap().trim_end(),
        r#"{"seq":99,"op":9,"at":1,"label":"Later","source":"ui","entity":"things","id":"z","field":"id","old":null,"new":"z"}"#
    );
    std::fs::write(&path, &ahead).unwrap();
    Db::open(dir.path()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), ahead);
}

fn assert_journal_fully_mirrored(db: &Db, dir: &Path) {
    let path = dir.join(journal::FILE_NAME);
    let rows = db.read(|conn| journal::entries_after(conn, 0)).unwrap();
    let seqs: Vec<i64> = rows.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=rows.len() as i64).collect::<Vec<_>>());
    let mirror = db.read(|conn| journal::compare(conn, &path)).unwrap();
    assert_eq!(
        mirror,
        journal::Mirror {
            rows: rows.len(),
            matching: rows.len(),
            edited: vec![]
        }
    );
    assert_eq!(journal_lines(dir).len(), rows.len());
}

#[test]
fn writes_undo_and_redo_leave_every_journal_row_in_the_file_exactly_once_in_order() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    db.undo().unwrap().unwrap();
    db.redo().unwrap().unwrap();
    db.undo().unwrap().unwrap();

    assert_journal_fully_mirrored(&db, dir.path());
}

#[test]
fn reopening_and_writing_more_keeps_every_journal_row_in_the_file_exactly_once() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    drop(db);

    let db = Db::open(dir.path()).unwrap();
    db.write(insert("b", "Gym", 200), Source::Ui).unwrap();
    db.write(rename("b", "Gym", "Club"), Source::Ui).unwrap();

    assert_eq!(journal_rows(&db), 10);
    assert_journal_fully_mirrored(&db, dir.path());
}

#[test]
fn a_write_after_the_file_was_removed_appends_only_its_own_rows() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    std::fs::remove_file(dir.path().join(journal::FILE_NAME)).unwrap();

    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();

    let after = journal_lines(dir.path());
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].split(',').next(), Some(r#"{"seq":5"#));
    assert!(!lines.contains(&after[0]));
}

#[test]
fn a_write_after_the_file_was_truncated_does_not_restore_the_lost_lines() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    let path = dir.path().join(journal::FILE_NAME);
    std::fs::write(&path, format!("{}\n{}\n", lines[0], lines[1])).unwrap();

    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();

    let after = journal_lines(dir.path());
    assert_eq!(after.len(), 3);
    assert_eq!(after[..2], lines[..2]);
    let mirror = db.read(|conn| journal::compare(conn, &path)).unwrap();
    assert_eq!(mirror.matching, 3);
    assert_eq!(mirror.edited, vec![3, 4]);
}

#[test]
fn reopening_after_the_file_was_removed_or_truncated_while_open_catches_up_the_whole_file() {
    let (dir, db) = open();
    db.write(insert("a", "Rent", 100), Source::Ui).unwrap();
    db.write(rename("a", "Rent", "Flat"), Source::Ui).unwrap();
    let lines = journal_lines(dir.path());
    let path = dir.path().join(journal::FILE_NAME);

    std::fs::remove_file(&path).unwrap();
    drop(db);
    let db = Db::open(dir.path()).unwrap();
    assert_eq!(journal_lines(dir.path()), lines);

    std::fs::write(&path, format!("{}\n", lines[0])).unwrap();
    drop(db);
    let db = Db::open(dir.path()).unwrap();
    assert_eq!(journal_lines(dir.path()), lines);
    assert_journal_fully_mirrored(&db, dir.path());
}

#[test]
fn a_file_from_a_newer_hamster_is_refused_when_opening() {
    let dir = temp_config_dir();
    {
        let conn = Connection::open(dir.path().join(FILE_NAME)).unwrap();
        conn.pragma_update(
            None,
            "user_version",
            migrations::MIGRATIONS.len() as i64 + 1,
        )
        .unwrap();
    }
    let err = Db::open(dir.path()).err().unwrap();
    assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
}
