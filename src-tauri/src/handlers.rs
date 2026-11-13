//! The Tauri commands the page may call, in one list.

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![crate::launch::take_launch_events, log_frontend_error]
}

/// Writes a page error (a crash, an e2e result) to the app's log, where scripts and bug reports find it.
#[tauri::command]
fn log_frontend_error(message: String) {
    log::error!("Page: {message}");
}
