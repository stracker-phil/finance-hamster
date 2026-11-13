//! The app's windows. Closing one hides it; the app keeps running until it quits (ADR 0035).

pub mod navigation;
pub mod settings;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::events::WINDOW_OPENED_EVENT;

pub const MAIN: &str = "main";
/// Closing Settings hides it, like the main window.
pub const SETTINGS: &str = "settings";

/// Unminimizes and focuses the window; skipped with `HAMSTER_BACKGROUND=1` in debug builds so scripts/dev
/// never steal focus.
fn bring_to_front(window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    if crate::dev::background() {
        return;
    }
    // Failing only means the window is gone; there is nothing left to bring forward.
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Shows the hidden or minimized main window and brings it to the front.
pub fn show_main(app: &AppHandle) {
    show_main_in_background(app);
    if let Some(window) = app.get_webview_window(MAIN) {
        bring_to_front(&window);
    }
}

/// Shows the main window without bringing the app to the front; emits `WINDOW_OPENED_EVENT` when it was
/// hidden.
pub fn show_main_in_background(app: &AppHandle) {
    let opening = !main_visible(app);
    if let Some(window) = app.get_webview_window(MAIN)
        && let Err(e) = window.show()
    {
        log::warn!("Couldn't show the Hamster window: {e}");
    }
    if opening && let Err(e) = app.emit(WINDOW_OPENED_EVENT, ()) {
        log::warn!("Couldn't tell the page the window opened: {e}");
    }
}

pub fn main_visible(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

pub fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label)
        && let Err(e) = window.hide()
    {
        log::warn!("Couldn't hide the {label} window: {e}");
    }
}

/// Hides the main window or Settings on close and returns true, so the close doesn't proceed.
pub fn close_requested(app: &AppHandle, label: &str) -> bool {
    let ours = label == MAIN || label == SETTINGS;
    if ours {
        hide(app, label);
    }
    ours
}
