//! Profiles as Tauri commands. Creating or switching starts the app anew on that profile.

use tauri::{AppHandle, Emitter, State};

use super::Profiles;
use crate::error::AppResult;
use crate::events::PROFILES_CHANGED;
use crate::state::AppState;

#[tauri::command]
pub async fn profiles_list() -> AppResult<Profiles> {
    Ok(super::load(&crate::config_dir()))
}

#[tauri::command]
pub async fn profile_create(app: AppHandle, name: String) -> AppResult<()> {
    super::create(&crate::config_dir(), &name)?;
    crate::instance::relaunch(&app)
}

#[tauri::command]
pub async fn profile_switch(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    if id == state.profile_id() {
        return Ok(());
    }
    super::activate(&crate::config_dir(), &id)?;
    crate::instance::relaunch(&app)
}

#[tauri::command]
pub async fn profile_rename(app: AppHandle, id: String, name: String) -> AppResult<()> {
    super::rename(&crate::config_dir(), &id, &name)?;
    if let Err(e) = app.emit(PROFILES_CHANGED, ()) {
        log::warn!("Couldn't announce the renamed profile: {e}");
    }
    Ok(())
}
