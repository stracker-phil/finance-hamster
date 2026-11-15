//! The native preview as Tauri commands; the page's `PdfPreview` placeholder drives them.

use tauri::{AppHandle, Manager, State};

use super::PdfSource;
use crate::error::{AppError, AppResult};
use crate::platform::pdf_view::{self, Change, Frame};
use crate::state::AppState;
use crate::tr;
use crate::window::MAIN;

fn main_window(app: &AppHandle) -> AppResult<tauri::WebviewWindow> {
    app.get_webview_window(MAIN).ok_or_else(|| {
        AppError::Internal(tr!("The main window is gone", "Das Hauptfenster ist weg"))
    })
}

fn change(app: &AppHandle, change: Change) -> AppResult<()> {
    pdf_view::apply(&main_window(app)?, change)
}

/// Whether the preview is on screen; `scripts/dev/e2e.sh` checks that every overlay hides it.
#[tauri::command]
pub async fn pdf_view_visible(app: AppHandle) -> AppResult<bool> {
    pdf_view::visible(&main_window(&app)?)
}

#[tauri::command]
pub fn pdf_view_show(
    app: AppHandle,
    state: State<'_, AppState>,
    source: PdfSource,
    frame: Frame,
) -> AppResult<()> {
    let path = super::resolve(&state.db, &crate::config_dir(), &source)?;
    change(&app, Change::Show { path, frame })
}

#[tauri::command]
pub fn pdf_view_move(app: AppHandle, frame: Frame) -> AppResult<()> {
    change(&app, Change::Move(frame))
}

#[tauri::command]
pub fn pdf_view_hide(app: AppHandle) -> AppResult<()> {
    change(&app, Change::Hide)
}

/// An overlay opened over the page; the preview hides until it resumes.
#[tauri::command]
pub fn pdf_view_suspend(app: AppHandle) -> AppResult<()> {
    change(&app, Change::Suspend)
}

#[tauri::command]
pub fn pdf_view_resume(app: AppHandle) -> AppResult<()> {
    change(&app, Change::Resume)
}
