//! Where a document's PDF lives: a draft with a payment date sits at `docs/<year>/<yyyy-mm-dd>-<vendor>.pdf`,
//! one without in `docs/inbox/` under its original name; a finished document never moves.
//!
//! Edits never change the `file` column: after every write `relocate` moves the drafts' files to where their
//! current facts say and records the new paths in an op of its own. Undoing an edit therefore moves the file
//! back on the next `relocate`, and no undo can leave a path pointing at nothing.

use std::path::{Path, PathBuf};

use super::store::{self, DocState, DocumentRow, VendorRow};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::AppResult;
use crate::statements::import::slug;
use crate::store::json_file::create_private_dir;

pub const FOLDER: &str = "docs";
const INBOX: &str = "docs/inbox";

fn stem(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
}

/// The path the document's facts ask for, without collisions resolved.
fn wanted_base(doc: &DocumentRow, vendors: &[VendorRow]) -> String {
    let Some((date, year)) = doc.pay_date.as_deref().and_then(|d| Some((d, d.get(..4)?))) else {
        return format!("{INBOX}/{}", doc.original_name);
    };
    let name = doc
        .vendor_id
        .as_ref()
        .and_then(|id| vendors.iter().find(|v| &v.id == id))
        .map_or_else(|| slug(stem(&doc.original_name)), |v| slug(&v.name));
    format!("{FOLDER}/{year}/{date}-{name}.pdf")
}

fn same_file(path: &Path, hash: &str) -> bool {
    std::fs::read(path).is_ok_and(|bytes| crate::statements::file_hash(&bytes) == hash)
}

/// `base`, or `base` with `-2`, `-3` before the extension when another file holds the name.
fn free_name(config_dir: &Path, base: &str, hash: &str) -> String {
    let (head, ext) = match base.rsplit_once('.') {
        Some((head, ext)) if !head.ends_with('/') => (head.to_string(), format!(".{ext}")),
        _ => (base.to_string(), String::new()),
    };
    let mut name = base.to_string();
    let mut n = 2;
    loop {
        let path = config_dir.join(&name);
        if !path.exists() || same_file(&path, hash) {
            return name;
        }
        name = format!("{head}-{n}{ext}");
        n += 1;
    }
}

fn ensure_parent(path: &Path) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    Ok(())
}

/// Copies a new document's bytes into the inbox; returns its relative path.
pub fn copy_in(
    config_dir: &Path,
    original_name: &str,
    bytes: &[u8],
    hash: &str,
) -> AppResult<String> {
    copy_to(config_dir, &format!("{INBOX}/{original_name}"), bytes, hash)
}

/// Copies bytes to the relative path `base`, numbered when another file holds the name; returns the path.
pub fn copy_to(config_dir: &Path, base: &str, bytes: &[u8], hash: &str) -> AppResult<String> {
    let name = free_name(config_dir, base, hash);
    let path = config_dir.join(&name);
    ensure_parent(&path)?;
    if !path.exists() {
        std::fs::write(&path, bytes)?;
    }
    Ok(name)
}

/// The file with `hash` somewhere under `docs/`, for a path that no longer holds it.
fn find_by_hash(config_dir: &Path, hash: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, hash: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path, hash) {
                    return Some(found);
                }
            } else if same_file(&path, hash) {
                return Some(path);
            }
        }
        None
    }
    walk(&config_dir.join(FOLDER), hash)
}

/// Where the document's file is: its stored path, or wherever its hash turns up under `docs/`.
fn source_of(config_dir: &Path, doc: &DocumentRow) -> Option<PathBuf> {
    let stored = config_dir.join(&doc.file);
    if same_file(&stored, &doc.file_hash) {
        Some(stored)
    } else {
        find_by_hash(config_dir, &doc.file_hash)
    }
}

fn move_file(from: &Path, to: &Path) -> AppResult<()> {
    if from != to {
        ensure_parent(to)?;
        std::fs::rename(from, to)?;
    }
    Ok(())
}

/// Moves every draft's file to where its facts say; None when every file is in place.
pub fn relocate(db: &Db, config_dir: &Path) -> AppResult<Option<Written>> {
    let vendors = db.read(store::vendors)?;
    let mut changes = Vec::new();
    for doc in db.read(store::documents)? {
        if doc.state != DocState::Draft {
            continue;
        }
        let base = wanted_base(&doc, &vendors);
        if base == doc.file {
            continue;
        }
        let Some(from) = source_of(config_dir, &doc) else {
            log::warn!("The file of document {} is missing", doc.id);
            continue;
        };
        let name = free_name(config_dir, &base, &doc.file_hash);
        if name == doc.file {
            continue;
        }
        if let Err(e) = move_file(&from, &config_dir.join(&name)) {
            log::warn!("Couldn't move the file of document {}: {e}", doc.id);
            continue;
        }
        let moved = DocumentRow {
            file: name,
            ..doc.clone()
        };
        changes.push(Change::update(store::DOCUMENTS, &doc, &moved)?);
    }
    if changes.is_empty() {
        return Ok(None);
    }
    db.write(Op::new("Move Document Files", changes), Source::App)
        .map(Some)
}
