//! The journal: one row per changed field in the `journal` table, mirrored line by line to `journal.jsonl`,
//! which is only ever appended to (ADR 0007).

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::change::FieldChange;
use crate::error::AppResult;

pub const FILE_NAME: &str = "journal.jsonl";

/// One journal row, as the table holds it and the file shows it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub seq: i64,
    /// The op it belongs to; every field an op changes shares it.
    pub op: i64,
    /// UTC milliseconds.
    pub at: i64,
    pub label: String,
    pub source: String,
    pub entity: String,
    pub id: String,
    pub field: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

/// What the op is written as: its label, where it came from and when.
pub struct OpHeader<'a> {
    pub label: &'a str,
    pub source: &'a str,
    pub at: i64,
}

/// Records every field of one op inside its transaction.
pub fn record(tx: &Transaction, header: &OpHeader, fields: &[FieldChange]) -> AppResult<()> {
    let op: i64 = tx.query_row("SELECT COALESCE(MAX(op), 0) + 1 FROM journal", [], |row| {
        row.get(0)
    })?;
    let mut insert = tx.prepare_cached(
        "INSERT INTO journal (op, at, label, source, entity, entity_id, field, old, new)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;
    for field in fields {
        insert.execute(rusqlite::params![
            op,
            header.at,
            header.label,
            header.source,
            field.entity,
            field.entity_id,
            field.field,
            field.old.as_ref().map(Value::to_string),
            field.new.as_ref().map(Value::to_string),
        ])?;
    }
    Ok(())
}

pub fn entries_after(conn: &Connection, seq: i64) -> AppResult<Vec<Entry>> {
    let mut query = conn.prepare(
        "SELECT seq, op, at, label, source, entity, entity_id, field, old, new FROM journal WHERE seq > ? ORDER BY seq",
    )?;
    let parse = |text: Option<String>| text.and_then(|t| serde_json::from_str(&t).ok());
    let rows = query.query_map([seq], |row| {
        Ok(Entry {
            seq: row.get(0)?,
            op: row.get(1)?,
            at: row.get(2)?,
            label: row.get(3)?,
            source: row.get(4)?,
            entity: row.get(5)?,
            id: row.get(6)?,
            field: row.get(7)?,
            old: parse(row.get(8)?),
            new: parse(row.get(9)?),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The last `seq` the file holds; 0 for a missing or empty file. Lines that do not parse, such as a torn last line, are skipped.
fn last_mirrored(path: &Path) -> AppResult<i64> {
    let Ok(file) = std::fs::File::open(path) else {
        return Ok(0);
    };
    let mut last = 0;
    for line in BufReader::new(file).lines() {
        if let Ok(entry) = serde_json::from_str::<Entry>(&line?) {
            last = last.max(entry.seq);
        }
    }
    Ok(last)
}

fn ends_mid_line(path: &Path) -> AppResult<bool> {
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() == 0 {
        return Ok(false);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0u8];
    file.read_exact(&mut last)?;
    Ok(last[0] != b'\n')
}

/// Appends every row the file lacks and syncs it.
pub fn catch_up(conn: &Connection, path: &Path) -> AppResult<usize> {
    let missing = entries_after(conn, last_mirrored(path)?)?;
    if missing.is_empty() {
        return Ok(0);
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    // A torn last line stays as it is; the next entry starts on a line of its own.
    let mut text = if ends_mid_line(path)? {
        "\n".to_string()
    } else {
        String::new()
    };
    for entry in &missing {
        text.push_str(&serde_json::to_string(entry)?);
        text.push('\n');
    }
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(missing.len())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::db::migrations;
    use crate::test_support::temp_config_dir;

    fn connection() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        conn
    }

    fn field(name: &str, old: Option<Value>, new: Option<Value>) -> FieldChange {
        FieldChange {
            entity: "things".into(),
            entity_id: "a".into(),
            field: name.into(),
            old,
            new,
        }
    }

    fn record_op(conn: &mut Connection, label: &str, fields: &[FieldChange]) {
        let tx = conn.transaction().unwrap();
        let header = OpHeader {
            label,
            source: "ui",
            at: 1_700_000_000_000,
        };
        record(&tx, &header, fields).unwrap();
        tx.commit().unwrap();
    }

    #[test]
    fn record_writes_one_row_per_field_sharing_one_op_number_that_grows_per_op() {
        let mut conn = connection();
        record_op(
            &mut conn,
            "Add Rent",
            &[
                field("name", None, Some(json!("Rent"))),
                field("cents", None, Some(json!(100))),
            ],
        );
        record_op(
            &mut conn,
            "Rename",
            &[field("name", Some(json!("Rent")), Some(json!("Flat")))],
        );

        let entries = entries_after(&conn, 0).unwrap();
        assert_eq!(
            entries,
            vec![
                Entry {
                    seq: 1,
                    op: 1,
                    at: 1_700_000_000_000,
                    label: "Add Rent".into(),
                    source: "ui".into(),
                    entity: "things".into(),
                    id: "a".into(),
                    field: "name".into(),
                    old: None,
                    new: Some(json!("Rent")),
                },
                Entry {
                    seq: 2,
                    op: 1,
                    at: 1_700_000_000_000,
                    label: "Add Rent".into(),
                    source: "ui".into(),
                    entity: "things".into(),
                    id: "a".into(),
                    field: "cents".into(),
                    old: None,
                    new: Some(json!(100)),
                },
                Entry {
                    seq: 3,
                    op: 2,
                    at: 1_700_000_000_000,
                    label: "Rename".into(),
                    source: "ui".into(),
                    entity: "things".into(),
                    id: "a".into(),
                    field: "name".into(),
                    old: Some(json!("Rent")),
                    new: Some(json!("Flat")),
                },
            ]
        );
    }

    #[test]
    fn entries_after_returns_only_later_rows_in_order() {
        let mut conn = connection();
        record_op(
            &mut conn,
            "Add",
            &[
                field("a", None, Some(json!(1))),
                field("b", None, Some(json!(2))),
                field("c", None, Some(json!(3))),
            ],
        );
        let seqs: Vec<i64> = entries_after(&conn, 1)
            .unwrap()
            .iter()
            .map(|e| e.seq)
            .collect();
        assert_eq!(seqs, vec![2, 3]);
        assert_eq!(entries_after(&conn, 3).unwrap(), vec![]);
    }

    #[test]
    fn last_mirrored_is_zero_for_a_missing_file() {
        let dir = temp_config_dir();
        assert_eq!(last_mirrored(&dir.path().join(FILE_NAME)).unwrap(), 0);
    }

    #[test]
    fn last_mirrored_is_the_highest_seq_and_ignores_a_torn_last_line() {
        let dir = temp_config_dir();
        let mut conn = connection();
        record_op(
            &mut conn,
            "Add",
            &[
                field("a", None, Some(json!(1))),
                field("b", None, Some(json!(2))),
            ],
        );
        let path = dir.path().join(FILE_NAME);
        let complete = serde_json::to_string(&entries_after(&conn, 0).unwrap()[0]).unwrap();
        std::fs::write(&path, format!("{complete}\n{{\"seq\":2,\"op\"")).unwrap();
        assert_eq!(last_mirrored(&path).unwrap(), 1);
    }

    #[test]
    fn catch_up_after_a_torn_last_line_keeps_it_and_starts_new_entries_on_their_own_lines() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        let mut conn = connection();
        record_op(
            &mut conn,
            "Add",
            &[
                field("a", None, Some(json!(1))),
                field("b", None, Some(json!(2))),
                field("c", None, Some(json!(3))),
            ],
        );
        let entries = entries_after(&conn, 0).unwrap();
        let first = serde_json::to_string(&entries[0]).unwrap();
        let torn = r#"{"seq":2,"op""#;
        std::fs::write(&path, format!("{first}\n{torn}")).unwrap();

        assert_eq!(catch_up(&conn, &path).unwrap(), 2);

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], first);
        assert_eq!(lines[1], torn);
        let appended: Vec<Entry> = lines[2..]
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(appended, entries[1..].to_vec());
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn catch_up_appends_only_the_missing_rows() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        let mut conn = connection();
        record_op(&mut conn, "Add", &[field("a", None, Some(json!(1)))]);
        assert_eq!(catch_up(&conn, &path).unwrap(), 1);
        let first = std::fs::read_to_string(&path).unwrap();

        record_op(
            &mut conn,
            "Add more",
            &[
                field("b", None, Some(json!(2))),
                field("c", None, Some(json!(3))),
            ],
        );
        assert_eq!(catch_up(&conn, &path).unwrap(), 2);
        let all = std::fs::read_to_string(&path).unwrap();
        assert!(all.starts_with(&first));
        assert_eq!(all.lines().count(), 3);

        assert_eq!(catch_up(&conn, &path).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), all);
    }

    #[test]
    fn catch_up_writes_one_entry_per_line_that_reads_back_unchanged() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        let mut conn = connection();
        record_op(
            &mut conn,
            "Add",
            &[field("a", None, Some(json!({ "nested": [1] })))],
        );
        catch_up(&conn, &path).unwrap();
        let read: Vec<Entry> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(read, entries_after(&conn, 0).unwrap());
    }

    #[test]
    fn nothing_to_mirror_creates_no_file() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        assert_eq!(catch_up(&connection(), &path).unwrap(), 0);
        assert!(!path.exists());
    }

    #[test]
    fn the_mirror_file_is_private_to_the_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        let mut conn = connection();
        record_op(&mut conn, "Add", &[field("a", None, Some(json!(1)))]);
        catch_up(&conn, &path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
