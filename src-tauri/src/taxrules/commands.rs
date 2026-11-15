//! Settings → Tax Rules as Tauri commands.

use chrono::{DateTime, Local};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use super::{Imported, ReloadPreview, RulesFile, files};
use crate::date::{self, DateOrder};
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn date_label(millis: i64) -> String {
    DateTime::from_timestamp_millis(millis)
        .map(|at| date::day_month_year(at.with_timezone(&Local).date_naive(), DateOrder::system()))
        .unwrap_or_default()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[tauri::command]
pub fn rules_overview(state: State<'_, AppState>) -> AppResult<Vec<RulesFile>> {
    super::overview(&state.db, &crate::config_dir(), &date_label)
}

#[tauri::command]
pub fn rules_reload_preview(state: State<'_, AppState>) -> AppResult<Vec<ReloadPreview>> {
    super::preview(&state.db, &crate::config_dir())
}

/// The label of the applied change, or null when no file had changed.
#[tauri::command]
pub fn rules_reload_apply(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let written = super::apply(&state.db, &crate::config_dir(), now())?;
    if let Some(written) = &written {
        announce(&app, written);
    }
    Ok(written.map(|w| w.label))
}

/// Asks for a JSON file and imports it; null when the panel was cancelled.
#[tauri::command]
pub async fn rules_import(app: AppHandle) -> AppResult<Option<Imported>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(crate::tr!("Import a Rules File", "Regeldatei importieren"))
        .add_filter(crate::tr!("Rules file", "Regeldatei"), &["json"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx.await.map_err(|_| {
        AppError::Io(crate::tr!(
            "The file panel closed without an answer",
            "Das Dateifenster wurde ohne Antwort geschlossen"
        ))
    })?;
    let Some(path) = chosen else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| {
        AppError::Io(crate::tr!(
            "The chosen file isn't a file path: {e}",
            "Die gewählte Datei ist kein Dateipfad: {e}"
        ))
    })?;
    let state = app.state::<AppState>();
    super::import_file(&state.db, &crate::config_dir(), &path).map(Some)
}

#[tauri::command]
pub fn rules_duplicate(state: State<'_, AppState>) -> AppResult<String> {
    super::duplicate(&state.db, &crate::config_dir())
}

/// Opens a year's file in the default editor; a closed year opens a read-only copy of its frozen rules.
#[tauri::command]
pub fn rules_open(state: State<'_, AppState>, year: i32) -> AppResult<()> {
    let path = super::open_target(&state.db, &crate::config_dir(), year)?;
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't open {}: {e}",
            "{} ließ sich nicht öffnen: {e}",
            path.display()
        ))
    })
}

#[tauri::command]
pub fn rules_show_folder() -> AppResult<()> {
    let dir = files::dir(&crate::config_dir());
    tauri_plugin_opener::reveal_item_in_dir(&dir).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't show the rules folder: {e}",
            "Der Regelordner ließ sich nicht zeigen: {e}"
        ))
    })
}
