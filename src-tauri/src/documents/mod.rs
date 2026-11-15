//! Documents: an invoice PDF plus a few facts. The payment date sets the tax year, the file's folder and the
//! items to choose from; linked bank lines are the amount paid.

pub mod add;
pub mod amounts;
pub mod commands;
pub mod depreciation;
pub mod edit;
pub mod files;
pub mod guess;
pub mod history;
pub mod lifecycle;
pub mod list;
pub mod payments;
pub mod store;
pub mod vendors;
pub mod view;

#[cfg(test)]
mod tests;

use std::path::Path;

use store::{DocumentRow, VendorRow};

use crate::db::Db;
use crate::error::{AppError, AppResult};

/// Refuses linking `line` to `doc_id` while another document that counts already has it: the year would count
/// the line's amount twice.
pub fn refuse_paid_elsewhere(db: &Db, line: &str, doc_id: &str) -> AppResult<()> {
    let others: Vec<String> = db
        .read(store::links)?
        .into_iter()
        .filter(|l| l.transaction_id == line && l.document_id != doc_id)
        .map(|l| l.document_id)
        .collect();
    if others.is_empty() {
        return Ok(());
    }
    let vendors = db.read(store::vendors)?;
    let holder = db
        .read(store::documents)?
        .into_iter()
        .find(|d| others.contains(&d.id) && d.state != store::DocState::Excluded);
    match holder {
        Some(doc) => Err(AppError::InvalidField {
            field: "document.lines".into(),
            message: format!(
                "This line already pays {}; unlink it there first",
                title_of(&doc, &vendors)
            ),
        }),
        None => Ok(()),
    }
}

/// `Pixelforge Ltd. invoice`, or the file's name while no vendor is set.
pub fn title_of(doc: &DocumentRow, vendors: &[VendorRow]) -> String {
    doc.vendor_id
        .as_ref()
        .and_then(|id| vendors.iter().find(|v| &v.id == id))
        .map_or_else(
            || {
                Path::new(&doc.original_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&doc.original_name)
                    .to_string()
            },
            |v| format!("{} invoice", v.name),
        )
}
