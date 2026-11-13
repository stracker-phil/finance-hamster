//! Adding documents as Tauri commands; dropped invoice PDFs arrive through `offer`.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use super::add::{self, AddInput, AddPreview};
use super::amounts::{self, Estimate};
use super::files;
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::events::ADD_READY_EVENT;
use crate::rules;
use crate::state::AppState;
use crate::statements::store as statements;

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
        .set_title("Add an Invoice PDF")
        .add_filter("Invoice PDF", &["pdf"])
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
    tauri::async_runtime::spawn_blocking(move || read(&app, &path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(Some)
}

/// Adds a read document as a draft; returns the label of the change.
#[tauri::command]
pub fn document_add_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AddInput,
) -> AppResult<String> {
    let pending = state.adds.get(&input.token).ok_or_else(|| {
        AppError::NotFound("This document is no longer waiting to be added".into())
    })?;
    let written = add::commit(
        &state.db,
        &crate::config_dir(),
        pending,
        &input,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    state.adds.take(&input.token);
    announce(&app, &written);
    relocate(&app, &state);
    Ok(written.label)
}

#[tauri::command]
pub fn document_add_cancel(state: State<'_, AppState>, token: String) {
    state.adds.take(&token);
}

/// What a bank line counts as with an item, for the Add sheet; null without a line.
#[tauri::command]
pub fn document_estimate(
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
    let year = line
        .value_date
        .get(..4)
        .and_then(|y| y.parse().ok())
        .unwrap_or(0);
    Ok(Some(amounts::estimate(
        ruleset.as_ref(),
        line.cents,
        item_key.as_deref(),
        share,
        year,
        state.settings.get().number_format,
    )))
}

/// Moves the drafts' files after a write and announces it.
pub fn relocate(app: &AppHandle, state: &AppState) {
    match files::relocate(&state.db, &crate::config_dir()) {
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
