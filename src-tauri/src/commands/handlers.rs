//! The bus as Tauri commands: the palette lists and runs commands.

use tauri::{AppHandle, Emitter};

use super::CommandInfo;
use crate::error::AppResult;

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
