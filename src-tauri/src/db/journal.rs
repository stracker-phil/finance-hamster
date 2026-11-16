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
pub fn last_mirrored(path: &Path) -> AppResult<i64> {
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

/// How `journal.jsonl` compares with the `journal` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mirror {
    /// Rows in the table.
    pub rows: usize,
    /// Rows the file holds exactly as the table does.
    pub matching: usize,
    /// Rows whose line says something else, or that the file lacks before its last line: the file was edited.
    pub edited: Vec<i64>,
}

impl Mirror {
    /// Rows still to append after the file's last line; the next write or launch adds them.
    pub fn behind(&self) -> usize {
        self.rows - self.matching - self.edited.len()
    }
}

/// Compares every row of the table with its line in the file.
pub fn compare(conn: &Connection, path: &Path) -> AppResult<Mirror> {
    // Compared as JSON: a field set to `null` reads back from the table as `Some(Null)` but from the file as `None`.
    let mut lines = std::collections::HashMap::new();
    if let Ok(file) = std::fs::File::open(path) {
        for line in BufReader::new(file).lines() {
            if let Ok(value) = serde_json::from_str::<Value>(&line?)
                && let Some(seq) = value.get("seq").and_then(Value::as_i64)
            {
                lines.insert(seq, value);
            }
        }
    }
    let last = lines.keys().copied().max().unwrap_or(0);
    let rows = entries_after(conn, 0)?;
    let mut matching = 0;
    let mut edited = Vec::new();
    for row in &rows {
        match lines.get(&row.seq) {
            Some(line) if *line == serde_json::to_value(row)? => matching += 1,
            None if row.seq > last => {}
            _ => edited.push(row.seq),
        }
    }
    Ok(Mirror {
        rows: rows.len(),
        matching,
        edited,
    })
}

/// Appends every row the file lacks and syncs it; returns the file's new last seq.
pub fn catch_up(conn: &Connection, path: &Path) -> AppResult<i64> {
    append_after(conn, path, last_mirrored(path)?)
}

/// Appends the rows after `seq` without reading the file; returns the new last seq.
pub fn append_after(conn: &Connection, path: &Path, seq: i64) -> AppResult<i64> {
    let missing = entries_after(conn, seq)?;
    append(path, &missing)?;
    Ok(missing.last().map_or(seq, |entry| entry.seq))
}

