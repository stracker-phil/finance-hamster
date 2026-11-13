//! Documents: an invoice PDF plus a few facts. The payment date sets the tax year, the file's folder and the
//! items to choose from; linked bank lines are the amount paid.

pub mod add;
pub mod amounts;
pub mod commands;
pub mod edit;
pub mod files;
pub mod guess;
pub mod list;
pub mod payments;
pub mod store;
pub mod vendors;
pub mod view;

#[cfg(test)]
mod tests;

use std::path::Path;

use store::{DocumentRow, VendorRow};

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
