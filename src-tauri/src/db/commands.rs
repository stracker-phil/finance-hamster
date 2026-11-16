//! Undo and Redo as Tauri commands; each announces what it changed.

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use super::Written;
use crate::error::AppResult;
use crate::events::DATA_CHANGED_EVENT;
use crate::state::AppState;

/// Payload of `data_changed`: the tables a write touched, so the page refetches only what shows them.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DataChanged {
    pub tables: Vec<String>,
}

pub fn announce(app: &AppHandle, written: &Written) {
    let payload = DataChanged {
        tables: written.tables.clone(),
    };
    if let Err(e) = app.emit(DATA_CHANGED_EVENT, payload) {
        log::warn!("Couldn't announce a change: {e}");
    }
}

/// The label of what was undone ("Mark as ANV"), or null when there was nothing to undo.
#[tauri::command]
pub async fn undo(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let written = state.db.undo()?;
    if let Some(written) = &written {
        announce(&app, written);
        crate::documents::commands::relocate(&app, &state);
    }
    Ok(written.map(|w| w.label))
}

#[tauri::command]
pub async fn redo(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let written = state.db.redo()?;
    if let Some(written) = &written {
        announce(&app, written);
        crate::documents::commands::relocate(&app, &state);
    }
    Ok(written.map(|w| w.label))
}
