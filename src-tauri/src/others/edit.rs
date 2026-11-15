//! Adding, changing and deleting other documents; each is one undoable op.

use std::path::Path;

use serde::Deserialize;
use ts_rs::TS;

use super::{OTHERS, OtherRow, by_hash, other};
use crate::date;
use crate::db::{Change, Db, Op, Source, Written};
use crate::documents::files;
use crate::error::{AppError, AppResult};
use crate::statements::import::slug;

/// One change to an other document; a blank date clears it.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "field", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum OtherEdit {
    Title { text: String },
    Date { value: String },
    Note { text: String },
}

fn stem(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_string()
}

/// Adds a PDF under its file name as title; the change and the new document's id.
pub fn add(
    db: &Db,
    config_dir: &Path,
    original_name: &str,
    bytes: &[u8],
    now: i64,
) -> AppResult<(Written, String)> {
    let hash = crate::statements::file_hash(bytes);
    if let Some(existing) = db.read(|c| by_hash(c, &hash))? {
        return Err(AppError::InvalidInput(crate::tr!(
            "This PDF is already kept as {}",
            "Dieses PDF ist schon als {} abgelegt",
            existing.title
        )));
    }
    let title = stem(original_name);
    // `slug` falls back to an account's name for text without letters or digits.
    let name = if title.chars().any(char::is_alphanumeric) {
        slug(&title)
    } else {
        "document".to_string()
    };
    let file = files::copy_to(
        config_dir,
        &format!("{}/other/{name}.pdf", files::FOLDER),
        bytes,
        &hash,
    )?;
    let row = OtherRow {
        id: uuid::Uuid::new_v4().to_string(),
        title: title.clone(),
        date: None,
        note: String::new(),
        file,
        file_hash: hash,
        created_at: now,
    };
    let written = db.write(
        Op::new(
            crate::tr!("Add {title}", "{title} hinzufügen"),
            vec![Change::insert(OTHERS, &row)?],
        ),
        Source::Ui,
    )?;
    Ok((written, row.id))
}

fn stored(db: &Db, id: &str) -> AppResult<OtherRow> {
    db.read(|c| other(c, id))?.ok_or_else(|| {
        AppError::NotFound(crate::tr!(
            "No such document",
            "Dieses Dokument gibt es nicht"
        ))
    })
}

/// Applies one change; None when the document already says that.
pub fn edit(db: &Db, id: &str, change: &OtherEdit) -> AppResult<Option<Written>> {
    let before = stored(db, id)?;
    let mut after = before.clone();
    let label = match change {
        OtherEdit::Title { text } => {
            let text = text.trim();
            if text.is_empty() {
                return Err(AppError::InvalidField {
                    field: "other.title".into(),
                    message: crate::tr!("A title is needed", "Ein Titel ist nötig"),
                });
            }
            after.title = text.to_string();
            crate::tr!(
                "Rename {} to {text}",
                "{} in {text} umbenennen",
                before.title
            )
        }
        OtherEdit::Date { value } => {
            after.date = if value.trim().is_empty() {
                None
            } else {
                Some(date::iso(date::parse(value.trim()).ok_or_else(|| {
                    AppError::InvalidField {
                        field: "other.date".into(),
                        message: crate::tr!("This isn't a date", "Das ist kein Datum"),
                    }
                })?))
            };
            crate::tr!("Change Date of {}", "Datum von {} ändern", before.title)
        }
        OtherEdit::Note { text } => {
            after.note = text.trim().to_string();
            crate::tr!("Change Note of {}", "Notiz von {} ändern", before.title)
        }
    };
    if after == before {
        return Ok(None);
    }
    db.write(
        Op::new(label, vec![Change::update(OTHERS, &before, &after)?]),
        Source::Ui,
    )
    .map(Some)
}

/// Deletes an other document; its PDF stays in the data folder.
pub fn delete(db: &Db, id: &str) -> AppResult<Written> {
    let row = stored(db, id)?;
    db.write(
        Op::new(
            crate::tr!("Delete {}", "{} löschen", row.title),
            vec![Change::delete(OTHERS, &row)?],
        ),
        Source::Ui,
    )
}
