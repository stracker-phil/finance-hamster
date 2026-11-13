//! A document's life: a draft is finished once its required facts are set and stays locked until unlocked;
//! any document can be excluded with a reason and restored; an excluded one can be deleted from the Archive
//! once it has waited there long enough, which removes its file and leaves a permanent deletion log entry.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::store::{self, DocState, DocumentRow};
use super::title_of;
use super::view::{linked_lines, paid_cents};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};

pub const DELETIONS: &str = "deletions";

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// A deleted document as the deletion log keeps it, forever.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeletionRow {
    pub id: String,
    pub document_id: String,
    pub title: String,
    pub original_name: String,
    pub file: String,
    pub file_hash: String,
    pub created_at: i64,
    pub excluded_at: Option<i64>,
    pub deleted_at: i64,
    pub reason: String,
}

fn load(db: &Db, id: &str) -> AppResult<DocumentRow> {
    db.read(|c| store::document_by_id(c, id))?
        .ok_or_else(|| AppError::NotFound(format!("No document {id}")))
}

fn title(db: &Db, doc: &DocumentRow) -> AppResult<String> {
    Ok(title_of(doc, &db.read(store::vendors)?))
}

/// Writes the document after `edit` as one op labelled "<verb> <title>".
fn transition(
    db: &Db,
    doc: DocumentRow,
    verb: &str,
    source: Source,
    edit: impl FnOnce(&mut DocumentRow),
) -> AppResult<Written> {
    let mut after = doc.clone();
    edit(&mut after);
    let label = format!("{verb} {}", title(db, &doc)?);
    db.write(
        Op::new(label, vec![Change::update(store::DOCUMENTS, &doc, &after)?]),
        source,
    )
}

/// The required facts a draft still lacks before it can be finished.
pub fn missing(db: &Db, doc: &DocumentRow) -> AppResult<Vec<&'static str>> {
    let lines = linked_lines(db, &doc.id)?;
    Ok([
        (doc.pay_date.is_none(), "payment date"),
        (paid_cents(doc, &lines).is_none(), "amount paid"),
        (doc.vendor_id.is_none(), "vendor"),
        (doc.item_key.is_none(), "item"),
    ]
    .into_iter()
    .filter(|(gone, _)| *gone)
    .map(|(_, name)| name)
    .collect())
}

/// Locks a draft whose required facts are all set.
pub fn finish(db: &Db, id: &str, now: i64, source: Source) -> AppResult<Written> {
    let doc = load(db, id)?;
    if doc.state != DocState::Draft {
        return Err(AppError::InvalidInput(
            "Only a draft can be finished".into(),
        ));
    }
    let gaps = missing(db, &doc)?;
    if !gaps.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Set the {} first",
            gaps.join(", ")
        )));
    }
    transition(db, doc, "Finish", source, |d| {
        d.state = DocState::Finished;
        d.finished_at = Some(now);
    })
}

/// Makes a finished document a draft again, so it can be changed.
pub fn unlock(db: &Db, id: &str, source: Source) -> AppResult<Written> {
    let doc = load(db, id)?;
    if doc.state != DocState::Finished {
        return Err(AppError::InvalidInput(
            "Only a finished document can be unlocked".into(),
        ));
    }
    transition(db, doc, "Unlock", source, |d| {
        d.state = DocState::Draft;
        d.finished_at = None;
    })
}

/// Takes a document out of every total and list; it shows only in its year's Archive.
pub fn exclude(db: &Db, id: &str, reason: &str, now: i64, source: Source) -> AppResult<Written> {
    let doc = load(db, id)?;
    if doc.state == DocState::Excluded {
        return Err(AppError::InvalidInput(
            "This document is excluded already".into(),
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::InvalidField {
            field: "document.reason".into(),
            message: "Say why it is excluded".into(),
        });
    }
    transition(db, doc, "Exclude", source, |d| {
        d.state = DocState::Excluded;
        d.excluded_at = Some(now);
        d.exclude_reason = Some(reason.to_string());
        d.finished_at = None;
    })
}

/// Brings an excluded document back as a draft.
pub fn restore(db: &Db, id: &str, source: Source) -> AppResult<Written> {
    let doc = load(db, id)?;
    if doc.state != DocState::Excluded {
        return Err(AppError::InvalidInput(
            "Only an excluded document can be restored".into(),
        ));
    }
    transition(db, doc, "Restore", source, |d| {
        d.state = DocState::Draft;
        d.excluded_at = None;
        d.exclude_reason = None;
    })
}

/// The first moment an excluded document may be deleted.
pub fn deletable_from(doc: &DocumentRow, archive_days: u32) -> Option<i64> {
    doc.excluded_at
        .map(|at| at + i64::from(archive_days) * DAY_MS)
}

/// Whether an excluded document has waited long enough in the Archive at `now`.
pub fn deletable_at(doc: &DocumentRow, archive_days: u32, now: i64) -> bool {
    deletable_from(doc, archive_days).is_none_or(|from| now >= from)
}

/// Deletes an excluded document that waited `archive_days` in the Archive: its file goes, its rows go, and a
/// deletion log entry stays. It can't be undone.
pub fn delete(
    db: &Db,
    config_dir: &Path,
    id: &str,
    reason: &str,
    understood: bool,
    archive_days: u32,
    now: i64,
) -> AppResult<Written> {
    let doc = load(db, id)?;
    if doc.state != DocState::Excluded {
        return Err(AppError::InvalidInput(
            "Only an excluded document can be deleted".into(),
        ));
    }
    if !deletable_at(&doc, archive_days, now) {
        return Err(AppError::InvalidInput(format!(
            "An excluded document can be deleted after {archive_days} days in the Archive"
        )));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::InvalidField {
            field: "deletion.reason".into(),
            message: "Say why it is deleted".into(),
        });
    }
    if !understood {
        return Err(AppError::InvalidField {
            field: "deletion.understood".into(),
            message: "Confirm that this can't be undone".into(),
        });
    }
    let title = title(db, &doc)?;
    let mut changes = Vec::new();
    for link in db.read(store::links)? {
        if link.document_id == doc.id {
            changes.push(Change::delete(store::LINKS, &link)?);
        }
    }
    changes.push(Change::delete(store::DOCUMENTS, &doc)?);
    changes.push(Change::insert(
        DELETIONS,
        &DeletionRow {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: doc.id.clone(),
            title: title.clone(),
            original_name: doc.original_name.clone(),
            file: doc.file.clone(),
            file_hash: doc.file_hash.clone(),
            created_at: doc.created_at,
            excluded_at: doc.excluded_at,
            deleted_at: now,
            reason: reason.to_string(),
        },
    )?);
    // Written as the app's own op, which is never undoable: a deleted file can't come back.
    let written = db.write(Op::new(format!("Delete {title}"), changes), Source::App)?;
    let path = config_dir.join(&doc.file);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(written)
}

