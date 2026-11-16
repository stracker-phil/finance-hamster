//! Profiles as Tauri commands. Creating or switching opens that profile in place (ADR 0061).

use tauri::{AppHandle, Emitter};

use super::Profiles;
use crate::error::AppResult;
use crate::events::PROFILES_CHANGED;

#[tauri::command]
pub async fn profiles_list() -> AppResult<Profiles> {
    Ok(super::load(&crate::config_dir()))
}

#[tauri::command]
pub async fn profile_create(app: AppHandle, name: String) -> AppResult<()> {
    let profile = super::add(&crate::config_dir(), &name)?;
    super::switch::switch(&app, &profile.id)
}

#[tauri::command]
pub async fn profile_switch(app: AppHandle, id: String) -> AppResult<()> {
    super::switch::switch(&app, &id)
}

#[tauri::command]
pub async fn profile_rename(app: AppHandle, id: String, name: String) -> AppResult<()> {
    super::rename(&crate::config_dir(), &id, &name)?;
    if let Err(e) = app.emit(PROFILES_CHANGED, ()) {
        log::warn!("Couldn't announce the renamed profile: {e}");
    }
    Ok(())
}
