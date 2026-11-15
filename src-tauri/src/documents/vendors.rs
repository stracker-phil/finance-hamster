//! Settings → Vendors: each vendor with its documents and item; set a default item, rename, merge into another,
//! delete one no document uses.

use serde::Serialize;
use ts_rs::TS;

use super::store::{self, VendorRow};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::rules::{self, ItemOption};
use crate::{tr, tr_plural};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct VendorView {
    pub id: String,
    pub name: String,
    /// `2 documents · KZ 169`.
    pub detail: String,
    pub default_item_key: Option<String>,
    /// No document uses it.
    pub unused: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct VendorsView {
    pub vendors: Vec<VendorView>,
    pub items: Vec<ItemOption>,
}

pub fn view(db: &Db) -> AppResult<VendorsView> {
    let documents = db.read(store::documents)?;
    let items = rules::items(db)?;
    let vendors = db
        .read(store::vendors)?
        .into_iter()
        .map(|v| {
            let own: Vec<_> = documents
                .iter()
                .filter(|d| d.vendor_id.as_deref() == Some(v.id.as_str()))
                .collect();
            let item = v
                .default_item_key
                .clone()
                .or_else(|| own.iter().rev().find_map(|d| d.item_key.clone()))
                .map(|key| {
                    items
                        .iter()
                        .find(|i| i.key == key)
                        .map_or(key, |i| format!("KZ {}", i.kz))
                });
            let n = own.len();
            let count = tr_plural!(
                n,
                "{n} document",
                "{n} documents",
                "{n} Beleg",
                "{n} Belege"
            );
            VendorView {
                detail: std::iter::once(count)
                    .chain(item)
                    .collect::<Vec<_>>()
                    .join(" · "),
                unused: own.is_empty(),
                id: v.id,
                name: v.name,
                default_item_key: v.default_item_key,
            }
        })
        .collect();
    Ok(VendorsView { vendors, items })
}

fn vendor(db: &Db, id: &str) -> AppResult<VendorRow> {
    db.read(store::vendors)?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| AppError::NotFound(tr!("No vendor {id}", "Kein Händler {id}")))
}

fn invalid(message: String) -> AppError {
    AppError::InvalidField {
        field: "vendor.name".into(),
        message,
    }
}

/// Renames a vendor; drafts' files follow on the next relocate, finished ones keep their names.
pub fn rename(db: &Db, id: &str, name: &str, source: Source) -> AppResult<Written> {
    let before = vendor(db, id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid(tr!(
            "The name can't be empty",
            "Der Name darf nicht leer sein"
        )));
    }
    if db
        .read(store::vendors)?
        .iter()
        .any(|v| v.id != id && v.name.eq_ignore_ascii_case(name))
    {
        return Err(invalid(tr!(
            "Another vendor has this name; merge them instead",
            "Ein anderer Händler hat diesen Namen; führe sie stattdessen zusammen"
        )));
    }
    let after = VendorRow {
        name: name.to_string(),
        ..before.clone()
    };
    let op = Op::new(
        tr!("Rename Vendor {}", "Händler umbenennen: {}", before.name),
        vec![Change::update(store::VENDORS, &before, &after)?],
    );
    db.write(op, source)
}

pub fn set_default_item(
    db: &Db,
    id: &str,
    item_key: Option<String>,
    source: Source,
) -> AppResult<Written> {
    let before = vendor(db, id)?;
    if let Some(key) = &item_key
        && !rules::items(db)?.iter().any(|i| &i.key == key)
    {
        return Err(AppError::InvalidField {
            field: "vendor.item".into(),
            message: tr!(
                "Choose one of the year's items",
                "Wähle einen der Posten des Jahres"
            ),
        });
    }
    let after = VendorRow {
        default_item_key: item_key,
        ..before.clone()
    };
    let op = Op::new(
        tr!(
            "Set Default Item of {}",
            "Standardposten festlegen: {}",
            before.name
        ),
        vec![Change::update(store::VENDORS, &before, &after)?],
    );
    db.write(op, source)
}

/// Moves every document of `from` to `into` and removes `from`.
pub fn merge(db: &Db, from: &str, into: &str, source: Source) -> AppResult<Written> {
    if from == into {
        return Err(AppError::InvalidInput(tr!(
            "A vendor can't merge into itself",
            "Ein Händler kann nicht mit sich selbst zusammengeführt werden"
        )));
    }
    let gone = vendor(db, from)?;
    let kept = vendor(db, into)?;
    let mut changes = Vec::new();
    for doc in db.read(store::documents)? {
        if doc.vendor_id.as_deref() == Some(from) {
            let moved = super::store::DocumentRow {
                vendor_id: Some(kept.id.clone()),
                ..doc.clone()
            };
            changes.push(Change::update(store::DOCUMENTS, &doc, &moved)?);
        }
    }
    changes.push(Change::delete(store::VENDORS, &gone)?);
    let op = Op::new(
        tr!(
            "Merge {} into {}",
            "{} in {} zusammenführen",
            gone.name,
            kept.name
        ),
        changes,
    );
    db.write(op, source)
}

pub fn delete(db: &Db, id: &str, source: Source) -> AppResult<Written> {
    let gone = vendor(db, id)?;
    if db
        .read(store::documents)?
        .iter()
        .any(|d| d.vendor_id.as_deref() == Some(id))
    {
        return Err(AppError::InvalidInput(tr!(
            "{} has documents; merge it into another vendor instead",
            "{} hat Belege; führe ihn stattdessen mit einem anderen Händler zusammen",
            gone.name
        )));
    }
    let op = Op::new(
        tr!("Delete Vendor {}", "Händler löschen: {}", gone.name),
        vec![Change::delete(store::VENDORS, &gone)?],
    );
    db.write(op, source)
}