/// Every deletion, newest first.
pub fn deletions(db: &Db) -> AppResult<Vec<DeletionRow>> {
    db.read(|conn| {
        let mut query = conn.prepare(&format!(
            "SELECT id, document_id, title, original_name, file, file_hash, created_at, excluded_at, deleted_at, \
             reason FROM {DELETIONS} ORDER BY deleted_at DESC"
        ))?;
        let rows = query.query_map([], |row| {
            Ok(DeletionRow {
                id: row.get(0)?,
                document_id: row.get(1)?,
                title: row.get(2)?,
                original_name: row.get(3)?,
                file: row.get(4)?,
                file_hash: row.get(5)?,
                created_at: row.get(6)?,
                excluded_at: row.get(7)?,
                deleted_at: row.get(8)?,
                reason: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

/// An excluded document as its year's Archive lists it.
#[derive(Serialize, Clone, Debug, PartialEq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ArchivedDocument {
    pub id: String,
    pub title: String,
    pub file: String,
    pub reason: String,
    /// `Excluded 2 Nov 2026`.
    pub excluded: String,
    pub deletable: bool,
    /// `Can be deleted from 9 Nov 2026`, while it can't yet.
    pub deletable_note: Option<String>,
}

/// A deletion as Settings → Privacy & Data lists it.
#[derive(Serialize, Clone, Debug, PartialEq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DeletionView {
    pub title: String,
    pub file: String,
    /// `Added 2 Mar 2026 · deleted 13 Nov 2026`.
    pub dates: String,
    pub reason: String,
}

fn day_of(at: i64, order: crate::date::DateOrder) -> String {
    chrono::DateTime::from_timestamp_millis(at)
        .map(|t| crate::date::day_month_year(t.with_timezone(&chrono::Local).date_naive(), order))
        .unwrap_or_default()
}

/// The year an excluded document belongs to: its payment's, else the year it was added.
fn archive_year(doc: &DocumentRow) -> Option<i32> {
    doc.pay_date
        .as_deref()
        .and_then(|d| d.get(..4)?.parse().ok())
        .or_else(|| {
            chrono::DateTime::from_timestamp_millis(doc.created_at)
                .map(|t| chrono::Datelike::year(&t.with_timezone(&chrono::Local)))
        })
}

/// The excluded documents of `year`, newest exclusion first.
pub fn archive(
    db: &Db,
    year: i32,
    archive_days: u32,
    now: i64,
    order: crate::date::DateOrder,
) -> AppResult<Vec<ArchivedDocument>> {
    let vendors = db.read(store::vendors)?;
    let mut docs: Vec<DocumentRow> = db
        .read(store::documents)?
        .into_iter()
        .filter(|d| d.state == DocState::Excluded && archive_year(d) == Some(year))
        .collect();
    docs.sort_by_key(|d| std::cmp::Reverse(d.excluded_at));
    Ok(docs
        .into_iter()
        .map(|d| {
            let from = deletable_from(&d, archive_days);
            ArchivedDocument {
                title: title_of(&d, &vendors),
                reason: d.exclude_reason.clone().unwrap_or_default(),
                excluded: format!(
                    "Excluded {}",
                    d.excluded_at.map(|a| day_of(a, order)).unwrap_or_default()
                ),
                deletable: deletable_at(&d, archive_days, now),
                deletable_note: from
                    .filter(|f| now < *f)
                    .map(|f| format!("Can be deleted from {}", day_of(f, order))),
                file: d.file,
                id: d.id,
            }
        })
        .collect())
}

pub fn deletion_log(db: &Db, order: crate::date::DateOrder) -> AppResult<Vec<DeletionView>> {
    Ok(deletions(db)?
        .into_iter()
        .map(|d| DeletionView {
            dates: format!(
                "Added {} · deleted {}",
                day_of(d.created_at, order),
                day_of(d.deleted_at, order)
            ),
            title: d.title,
            file: d.file,
            reason: d.reason,
        })
        .collect())
}
