use std::path::Path;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::entries::{self, NewEntry};
use super::page::{self, YearPage};
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::taxrules::model::Ruleset;
use crate::taxyears;

/// `query` filters the rows (the header's filter field).
#[tauri::command]
pub fn year_page(state: State<'_, AppState>, year: i32, query: String) -> AppResult<YearPage> {
    page::page(
        &state.db,
        year,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

/// Answers one of the year's questions; returns the label of the change, or null when nothing changed.
#[tauri::command]
pub fn year_answer(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    question: String,
    value: String,
) -> AppResult<Option<String>> {
    let ruleset: Ruleset = serde_json::from_str(&year_row(&state, year)?.rules)?;
    let written = super::answers::answer(
        &state.db,
        &ruleset,
        year,
        &question,
        &value,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    if let Some(written) = &written {
        announce(&app, written);
    }
    Ok(written.map(|w| w.label))
}

fn year_row(state: &AppState, year: i32) -> AppResult<taxyears::TaxYearRow> {
    state
        .db
        .read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(format!("No tax year {year}")))
}

/// The year's row, refused once the year is closed: its entries are frozen.
fn open_year(state: &AppState, year: i32) -> AppResult<taxyears::TaxYearRow> {
    let row = year_row(state, year)?;
    if row.state == taxyears::YearState::Closed {
        return Err(AppError::InvalidInput(format!("Tax year {year} is closed")));
    }
    Ok(row)
}

/// Adds an amount or a trip; returns the label of the change.
#[tauri::command]
pub fn year_entry_add(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    entry: NewEntry,
) -> AppResult<String> {
    let ruleset: Ruleset = serde_json::from_str(&open_year(&state, year)?.rules)?;
    let written = entries::add(
        &state.db,
        &crate::config_dir(),
        &ruleset,
        year,
        &entry,
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    announce(&app, &written);
    Ok(written.label)
}

#[tauri::command]
pub fn year_entry_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    id: String,
) -> AppResult<String> {
    open_year(&state, year)?;
    let written = entries::delete(&state.db, year, &id, Source::Ui)?;
    announce(&app, &written);
    Ok(written.label)
}

/// Asks for an evidence PDF; its full path, or None when cancelled.
#[tauri::command]
pub async fn year_evidence_choose(app: AppHandle) -> AppResult<Option<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose the Evidence PDF")
        .add_filter("PDF", &["pdf"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx
        .await
        .map_err(|_| AppError::Io("The file panel closed without an answer".into()))?;
    chosen.map(chosen_path).transpose()
}

fn chosen_path(path: FilePath) -> AppResult<String> {
    path.into_path()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| AppError::Io(format!("The chosen file isn't a file path: {e}")))
}

/// Attaches the PDF at `path` to an amount; returns the label of the change.
#[tauri::command]
pub fn year_entry_attach(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    id: String,
    path: String,
) -> AppResult<String> {
    open_year(&state, year)?;
    let written = entries::attach(
        &state.db,
        &crate::config_dir(),
        year,
        &id,
        Path::new(&path),
        Source::Ui,
    )?;
    announce(&app, &written);
    Ok(written.label)
}

/// Opens an amount's evidence PDF in the default viewer.
#[tauri::command]
pub fn year_evidence_open(state: State<'_, AppState>, year: i32, id: String) -> AppResult<()> {
    let path = entries::evidence_path(&state.db, &crate::config_dir(), year, &id)?;
    tauri_plugin_opener::open_path(&path, None::<&str>)
        .map_err(|e| AppError::Internal(format!("Couldn't open {}: {e}", path.display())))
}
