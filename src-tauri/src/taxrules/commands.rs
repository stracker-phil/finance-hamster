//! Settings → Tax Rules as Tauri commands.

use std::collections::BTreeSet;

use tauri::{AppHandle, State};

use super::edit::{self, JsonPreview};
use super::view::{self, RulesYearView};
use super::{RulesOverview, check};
use crate::date::DateOrder;
use crate::db::Written;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::taxyears;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn announced(app: &AppHandle, written: Written) -> String {
    announce(app, &written);
    written.label
}

#[tauri::command]
pub fn rules_overview(state: State<'_, AppState>) -> AppResult<RulesOverview> {
    super::overview(&state.db)
}

#[tauri::command]
pub fn rules_year(state: State<'_, AppState>, year: i32) -> AppResult<RulesYearView> {
    let row = state
        .db
        .read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(crate::tr!("Tax year {year}", "Steuerjahr {year}")))?;
    let ruleset = check::read(&row.rules, &BTreeSet::new())
        .map_err(|p| AppError::InvalidInput(p.join("; ")))?;
    Ok(view::year_view(
        &row,
        &ruleset,
        state.settings.get().number_format,
        DateOrder::system(),
    ))
}

/// The label of the change.
#[tauri::command]
pub fn rules_set_value(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    path: String,
    value: String,
) -> AppResult<String> {
    let format = state.settings.get().number_format;
    let written = edit::set_value(&state.db, year, &path, &value, format, now())?;
    Ok(announced(&app, written))
}

#[tauri::command]
pub fn rules_json(state: State<'_, AppState>, year: i32) -> AppResult<String> {
    edit::json(&state.db, year)
}

#[tauri::command]
pub fn rules_preview_json(
    state: State<'_, AppState>,
    year: i32,
    text: String,
) -> AppResult<JsonPreview> {
    edit::preview_json(&state.db, year, &text)
}

#[tauri::command]
pub fn rules_set_json(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    text: String,
) -> AppResult<String> {
    let written = edit::set_json(&state.db, year, &text, now())?;
    Ok(announced(&app, written))
}

#[tauri::command]
pub fn rules_use_builtin(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<String> {
    let written = super::use_builtin(&state.db, year, now())?;
    Ok(announced(&app, written))
}

#[tauri::command]
pub fn rules_start_next(app: AppHandle, state: State<'_, AppState>) -> AppResult<String> {
    let written = super::start_next(&state.db, now())?;
    Ok(announced(&app, written))
}

/// What happened to the rules while the app started (a newer built-in version taken on), once.
#[tauri::command]
pub fn rules_take_notices(state: State<'_, AppState>) -> Vec<String> {
    state
        .notices
        .lock()
        .map(|mut n| std::mem::take(&mut *n))
        .unwrap_or_default()
}
