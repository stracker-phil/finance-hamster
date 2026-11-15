//! Other documents (ADR 0056): any PDF the owner wants to keep (a contract, a policy, a tax assessment), with a
//! title, an optional date and a note. Nothing reads them and they count in no tax year.

pub mod commands;
pub mod edit;
pub mod view;

#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

pub const OTHERS: &str = "others";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OtherRow {
    pub id: String,
    pub title: String,
    /// `YYYY-MM-DD`; None when the owner gave none.
    pub date: Option<String>,
    pub note: String,
    /// Under the data folder.
    pub file: String,
    pub file_hash: String,
    pub created_at: i64,
}

const COLUMNS: &str = "id, title, date, note, file, file_hash, created_at";

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<OtherRow> {
    Ok(OtherRow {
        id: r.get(0)?,
        title: r.get(1)?,
        date: r.get(2)?,
        note: r.get(3)?,
        file: r.get(4)?,
        file_hash: r.get(5)?,
        created_at: r.get(6)?,
    })
}

/// Every other document, newest date first, those without a date last.
pub fn others(conn: &Connection) -> AppResult<Vec<OtherRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM {OTHERS} ORDER BY date IS NULL, date DESC, created_at DESC"
    ))?;
    let rows = query.query_map([], read_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn other(conn: &Connection, id: &str) -> AppResult<Option<OtherRow>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM {OTHERS} WHERE id = ?"),
            [id],
            read_row,
        )
        .optional()?)
}

pub fn by_hash(conn: &Connection, hash: &str) -> AppResult<Option<OtherRow>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM {OTHERS} WHERE file_hash = ?"),
            [hash],
            read_row,
        )
        .optional()?)
}
