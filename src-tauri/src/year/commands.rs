use tauri::{AppHandle, State};

use super::page::{self, YearPage};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::taxrules::model::Ruleset;
use crate::taxyears;

/// `query` filters the rows (the header's filter field).
#[tauri::command]
pub fn year_page(state: State<'_, AppState>, year: i32, query: String) -> AppResult<YearPage> {
    page::page(
        &state.db,
        year,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Answers one of the year's questions; returns the label of the change, or null when nothing changed.
#[tauri::command]
pub fn year_answer(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    question: String,
    value: String,
) -> AppResult<Option<String>> {
    let row = state
        .db
        .read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(format!("No tax year {year}")))?;
    let ruleset: Ruleset = serde_json::from_str(&row.rules)?;
    let written = super::answers::answer(
        &state.db,
        &ruleset,
        year,
        &question,
        &value,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    if let Some(written) = &written {
        announce(&app, written);
    }
    Ok(written.map(|w| w.label))
}
