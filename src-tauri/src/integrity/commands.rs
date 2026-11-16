use tauri::{AppHandle, Emitter, State};

use super::status::{self, StatusGroup};
use super::{FileCheck, LastCheck};
use crate::date::DateOrder;
use crate::error::AppResult;
use crate::events::FILES_CHECKED;
use crate::state::AppState;

/// Checks every stored PDF now; the banner and Settings follow the event.
#[tauri::command]
pub async fn files_verify(
    app: AppHandle,
    state: State<'_, AppState>,
    last: State<'_, LastCheck>,
) -> AppResult<FileCheck> {
    let db = state.db();
    let check = super::verify(&db, db.dir(), chrono::Utc::now().timestamp_millis())?;
    last.set(check.clone());
    let _ = app.emit(FILES_CHECKED, ());
    Ok(check)
}

/// The last check since launch; None before the first.
#[tauri::command]
pub fn files_last_check(last: State<'_, LastCheck>) -> Option<FileCheck> {
    last.get()
}

#[tauri::command]
pub async fn status_view(
    state: State<'_, AppState>,
    last: State<'_, LastCheck>,
) -> AppResult<Vec<StatusGroup>> {
    let db = state.db();
    status::status(
        &db,
        db.dir(),
        last.get().as_ref(),
        chrono::Local::now().date_naive(),
        DateOrder::system(),
    )
}

/// Opens Settings at a page or row, e.g. `verify-files` from the main window's banner.
#[tauri::command]
pub fn settings_show(app: AppHandle, page: String) {
    crate::window::settings::open(&app, Some(&page));
}
