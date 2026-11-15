//! Settings as Tauri commands.

use tauri::{AppHandle, Emitter, State};

use super::{Settings, SettingsPatch};
use crate::error::AppResult;
use crate::events::{COMMANDS_CHANGED, SETTINGS_CHANGED_EVENT};
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

/// Tells every window, so they follow the new look at once; a new language also rebuilds the menu bar and the
/// command titles.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> AppResult<Settings> {
    let before = state.settings.get().language;
    let settings = state.settings.update(patch)?;
    if settings.language != before {
        crate::i18n::set(settings.language);
        crate::menu::refresh(&app);
        crate::window::settings::retitle(&app);
        if let Err(e) = app.emit(COMMANDS_CHANGED, ()) {
            log::warn!("Couldn't announce the new command titles: {e}");
        }
    }
    if let Err(e) = app.emit(SETTINGS_CHANGED_EVENT, ()) {
        log::warn!("Couldn't announce the new settings: {e}");
    }
    Ok(settings)
}
