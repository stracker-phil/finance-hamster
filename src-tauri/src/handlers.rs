//! The Tauri commands the page may call, in one list.

pub fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::launch::take_launch_events,
        log_frontend_error,
        crate::commands::handlers::list_commands,
        crate::commands::handlers::run_command,
        crate::settings::commands::get_settings,
        crate::settings::commands::update_settings,
        show_data_folder,
        data_folder,
    ]
}

/// The data folder with the home folder shortened to `~`, as Settings shows it.
#[tauri::command]
fn data_folder() -> String {
    let dir = crate::config_dir();
    match std::env::home_dir()
        .and_then(|home| dir.strip_prefix(home).ok().map(|rest| rest.to_path_buf()))
    {
        Some(rest) => format!("~/{}/", rest.display()),
        None => format!("{}/", dir.display()),
    }
}

/// Reveals the data folder in Finder (Settings → Privacy & Data).
#[tauri::command]
fn show_data_folder() -> crate::error::AppResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(crate::config_dir()).map_err(|e| {
        crate::error::AppError::Internal(format!("Couldn't show the data folder: {e}"))
    })
}

/// Writes a page error (a crash, an e2e result) to the app's log, where scripts and bug reports find it.
#[tauri::command]
fn log_frontend_error(message: String) {
    log::error!("Page: {message}");
}
