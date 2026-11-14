use tauri::{AppHandle, State};

use super::YearCheckView;
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn year_check_view(state: State<'_, AppState>, year: i32) -> AppResult<YearCheckView> {
    super::view(
        &state.db,
        year,
        chrono::Local::now().date_naive(),
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Records that a run answered every question; returns the view with the new date.
#[tauri::command]
pub fn year_check_complete(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<YearCheckView> {
    let written = super::complete(
        &state.db,
        year,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    announce(&app, &written);
    year_check_view(state, year)
}
