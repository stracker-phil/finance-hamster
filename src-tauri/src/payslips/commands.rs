//! Payslips as Tauri commands: the year's panel, one payslip's form, adding from a file or a dropped PDF.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use super::edit::{self, PayslipEdit};
use super::view::{self, PayslipView, PayslipsYear};
use crate::db::Written;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::tr;

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn announced(app: &AppHandle, written: Written) -> String {
    announce(app, &written);
    written.label
}

/// Asks for a payslip PDF; its bytes, or None when cancelled.
async fn choose_pdf(app: &AppHandle) -> AppResult<Option<Vec<u8>>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(tr!("Choose the Payslip PDF", "Lohnzettel-PDF wählen"))
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
    let path: PathBuf = chosen.into_path().map_err(|e| {
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
    Ok(Some(bytes))
}

#[tauri::command]
pub async fn payslips_year(state: State<'_, AppState>, year: i32) -> AppResult<PayslipsYear> {
    view::year_view(&state.db(), year, state.settings.get().number_format)
}

/// Dokumente → Lohnzettel: every year with a payslip or an open tax year, twelve months each.
#[tauri::command]
pub async fn payslips_list(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<Vec<PayslipsYear>> {
    view::list(&state.db(), &query, state.settings.get().number_format)
}

#[tauri::command]
pub async fn payslip_view(state: State<'_, AppState>, id: String) -> AppResult<PayslipView> {
    view::payslip_view(&state.db(), &id, state.settings.get().number_format)
}

/// A new payslip's id and the label of the change.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipAdded {
    pub id: String,
    pub label: String,
}

/// Adds the month's payslip, typed by hand.
#[tauri::command]
pub async fn payslip_add(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    month: u32,
) -> AppResult<PayslipAdded> {
    let db = state.db();
    let (written, id) = edit::add(&db, db.dir(), year, month, None, now())?;
    Ok(PayslipAdded {
        id,
        label: announced(&app, written),
    })
}

/// Asks for a PDF and adds the month's payslip with it; None when cancelled.
#[tauri::command]
pub async fn payslip_add_pdf(
    app: AppHandle,
    year: i32,
    month: u32,
) -> AppResult<Option<PayslipAdded>> {
    let Some(bytes) = choose_pdf(&app).await? else {
        return Ok(None);
    };
    let state = tauri::Manager::state::<AppState>(&app);
    let db = state.db();
    let (written, id) = edit::add(&db, db.dir(), year, month, Some(&bytes), now())?;
    Ok(Some(PayslipAdded {
        id,
        label: announced(&app, written),
    }))
}

/// Asks for a PDF and attaches it to a payslip; None when cancelled.
#[tauri::command]
pub async fn payslip_attach(app: AppHandle, id: String) -> AppResult<Option<String>> {
    let Some(bytes) = choose_pdf(&app).await? else {
        return Ok(None);
    };
    let state = tauri::Manager::state::<AppState>(&app);
    let db = state.db();
    let written = edit::attach(&db, db.dir(), &id, &bytes)?;
    Ok(Some(announced(&app, written)))
}

/// The month a PDF waiting in the Add sheet is for, as `[year, month]`, when its top names one.
#[tauri::command]
pub async fn payslip_guess(
    state: State<'_, AppState>,
    token: String,
) -> AppResult<Option<(i32, u32)>> {
    Ok(state
        .adds
        .get(&token)
        .and_then(|pending| super::guess::month(&crate::documents::add::text_of(&pending.bytes))))
}

/// Adds a PDF waiting in the Add sheet as the month's payslip.
#[tauri::command]
pub async fn payslip_add_waiting(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    year: i32,
    month: u32,
) -> AppResult<PayslipAdded> {
    let pending = state.adds.get(&token).ok_or_else(|| {
        AppError::NotFound(tr!(
            "This PDF is no longer waiting to be added",
            "Dieses PDF wartet nicht mehr aufs Hinzufügen"
        ))
    })?;
    let db = state.db();
    let (written, id) = edit::add(&db, db.dir(), year, month, Some(&pending.bytes), now())?;
    state.adds.take(&token);
    Ok(PayslipAdded {
        id,
        label: announced(&app, written),
    })
}

/// The label of the change, or None when nothing changed.
#[tauri::command]
pub async fn payslip_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    change: PayslipEdit,
) -> AppResult<Option<String>> {
    Ok(edit::edit(&state.db(), &id, &change)?.map(|w| announced(&app, w)))
}

#[tauri::command]
pub async fn payslip_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<String> {
    let written = edit::delete(&state.db(), &id)?;
    Ok(announced(&app, written))
}