fn append(path: &Path, missing: &[Entry]) -> AppResult<()> {
    if missing.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    // A torn last line stays as it is; the next entry starts on a line of its own.
    let mut text = if ends_mid_line(path)? {
        "\n".to_string()
    } else {
        String::new()
    };
    for entry in missing {
        text.push_str(&serde_json::to_string(entry)?);
        text.push('\n');
    }
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
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

        assert_eq!(catch_up(&conn, &path).unwrap(), 3);

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
    fn catch_up_appends_only_the_missing_rows_and_returns_the_files_last_seq() {
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
        assert_eq!(catch_up(&conn, &path).unwrap(), 3);
        let all = std::fs::read_to_string(&path).unwrap();
        assert!(all.starts_with(&first));
        assert_eq!(all.lines().count(), 3);

        assert_eq!(catch_up(&conn, &path).unwrap(), 3);
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

    fn three_rows_with_lines() -> (tempfile::TempDir, Connection, Vec<String>) {
        let dir = temp_config_dir();
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
        let lines = entries_after(&conn, 0)
            .unwrap()
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        (dir, conn, lines)
    }

    #[test]
    fn a_file_that_mirrors_every_row_compares_clean() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
        let mirror = compare(&conn, &path).unwrap();
        assert_eq!(
            mirror,
            Mirror {
                rows: 3,
                matching: 3,
                edited: vec![]
            }
        );
        assert_eq!(mirror.behind(), 0);
    }

    #[test]
    fn a_missing_file_leaves_every_row_behind_and_nothing_edited() {
        let (dir, conn, _) = three_rows_with_lines();
        let mirror = compare(&conn, &dir.path().join(FILE_NAME)).unwrap();
        assert_eq!(
            mirror,
            Mirror {
                rows: 3,
                matching: 0,
                edited: vec![]
            }
        );
        assert_eq!(mirror.behind(), 3);
    }

    #[test]
    fn rows_after_the_files_last_line_are_behind_not_edited() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, format!("{}\n", lines[0])).unwrap();
        let mirror = compare(&conn, &path).unwrap();
        assert_eq!(mirror.matching, 1);
        assert!(mirror.edited.is_empty());
        assert_eq!(mirror.behind(), 2);
    }

    #[test]
    fn a_line_whose_content_differs_from_its_row_is_edited() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        let changed = lines[1].replace("\"new\":2", "\"new\":20");
        assert_ne!(changed, lines[1]);
        std::fs::write(&path, format!("{}\n{}\n{}\n", lines[0], changed, lines[2])).unwrap();
        let mirror = compare(&conn, &path).unwrap();
        assert_eq!(
            mirror,
            Mirror {
                rows: 3,
                matching: 2,
                edited: vec![2]
            }
        );
        assert_eq!(mirror.behind(), 0);
    }

    #[test]
    fn a_row_missing_before_the_files_last_line_is_edited() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, format!("{}\n{}\n", lines[0], lines[2])).unwrap();
        let mirror = compare(&conn, &path).unwrap();
        assert_eq!(
            mirror,
            Mirror {
                rows: 3,
                matching: 2,
                edited: vec![2]
            }
        );
    }

    #[test]
    fn a_torn_last_line_is_skipped_and_its_row_counts_as_behind() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(
            &path,
            format!("{}\n{}\n{{\"seq\":3,\"op\"", lines[0], lines[1]),
        )
        .unwrap();
        let mirror = compare(&conn, &path).unwrap();
        assert_eq!(
            mirror,
            Mirror {
                rows: 3,
                matching: 2,
                edited: vec![]
            }
        );
        assert_eq!(mirror.behind(), 1);
    }

    fn parsed_lines(path: &Path) -> Vec<Entry> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn append_after_appends_exactly_the_rows_after_the_given_seq_and_returns_the_last() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, format!("{}\n", lines[0])).unwrap();

        assert_eq!(append_after(&conn, &path, 1).unwrap(), 3);

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("{}\n", lines.join("\n"))
        );
    }

    #[test]
    fn append_after_with_every_row_mirrored_leaves_the_file_unchanged_and_returns_the_seq() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        let content = format!("{}\n", lines.join("\n"));
        std::fs::write(&path, &content).unwrap();

        assert_eq!(append_after(&conn, &path, 3).unwrap(), 3);

        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn append_after_with_nothing_to_append_creates_no_file() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        assert_eq!(append_after(&connection(), &path, 0).unwrap(), 0);
        assert!(!path.exists());
    }

    #[test]
    fn append_after_appends_from_the_given_seq_even_when_the_file_disagrees() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, format!("{}\n", lines[0])).unwrap();

        assert_eq!(append_after(&conn, &path, 0).unwrap(), 3);

        let seqs: Vec<i64> = parsed_lines(&path).iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![1, 1, 2, 3]);
    }

    #[test]
    fn append_after_a_torn_last_line_keeps_it_and_starts_new_entries_on_their_own_lines() {
        let (dir, conn, lines) = three_rows_with_lines();
        let path = dir.path().join(FILE_NAME);
        let torn = r#"{"seq":2,"op""#;
        std::fs::write(&path, format!("{}\n{torn}", lines[0])).unwrap();

        assert_eq!(append_after(&conn, &path, 1).unwrap(), 3);

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text.lines().collect::<Vec<_>>(),
            vec![
                lines[0].as_str(),
                torn,
                lines[1].as_str(),
                lines[2].as_str()
            ]
        );
        assert!(text.ends_with('\n'));
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
