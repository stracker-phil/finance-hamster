//! The bus as Tauri commands: the palette lists and runs commands.

use tauri::{AppHandle, Emitter, Manager};

use super::CommandInfo;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn list_commands() -> Vec<CommandInfo> {
    super::list()
}

#[tauri::command]
pub fn run_command(app: AppHandle, id: String) -> AppResult<()> {
    super::run(&app, &id)
}

/// Gives a command its own shortcut (null: none); the menu bar and every window follow.
#[tauri::command]
pub fn keybinding_set(app: AppHandle, id: String, accelerator: Option<String>) -> AppResult<()> {
    super::keys::set(&crate::config_dir(), &id, accelerator.as_deref())?;
    changed(&app);
    Ok(())
}

/// The command that already holds this shortcut, if any.
#[tauri::command]
pub fn keybinding_conflict(
    id: String,
    accelerator: String,
) -> AppResult<Option<super::keys::KeyConflict>> {
    super::keys::conflict(&crate::config_dir(), &id, &accelerator)
}

/// Moves a shortcut from the command holding it to this one.
#[tauri::command]
pub fn keybinding_reassign(app: AppHandle, id: String, accelerator: String) -> AppResult<()> {
    super::keys::reassign(&crate::config_dir(), &id, &accelerator)?;
    changed(&app);
    Ok(())
}

/// Back to the command's default shortcut.
#[tauri::command]
pub fn keybinding_reset(app: AppHandle, id: String) -> AppResult<()> {
    super::keys::reset(&crate::config_dir(), &id)?;
    changed(&app);
    Ok(())
}

fn changed(app: &AppHandle) {
    crate::menu::refresh(app);
    let _ = app.emit(crate::events::COMMANDS_CHANGED, ());
}

/// Help → Show Data Folder: the folder holding the database, the journal and every PDF, in Finder.
pub fn show_data_folder(_app: &AppHandle) -> AppResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(crate::config_dir()).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't show the data folder: {e}",
            "Der Datenordner konnte nicht angezeigt werden: {e}"
        ))
    })
}

/// Help → Open Log Folder.
pub fn open_log_folder(app: &AppHandle) -> AppResult<()> {
    let dir = app.path().app_log_dir().map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't find the log folder: {e}",
            "Der Log-Ordner wurde nicht gefunden: {e}"
        ))
    })?;
    crate::store::json_file::create_private_dir(&dir)?;
    tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't open the log folder: {e}",
            "Der Log-Ordner konnte nicht geöffnet werden: {e}"
        ))
    })
}
