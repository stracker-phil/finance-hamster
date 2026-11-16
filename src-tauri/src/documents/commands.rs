//! Adding documents as Tauri commands; dropped invoice PDFs arrive through `offer`.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use super::add::{self, AddInput, AddPreview};
use super::amounts::{self, Estimate};
use super::edit::{self, DocumentEdit};
use super::files;
use super::history::{self, HistoryEntry};
use super::lifecycle::{self, ArchivedDocument, DeletionView};
use super::list::{self, DocumentGroup};
use super::payments::LineChoice;
use super::vendors::{self, VendorsView};
use super::view::{self, DocumentView};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::events::ADD_READY_EVENT;
use crate::rules;
use crate::state::AppState;
use crate::statements::store as statements;
use crate::tr;

/// Reads the file, keeps it under a token and describes it for the Add document sheet.
fn read(app: &AppHandle, path: &Path) -> AppResult<AddPreview> {
    let state = app.state::<AppState>();
    let bytes = std::fs::read(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.pdf".into());
    let pending = add::prepare(&state.db, bytes, name)?;
    let token = state.adds.put(pending.clone());
    add::preview(
        &state.db,
        token,
        &pending,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Asks for an invoice PDF and reads it; null when the panel was cancelled.
#[tauri::command]
pub async fn document_add_choose(app: AppHandle) -> AppResult<Option<AddPreview>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(tr!("Add an Invoice PDF", "Rechnungs-PDF hinzufügen"))
        .add_filter(tr!("Invoice PDF", "Rechnungs-PDF"), &["pdf"])
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
    tauri::async_runtime::spawn_blocking(move || read(&app, &path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(Some)
}

/// Adds a read document as a draft; returns the label of the change.
#[tauri::command]
pub async fn document_add_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AddInput,
) -> AppResult<String> {
    let pending = state.adds.get(&input.token).ok_or_else(|| {
        AppError::NotFound(tr!(
            "This document is no longer waiting to be added",
            "Dieser Beleg wartet nicht mehr aufs Hinzufügen"
        ))
    })?;
    let written = add::commit(
        &state.db,
        &crate::profile_dir(),
        pending,
        &input,
        now(),
        Source::Ui,
    )?;
    state.adds.take(&input.token);
    announce(&app, &written);
    relocate(&app, &state);
    Ok(written.label)
}

/// A bank line as the Add document sheet's picker shows it, for an invoice added from that line.
#[tauri::command]
pub async fn document_line_choice(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<LineChoice>> {
    super::payments::choice_of(
        &state.db,
        &id,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub fn document_add_cancel(state: State<'_, AppState>, token: String) {
    state.adds.take(&token);
}

/// What a bank line counts as with an item, for the Add sheet; null without a line.
#[tauri::command]
pub async fn document_estimate(
    state: State<'_, AppState>,
    line_id: Option<String>,
    item_key: Option<String>,
) -> AppResult<Option<Estimate>> {
    let Some(id) = line_id else {
        return Ok(None);
    };
    let Some(line) = state.db.read(|c| statements::transaction_by_id(c, &id))? else {
        return Ok(None);
    };
    let ruleset = rules::ruleset(&state.db)?;
    let share = amounts::default_work_share(ruleset.as_ref(), item_key.as_deref());
    Ok(Some(amounts::estimate(
        ruleset.as_ref(),
        line.cents,
        item_key.as_deref(),
        share,
        state.settings.get().number_format,
    )))
}

/// Moves the drafts' files after a write and announces it.
pub fn relocate(app: &AppHandle, state: &AppState) {
    match files::relocate(&state.db, &crate::profile_dir()) {
        Ok(Some(written)) => announce(app, &written),
        Ok(None) => {}
        Err(e) => log::warn!("Couldn't move document files: {e}"),
    }
}

/// Payload of `add_ready`: a dropped invoice read and waiting, or why it can't be added.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AddReady {
    pub preview: Option<AddPreview>,
    pub error: Option<String>,
}

/// A dropped PDF that isn't a statement: offered in the Add document sheet.
pub fn offer(app: &AppHandle, path: &Path) {
    let payload = match read(app, path) {
        Ok(preview) => AddReady {
            preview: Some(preview),
            error: None,
        },
        Err(e) => AddReady {
            preview: None,
            error: Some(e.to_string()),
        },
    };
    if let Err(e) = app.emit(ADD_READY_EVENT, payload) {
        log::warn!("Couldn't offer a dropped document: {e}");
    }
}

#[tauri::command]
pub async fn document_view(state: State<'_, AppState>, id: String) -> AppResult<DocumentView> {
    view::document(
        &state.db,
        &id,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Changes one field of a draft; returns the label of the change, or null when nothing changed.
#[tauri::command]
pub async fn document_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    change: DocumentEdit,
) -> AppResult<Option<String>> {
    let written = edit::edit(
        &state.db,
        &id,
        &change,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    if let Some(written) = &written {
        announce(&app, written);
        relocate(&app, &state);
    }
    Ok(written.map(|w| w.label))
}

/// The bank lines the picker offers for a document: likely matches, or lines containing `query`.
#[tauri::command]
pub async fn document_line_options(
    state: State<'_, AppState>,
    id: String,
    query: String,
) -> AppResult<Vec<LineChoice>> {
    view::line_options(
        &state.db,
        &id,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn documents_list(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<Vec<DocumentGroup>> {
    list::documents(
        &state.db,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn vendors_view(state: State<'_, AppState>) -> AppResult<VendorsView> {
    vendors::view(&state.db)
}

/// One change to a vendor (Settings → Vendors); returns the label of the change.
#[derive(serde::Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum VendorChange {
    Rename { name: String },
    DefaultItem { key: Option<String> },
    MergeInto { into: String },
    Delete,
}

#[tauri::command]
pub async fn vendor_change(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    change: VendorChange,
) -> AppResult<String> {
    let written = match change {
        VendorChange::Rename { name } => vendors::rename(&state.db, &id, &name, Source::Ui)?,
        VendorChange::DefaultItem { key } => {
            vendors::set_default_item(&state.db, &id, key, Source::Ui)?
        }
        VendorChange::MergeInto { into } => vendors::merge(&state.db, &id, &into, Source::Ui)?,
        VendorChange::Delete => vendors::delete(&state.db, &id, Source::Ui)?,
    };
    announce(&app, &written);
    relocate(&app, &state);
    Ok(written.label)
}

/// A step in a document's life; returns the label of the change.
#[derive(serde::Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum DocumentStep {
    Finish,
    Unlock,
    Exclude { reason: String },
    Restore,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[tauri::command]
pub async fn document_step(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    step: DocumentStep,
) -> AppResult<String> {
    let db = &state.db;
    let written = match step {
        DocumentStep::Finish => lifecycle::finish(db, &id, now(), Source::Ui)?,
        DocumentStep::Unlock => lifecycle::unlock(db, &id, Source::Ui)?,
        DocumentStep::Exclude { reason } => {
            lifecycle::exclude(db, &id, &reason, now(), Source::Ui)?
        }
        DocumentStep::Restore => lifecycle::restore(db, &id, Source::Ui)?,
    };
    announce(&app, &written);
    relocate(&app, &state);
    Ok(written.label)
}

/// Deletes an excluded document for good; returns the label of the change.
#[tauri::command]
pub async fn document_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    reason: String,
    understood: bool,
) -> AppResult<String> {
    let written = lifecycle::delete(
        &state.db,
        &crate::profile_dir(),
        &id,
        &reason,
        understood,
        state.settings.get().archive_days,
        now(),
    )?;
    announce(&app, &written);
    Ok(written.label)
}

#[tauri::command]
pub async fn document_history(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<HistoryEntry>> {
    history::document_history(
        &state.db,
        &id,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn year_log(
    state: State<'_, AppState>,
    year: i32,
    query: String,
) -> AppResult<Vec<HistoryEntry>> {
    history::year_log(
        &state.db,
        year,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn archive_list(
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<Vec<ArchivedDocument>> {
    lifecycle::archive(
        &state.db,
        year,
        state.settings.get().archive_days,
        now(),
        DateOrder::system(),
    )
}

#[tauri::command]
pub async fn deletion_log(state: State<'_, AppState>) -> AppResult<Vec<DeletionView>> {
    lifecycle::deletion_log(&state.db, DateOrder::system())
}
