use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::close::{self, CloseCheck};
use super::entries::{self, NewEntry};
use super::page::{self, YearPage};
use super::report;
use crate::date::DateOrder;
use crate::db::Source;
use crate::db::commands::announce;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::taxrules::model::Ruleset;
use crate::taxyears;

/// `query` filters the rows (the header's filter field).
#[tauri::command]
pub async fn year_page(
    state: State<'_, AppState>,
    year: i32,
    query: String,
) -> AppResult<YearPage> {
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
pub async fn year_answer(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    question: String,
    value: String,
) -> AppResult<Option<String>> {
    let ruleset: Ruleset = serde_json::from_str(&open_year(&state, year)?.rules)?;
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

/// The year's categories with whether each shows while recording.
#[tauri::command]
pub async fn year_categories(
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<Vec<super::hidden::CategoryChoice>> {
    let ruleset: Ruleset = serde_json::from_str(&year_row(&state, year)?.rules)?;
    let hidden = state.db.read(|c| super::hidden::hidden(c, year))?;
    Ok(super::hidden::choices(&ruleset, &hidden))
}

/// Shows or hides a category while recording; the label of the change, or null when nothing changed.
#[tauri::command]
pub async fn year_set_category_shown(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    key: String,
    shown: bool,
) -> AppResult<Option<String>> {
    let ruleset: Ruleset = serde_json::from_str(&year_row(&state, year)?.rules)?;
    let written = super::hidden::set_shown(&state.db, &ruleset, year, &key, shown)?;
    if let Some(written) = &written {
        announce(&app, written);
    }
    Ok(written.map(|w| w.label))
}

fn year_row(state: &AppState, year: i32) -> AppResult<taxyears::TaxYearRow> {
    state.db.read(|c| taxyears::row(c, year))?.ok_or_else(|| {
        AppError::NotFound(crate::tr!("No tax year {year}", "Kein Steuerjahr {year}"))
    })
}

/// The year's row, refused once the year is closed: its entries are frozen.
fn open_year(state: &AppState, year: i32) -> AppResult<taxyears::TaxYearRow> {
    let row = year_row(state, year)?;
    if row.state == taxyears::YearState::Closed {
        return Err(AppError::InvalidInput(crate::tr!(
            "Tax year {year} is closed",
            "Das Steuerjahr {year} ist abgeschlossen"
        )));
    }
    Ok(row)
}

/// Adds an amount or a trip; returns the label of the change.
#[tauri::command]
pub async fn year_entry_add(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    entry: NewEntry,
) -> AppResult<String> {
    let ruleset: Ruleset = serde_json::from_str(&open_year(&state, year)?.rules)?;
    let written = entries::add(
        &state.db,
        &crate::profile_dir(),
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
pub async fn year_entry_delete(
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
        .set_title(crate::tr!("Choose the Evidence PDF", "Nachweis-PDF wählen"))
        .add_filter("PDF", &["pdf"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx.await.map_err(|_| {
        AppError::Io(crate::tr!(
            "The file panel closed without an answer",
            "Das Dateifenster wurde ohne Antwort geschlossen"
        ))
    })?;
    chosen.map(chosen_path).transpose()
}

fn chosen_path(path: FilePath) -> AppResult<String> {
    path.into_path()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| {
            AppError::Io(crate::tr!(
                "The chosen file isn't a file path: {e}",
                "Die gewählte Datei ist kein Dateipfad: {e}"
            ))
        })
}

/// Attaches the PDF at `path` to an amount; returns the label of the change.
#[tauri::command]
pub async fn year_entry_attach(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
    id: String,
    path: String,
) -> AppResult<String> {
    open_year(&state, year)?;
    let written = entries::attach(
        &state.db,
        &crate::profile_dir(),
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
pub async fn year_evidence_open(
    state: State<'_, AppState>,
    year: i32,
    id: String,
) -> AppResult<()> {
    let path = entries::evidence_path(&state.db, &crate::profile_dir(), year, &id)?;
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't open {}: {e}",
            "{} ließ sich nicht öffnen: {e}",
            path.display()
        ))
    })
}

fn stamp(prefix: &str) -> String {
    let today = chrono::Local::now().date_naive();
    format!(
        "{prefix} {}",
        crate::date::day_month_year(today, DateOrder::system())
    )
}

#[tauri::command]
pub async fn year_close_check(state: State<'_, AppState>, year: i32) -> AppResult<CloseCheck> {
    let page = page::page(
        &state.db,
        year,
        "",
        state.settings.get().number_format,
        DateOrder::system(),
    )?;
    Ok(close::check(&page))
}

/// Closes the year, storing its snapshot and summary PDF; returns the label of the change.
#[tauri::command]
pub async fn year_close(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<String> {
    let written = close::close(
        &state.db,
        &crate::profile_dir(),
        year,
        state.settings.get().number_format,
        DateOrder::system(),
        &stamp(&crate::tr!("Closed on", "Abgeschlossen am")),
        chrono::Utc::now().timestamp_millis(),
        Source::Ui,
    )?;
    announce(&app, &written);
    Ok(written.label)
}

#[tauri::command]
pub async fn year_reopen(
    app: AppHandle,
    state: State<'_, AppState>,
    year: i32,
) -> AppResult<String> {
    let written = close::reopen(&state.db, year, Source::Ui)?;
    announce(&app, &written);
    Ok(written.label)
}

/// Opens the newest snapshot's summary PDF in the default viewer.
#[tauri::command]
pub async fn year_open_snapshot(state: State<'_, AppState>, year: i32) -> AppResult<()> {
    let path = close::snapshot_file(&state.db, &crate::profile_dir(), year)?;
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| {
        AppError::Internal(crate::tr!(
            "Couldn't open {}: {e}",
            "{} ließ sich nicht öffnen: {e}",
            path.display()
        ))
    })
}

/// Asks where to save the year's summary PDF and writes it; false when cancelled.
#[tauri::command]
pub async fn year_export_summary(app: AppHandle, year: i32) -> AppResult<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(crate::tr!(
            "Export Summary PDF",
            "Zusammenfassung als PDF exportieren"
        ))
        .set_file_name(crate::tr!(
            "Tax year {year} summary.pdf",
            "Steuerjahr {year} Zusammenfassung.pdf"
        ))
        .add_filter("PDF", &["pdf"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let chosen = rx.await.map_err(|_| {
        AppError::Io(crate::tr!(
            "The file panel closed without an answer",
            "Das Dateifenster wurde ohne Antwort geschlossen"
        ))
    })?;
    let Some(path) = chosen.map(chosen_path).transpose()? else {
        return Ok(false);
    };
    let state = app.state::<AppState>();
    let page = page::page(
        &state.db,
        year,
        "",
        state.settings.get().number_format,
        DateOrder::system(),
    )?;
    std::fs::write(
        &path,
        report::summary_pdf(&page, &stamp(&crate::tr!("Exported on", "Exportiert am"))),
    )?;
    Ok(true)
}
