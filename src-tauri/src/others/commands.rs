//! Other documents as Tauri commands.

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use super::edit::{self, OtherEdit};
use super::view::{self, OtherGroup, OtherView};
use crate::date::DateOrder;
use crate::db::Written;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::payslips::commands::PayslipAdded as Added;
use crate::state::AppState;
use crate::tr;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn announced(app: &AppHandle, written: Written) -> String {
    announce(app, &written);
    written.label
}

#[tauri::command]
pub fn others_list(state: State<'_, AppState>, query: String) -> AppResult<Vec<OtherGroup>> {
    view::list(&state.db, &query, DateOrder::system())
}

#[tauri::command]
pub fn other_view(state: State<'_, AppState>, id: String) -> AppResult<OtherView> {
    view::view(&state.db, &id)
}

/// Asks for a PDF and keeps it; None when cancelled.
#[tauri::command]
pub async fn other_add_pdf(app: AppHandle) -> AppResult<Option<Added>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(tr!("Add a Document", "Dokument hinzufügen"))
        .add_filter("PDF", &["pdf"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx.await.map_err(|_| {
        AppError::Io(tr!(
            "The file panel closed without an answer",
            "Das Dateifenster wurde ohne Antwort geschlossen"
        ))
    })?;
    let Some(chosen) = chosen else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|e| {
        AppError::Io(tr!(
            "The chosen file isn't a file path: {e}",
            "Die gewählte Datei ist kein Dateipfad: {e}"
        ))
    })?;
    let bytes = std::fs::read(&path)?;
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::InvalidInput(tr!(
            "{} isn't a PDF",
            "{} ist kein PDF",
            path.display()
        )));
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    let state = tauri::Manager::state::<AppState>(&app);
    let (written, id) = edit::add(&state.db, &crate::config_dir(), &name, &bytes, now())?;
    Ok(Some(Added {
        id,
        label: announced(&app, written),
    }))
}

/// Keeps a PDF waiting in the Add sheet as an other document.
#[tauri::command]
pub fn other_add_waiting(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
) -> AppResult<Added> {
    let pending = state.adds.get(&token).ok_or_else(|| {
        AppError::NotFound(tr!(
            "This PDF is no longer waiting to be added",
            "Dieses PDF wartet nicht mehr aufs Hinzufügen"
        ))
    })?;
    let (written, id) = edit::add(
        &state.db,
        &crate::config_dir(),
        &pending.original_name,
        &pending.bytes,
        now(),
    )?;
    state.adds.take(&token);
    Ok(Added {
        id,
        label: announced(&app, written),
    })
}

#[tauri::command]
pub fn other_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    change: OtherEdit,
) -> AppResult<Option<String>> {
    Ok(edit::edit(&state.db, &id, &change)?.map(|w| announced(&app, w)))
}

#[tauri::command]
pub fn other_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<String> {
    let written = edit::delete(&state.db, &id)?;
    Ok(announced(&app, written))
}
