//! Statement import, coverage and account lines as Tauri commands; dropped PDFs arrive through `dropped`.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use super::import::{self, ImportPreview};
use super::view::{self, AccountLines, Coverage};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::events::IMPORT_READY_EVENT;
use crate::state::AppState;

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

/// Reads and checks the file, keeps it under a token and describes it for the Import sheet.
fn prepare(app: &AppHandle, path: &std::path::Path) -> AppResult<ImportPreview> {
    let state = app.state::<AppState>();
    let bytes = std::fs::read(path)?;
    let pending = import::prepare(&state.db, &crate::config_dir(), bytes)?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let token = state.imports.put(pending.clone());
    import::preview(
        &state.db,
        token,
        file_name,
        &pending,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Asks for a statement PDF and reads it; null when the panel was cancelled.
#[tauri::command]
pub async fn statement_import_choose(app: AppHandle) -> AppResult<Option<ImportPreview>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Import a Statement")
        .add_filter("Statement PDF", &["pdf"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx
        .await
        .map_err(|_| AppError::Io("The file panel closed without an answer".into()))?;
    let Some(path) = chosen else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::Io(format!("The chosen file isn't a file path: {e}")))?;
    tauri::async_runtime::spawn_blocking(move || prepare(&app, &path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(Some)
}

/// Imports a read statement; returns the label of the change.
#[tauri::command]
pub fn statement_import_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
) -> AppResult<String> {
    let pending = state.imports.take(&token).ok_or_else(|| {
        AppError::NotFound("This statement is no longer waiting to be imported".into())
    })?;
    let now = chrono::Utc::now().timestamp_millis();
    let (written, _) = import::commit(
        &state.db,
        &crate::config_dir(),
        pending,
        now,
        Source::Import,
    )?;
    announce(&app, &written);
    Ok(written.label)
}

#[tauri::command]
pub fn statement_import_cancel(state: State<'_, AppState>, token: String) {
    state.imports.take(&token);
}

#[tauri::command]
pub fn statements_coverage(state: State<'_, AppState>) -> AppResult<Coverage> {
    view::coverage(&state.db, today(), DateOrder::system())
}

#[tauri::command]
pub fn account_lines(state: State<'_, AppState>, account_id: String) -> AppResult<AccountLines> {
    view::account_lines(
        &state.db,
        &account_id,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Payload of `import_ready`: a dropped statement read and waiting, or why it can't be imported.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ImportReady {
    pub preview: Option<ImportPreview>,
    pub error: Option<String>,
}

/// PDFs dropped on the main window: each is read as a statement and offered in the Import sheet.
pub fn dropped(app: &AppHandle, paths: Vec<PathBuf>) {
    let pdfs: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
        .collect();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        for path in pdfs {
            let payload = match prepare(&app, &path) {
                Ok(preview) => ImportReady {
                    preview: Some(preview),
                    error: None,
                },
                Err(e) => ImportReady {
                    preview: None,
                    error: Some(e.to_string()),
                },
            };
            if let Err(e) = app.emit(IMPORT_READY_EVENT, payload) {
                log::warn!("Couldn't offer a dropped statement: {e}");
            }
        }
    });
}
