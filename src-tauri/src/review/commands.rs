use tauri::{AppHandle, State};

use super::ReviewQueue;
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::AppResult;
use crate::rules::store::Action;
use crate::state::AppState;

#[tauri::command]
pub async fn review_queue(state: State<'_, AppState>) -> AppResult<ReviewQueue> {
    super::queue(
        &state.db,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Decides a line by hand; returns the label of the change.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn review_decide(
    app: AppHandle,
    state: State<'_, AppState>,
    line_id: String,
    action: Action,
    item_key: Option<String>,
    subscription_id: Option<String>,
    always: bool,
    term: Option<String>,
) -> AppResult<String> {
    let written = super::decide_as(
        &state.db,
        &line_id,
        action,
        item_key,
        subscription_id,
        always,
        term.as_deref(),
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    announce(&app, &written);
    Ok(written.label)
}
