//! Other documents as the window shows them: grouped by the year of their date, and one document's form.

use serde::Serialize;
use ts_rs::TS;

use super::{OtherRow, other, others};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::{AppError, AppResult};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct OtherListRow {
    pub id: String,
    pub title: String,
    /// `10 Nov 2026`; empty without a date.
    pub date: String,
}

/// The documents of one year, or of none (`year` None) for those without a date.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct OtherGroup {
    pub title: String,
    pub year: Option<i32>,
    pub documents: Vec<OtherListRow>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct OtherView {
    pub id: String,
    pub title: String,
    /// `YYYY-MM-DD` for the date field; empty without one.
    pub date: String,
    pub note: String,
}

fn year_of(row: &OtherRow) -> Option<i32> {
    row.date.as_deref()?.get(..4)?.parse().ok()
}

/// Newest year first, the undated last; `query` keeps those whose title or note contains it.
pub fn list(db: &Db, query: &str, order: DateOrder) -> AppResult<Vec<OtherGroup>> {
    let needle = query.trim().to_lowercase();
    let mut groups: Vec<OtherGroup> = Vec::new();
    for row in db.read(others)? {
        if !needle.is_empty()
            && ![&row.title, &row.note]
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
        {
            continue;
        }
        let year = year_of(&row);
        let item = OtherListRow {
            id: row.id.clone(),
            title: row.title.clone(),
            date: row
                .date
                .as_deref()
                .and_then(date::parse)
                .map(|d| date::day_month_year(d, order))
                .unwrap_or_default(),
        };
        match groups.last_mut() {
            Some(group) if group.year == year => group.documents.push(item),
            _ => groups.push(OtherGroup {
                title: year.map_or_else(
                    || crate::tr!("Without date", "Ohne Datum"),
                    |y| y.to_string(),
                ),
                year,
                documents: vec![item],
            }),
        }
    }
    Ok(groups)
}

pub fn view(db: &Db, id: &str) -> AppResult<OtherView> {
    let row = db.read(|c| other(c, id))?.ok_or_else(|| {
        AppError::NotFound(crate::tr!(
            "No such document",
            "Dieses Dokument gibt es nicht"
        ))
    })?;
    Ok(OtherView {
        id: row.id,
        title: row.title,
        date: row.date.unwrap_or_default(),
        note: row.note,
    })
}
