//! Statement import, coverage and account lines as Tauri commands; dropped PDFs arrive through `dropped`.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use super::import::{self, ImportPreview};
use super::lines::{self, AccountLines};
use super::view::{self, Coverage};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::events::IMPORT_READY_EVENT;
use crate::state::AppState;
use crate::tr;

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

/// Reads and checks the file, keeps it under a token and describes it for the Import sheet.
fn prepare(app: &AppHandle, path: &std::path::Path) -> AppResult<ImportPreview> {
    let state = app.state::<AppState>();
    let bytes = std::fs::read(path)?;
    let pending = import::prepare(&state.db, &crate::profile_dir(), bytes)?;
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
        .set_title(tr!("Import a Statement", "Kontoauszug importieren"))
        .add_filter(tr!("Statement PDF", "Kontoauszug-PDF"), &["pdf"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx.await.map_err(|_| {
        AppError::Io(tr!(
            "The file panel closed without an answer",
            "Das Dateifenster wurde ohne Antwort geschlossen"
        ))
    })?;
    let Some(path) = chosen else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| {
        AppError::Io(tr!(
            "The chosen file isn't a file path: {e}",
            "Die gewählte Datei ist kein Dateipfad: {e}"
        ))
    })?;
    tauri::async_runtime::spawn_blocking(move || prepare(&app, &path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(Some)
}

/// Imports a read statement; returns the label of the change.
#[tauri::command]
pub async fn statement_import_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
) -> AppResult<String> {
    let pending = state.imports.take(&token).ok_or_else(|| {
        AppError::NotFound(tr!(
            "This statement is no longer waiting to be imported",
            "Dieser Kontoauszug wartet nicht mehr auf den Import"
        ))
    })?;
    let now = chrono::Utc::now().timestamp_millis();
    let (written, _) = import::commit(
        &state.db,
        &crate::profile_dir(),
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

/// Dokumente → Kontoauszüge: every imported statement by account.
#[tauri::command]
pub async fn statements_list(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<Vec<super::documents::AccountStatements>> {
    super::documents::list(
        &state.db,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn statement_view(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<super::documents::StatementView> {
    super::documents::view(
        &state.db,
        &id,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// The accounts' months in `year`, or in this year without one.
#[tauri::command]
pub async fn statements_coverage(
    state: State<'_, AppState>,
    query: String,
    year: Option<i32>,
) -> AppResult<Coverage> {
    let today = today();
    view::year_coverage(
        &state.db,
        today,
        year.unwrap_or_else(|| chrono::Datelike::year(&today)),
        &query,
        DateOrder::system(),
        state.settings.get().import_after_days,
    )
}

#[tauri::command]
pub async fn account_lines(
    state: State<'_, AppState>,
    account_id: String,
    query: String,
) -> AppResult<AccountLines> {
    lines::account_lines(
        &state.db,
        &account_id,
        &query,
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

/// PDFs dropped on the main window: a statement is offered in the Import sheet, any other PDF in the Add
/// document sheet.
pub fn dropped(app: &AppHandle, paths: Vec<PathBuf>) {
    let pdfs: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
        .collect();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        for path in pdfs {
            let statement = std::fs::read(&path).is_ok_and(|bytes| super::is_statement(&bytes));
            if !statement {
                crate::documents::commands::offer(&app, &path);
                continue;
            }
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
