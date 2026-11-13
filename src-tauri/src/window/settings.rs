//! The Settings window: a regular window of its own, built on first use and hidden on close, so it
//! reopens on the page it showed.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::events::OPEN_SETTINGS_EVENT;

fn url(page: Option<&str>) -> WebviewUrl {
    let query = page.map(|page| format!("?page={page}")).unwrap_or_default();
    WebviewUrl::App(format!("src/windows/settings/index.html{query}").into())
}

fn build(app: &AppHandle, page: Option<&str>) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, super::SETTINGS, url(page))
        .title("Settings")
        .inner_size(880.0, 720.0)
        .min_inner_size(760.0, 560.0)
        .maximizable(false)
        .visible(false)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(20.0, 22.0))
        .build()
}

fn find_or_build(app: &AppHandle, page: Option<&str>) -> Option<WebviewWindow> {
    if let Some(window) = app.get_webview_window(super::SETTINGS) {
        if let Some(page) = page
            && let Err(e) = window.emit_to(super::SETTINGS, OPEN_SETTINGS_EVENT, page)
        {
            log::warn!("Couldn't show the Settings page {page}: {e}");
        }
        return Some(window);
    }
    let window = build(app, page)
        .inspect_err(|e| log::warn!("Couldn't open Settings: {e}"))
        .ok()?;
    #[cfg(debug_assertions)]
    crate::dev::keep_painting_when_covered(&window);
    Some(window)
}

/// Shows Settings at `page` when given (else where it was) and brings it forward.
pub fn open(app: &AppHandle, page: Option<&str>) {
    let Some(window) = find_or_build(app, page) else {
        return;
    };
    if let Err(e) = window.show() {
        log::warn!("Couldn't show Settings: {e}");
    }
    super::bring_to_front(&window);
}
