//! Settings as Tauri commands.

use tauri::{AppHandle, Emitter, State};

use super::{Settings, SettingsPatch};
use crate::error::AppResult;
use crate::events::SETTINGS_CHANGED_EVENT;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

/// Tells every window, so they follow the new look at once.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> AppResult<Settings> {
    let settings = state.settings.update(patch)?;
    if let Err(e) = app.emit(SETTINGS_CHANGED_EVENT, ()) {
        log::warn!("Couldn't announce the new settings: {e}");
    }
    Ok(settings)
}
