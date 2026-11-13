use tauri::{AppHandle, State};

use super::ReviewQueue;
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::AppResult;
use crate::rules::store::Action;
use crate::state::AppState;

#[tauri::command]
pub fn review_queue(state: State<'_, AppState>) -> AppResult<ReviewQueue> {
    super::queue(
        &state.db,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Decides a line by hand; returns the label of the change.
#[tauri::command]
pub fn review_decide(
    app: AppHandle,
    state: State<'_, AppState>,
    line_id: String,
    action: Action,
    item_key: Option<String>,
    always: bool,
) -> AppResult<String> {
    let written = super::decide(
        &state.db,
        &line_id,
        action,
        item_key,
        always,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    announce(&app, &written);
    Ok(written.label)
}
