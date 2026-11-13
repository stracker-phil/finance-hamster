//! The `documents`, `vendors` and `document_links` rows.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

pub const DOCUMENTS: &str = "documents";
pub const VENDORS: &str = "vendors";
pub const LINKS: &str = "document_links";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum DocState {
    /// Editable; counts, flagged as unfinished.
    Draft,
    /// Locked.
    Finished,
    /// Out of every total and list; shown only in its year's Archive.
    Excluded,
}

impl DocState {
    fn parse(text: &str) -> DocState {
        match text {
            "finished" => DocState::Finished,
            "excluded" => DocState::Excluded,
            _ => DocState::Draft,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DocumentRow {
    pub id: String,
    pub state: DocState,
    /// Relative to the data folder: `docs/2026/2026-11-10-pixelforge.pdf` or `docs/inbox/<original name>`.
    pub file: String,
    pub original_name: String,
    pub file_hash: String,
    pub vendor_id: Option<String>,
    /// `YYYY-MM-DD`; sets the tax year, the folder and the items.
    pub pay_date: Option<String>,
    /// Typed only while no bank line is linked; linked lines are the amount otherwise.
    pub paid_cents: Option<i64>,
    pub item_key: Option<String>,
    /// Percent of the payment that is for work.
    pub work_share: i64,
    pub note: String,
    pub invoice_cents: Option<i64>,
    pub invoice_currency: Option<String>,
    pub invoice_number: Option<String>,
    pub invoice_date: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    pub excluded_at: Option<i64>,
    pub exclude_reason: Option<String>,
    /// Years to depreciate over, overriding the item's asset class.
    #[serde(default)]
    pub useful_life: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VendorRow {
    pub id: String,
    pub name: String,
    /// The item a new document from this vendor starts with; its last document's item otherwise.
    pub default_item_key: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LinkRow {
    pub id: String,
    pub document_id: String,
    pub transaction_id: String,
    pub created_at: i64,
}

const DOCUMENT_COLUMNS: &str = "id, state, file, original_name, file_hash, vendor_id, pay_date, paid_cents, \
     item_key, work_share, note, invoice_cents, invoice_currency, invoice_number, invoice_date, created_at, \
     finished_at, excluded_at, exclude_reason, useful_life";

fn document(row: &rusqlite::Row) -> rusqlite::Result<DocumentRow> {
    let state: String = row.get(1)?;
    Ok(DocumentRow {
        id: row.get(0)?,
        state: DocState::parse(&state),
        file: row.get(2)?,
        original_name: row.get(3)?,
        file_hash: row.get(4)?,
        vendor_id: row.get(5)?,
        pay_date: row.get(6)?,
        paid_cents: row.get(7)?,
        item_key: row.get(8)?,
        work_share: row.get(9)?,
        note: row.get(10)?,
        invoice_cents: row.get(11)?,
        invoice_currency: row.get(12)?,
        invoice_number: row.get(13)?,
        invoice_date: row.get(14)?,
        created_at: row.get(15)?,
        finished_at: row.get(16)?,
        excluded_at: row.get(17)?,
        exclude_reason: row.get(18)?,
        useful_life: row.get(19)?,
    })
}

/// Every document, oldest first.
pub fn documents(conn: &Connection) -> AppResult<Vec<DocumentRow>> {
    let sql = format!("SELECT {DOCUMENT_COLUMNS} FROM {DOCUMENTS} ORDER BY created_at, id");
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], document)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn document_by_id(conn: &Connection, id: &str) -> AppResult<Option<DocumentRow>> {
    let sql = format!("SELECT {DOCUMENT_COLUMNS} FROM {DOCUMENTS} WHERE id = ?");
    Ok(conn.query_row(&sql, [id], document).optional()?)
}

pub fn document_by_hash(conn: &Connection, hash: &str) -> AppResult<Option<DocumentRow>> {
    let sql = format!("SELECT {DOCUMENT_COLUMNS} FROM {DOCUMENTS} WHERE file_hash = ?");
    Ok(conn.query_row(&sql, [hash], document).optional()?)
}

fn vendor(row: &rusqlite::Row) -> rusqlite::Result<VendorRow> {
    Ok(VendorRow {
        id: row.get(0)?,
        name: row.get(1)?,
        default_item_key: row.get(2)?,
        created_at: row.get(3)?,
    })
}

/// Every vendor by name, ignoring case.
pub fn vendors(conn: &Connection) -> AppResult<Vec<VendorRow>> {
    let sql = format!(
        "SELECT id, name, default_item_key, created_at FROM {VENDORS} ORDER BY name COLLATE NOCASE"
    );
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], vendor)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn link(row: &rusqlite::Row) -> rusqlite::Result<LinkRow> {
    Ok(LinkRow {
        id: row.get(0)?,
        document_id: row.get(1)?,
        transaction_id: row.get(2)?,
        created_at: row.get(3)?,
    })
}

/// Every link, oldest first.
pub fn links(conn: &Connection) -> AppResult<Vec<LinkRow>> {
    let sql = format!(
        "SELECT id, document_id, transaction_id, created_at FROM {LINKS} ORDER BY created_at, id"
    );
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], link)?;
    Ok(rows.collect::<Result<_, _>>()?)
}
