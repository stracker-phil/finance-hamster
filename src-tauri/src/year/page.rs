//! The tax year page: the total to enter, the year's questions, what needs attention, one copy value per
//! Kennzahl with its rows and note, and the same rows by month.

use serde::Serialize;
use ts_rs::TS;

use super::facts::{self, Placed, eur};
use super::{answers, summary};
use crate::date::DateOrder;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::settings::NumberFormat;
use crate::taxrules::model::{Item, Ruleset};
use crate::taxyears::{self, YearState};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct QuestionView {
    pub id: String,
    pub label: String,
    pub help: Option<String>,
    pub unit: Option<String>,
    /// The answer as the field shows it; empty while unanswered.
    pub value: String,
    pub answered: bool,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum AttentionKind {
    /// An ANV payment on an item that needs a document, without one.
    MissingInvoice,
    Draft,
    /// A document without a payment date.
    Undated,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AttentionView {
    pub kind: AttentionKind,
    pub title: String,
    pub detail: String,
    /// The document to open; None for a missing invoice.
    pub document_id: Option<String>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum RowKind {
    Document,
    /// An ANV bank line on an item that needs no document.
    Line,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct YearRow {
    pub id: String,
    pub kind: RowKind,
    pub title: String,
    /// `10 Nov · 60 % of €22.48`.
    pub detail: String,
    /// What it counts, `€13.49`.
    pub amount: String,
    pub draft: bool,
    /// The purchase year of a depreciation slice counted in a later year.
    pub carried_from: Option<i32>,
    pub kz: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct KennzahlView {
    pub key: String,
    pub kz: String,
    pub name: String,
    /// The plain number to copy into FinanzOnline, `1228,40`.
    pub copy: String,
    pub rows: Vec<YearRow>,
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct MonthView {
    /// `November`, `Every month`, `Whole year`.
    pub label: String,
    pub total: String,
    pub rows: Vec<YearRow>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct YearPage {
    pub year: i32,
    pub ruleset: String,
    pub state: YearState,
    /// `€1,228.40`.
    pub total: String,
    /// `Draft · 2 invoices and 1 answer missing`, `Ready to close`.
    pub progress: String,
    /// Done facts out of all, 0–100.
    pub progress_percent: u32,
    /// Shown when the total is below the Werbungskostenpauschale.
    pub pauschale_note: Option<String>,
    pub questions: Vec<QuestionView>,
    pub attention: Vec<AttentionView>,
    pub kennzahlen: Vec<KennzahlView>,
    pub months: Vec<MonthView>,
    pub archived: u32,
}

/// `query` filters the rows by title, Kennzahl or item name.
pub fn page(
    db: &Db,
    year: i32,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<YearPage> {
    let row = db
        .read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(format!("No tax year {year}")))?;
    let ruleset: Ruleset = serde_json::from_str(&row.rules)?;
    let given = answers::answers(db, year)?;
    let facts = facts::gather(db, year, &ruleset, format, order)?;
    let needle = query.trim().to_lowercase();
    let shown = |p: &Placed, item: &Item| {
        needle.is_empty()
            || [p.row.title.as_str(), item.kz.as_str(), item.name.as_str()]
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
    };
    let total: i64 = facts.placed.iter().map(|p| p.cents).sum();
    let in_view: Vec<&Placed> = facts
        .placed
        .iter()
        .filter(|p| ruleset.items.iter().any(|i| i.key == p.item && shown(p, i)))
        .collect();
    let questions = summary::question_views(&ruleset, &given, format);
    let unanswered = questions.iter().filter(|q| !q.answered).count();
    let (progress, progress_percent) = summary::progress_text(&facts, unanswered, questions.len());
    Ok(YearPage {
        year,
        ruleset: row.ruleset.clone(),
        state: row.state,
        total: eur(total, format),
        progress: if row.state == YearState::Closed {
            "Closed".into()
        } else {
            progress
        },
        progress_percent,
        pauschale_note: summary::pauschale_note(&ruleset, total, format),
        kennzahlen: summary::kennzahlen(
            &ruleset,
            &facts,
            &given,
            !needle.is_empty(),
            &shown,
            format,
        ),
        months: summary::months(&in_view, format),
        questions,
        archived: facts.archived as u32,
        attention: facts.attention,
    })
}
