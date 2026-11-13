//! The bus as Tauri commands: the palette lists and runs commands.

use tauri::AppHandle;

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
