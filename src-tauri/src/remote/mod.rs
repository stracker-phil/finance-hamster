//! Requests the CLI sends over the socket, each mapped to its app action.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::cli::reply::StatusReport;
use crate::error::AppError;
use crate::ipc::{HandlerResult, IpcMessage};
use crate::profiles::cli::ProfileRequest;
use crate::{launch, window};

/// Lets the reply to `quit` or a profile switch reach the sender before the app exits.
const QUIT_DELAY: Duration = Duration::from_millis(200);

/// Nothing for the sender to print.
fn acknowledged() -> HandlerResult {
    Ok(serde_json::Value::Null)
}

fn json(value: impl Serialize) -> HandlerResult {
    serde_json::to_value(value).map_err(|e| AppError::from(e).into())
}

/// The running app's side of `hamster profile`: a changed active profile restarts the app once the reply is
/// sent; any other change refreshes the switcher.
fn profile(app: AppHandle, request: &ProfileRequest) -> HandlerResult {
    let root = crate::config_dir();
    let answer =
        crate::profiles::cli::answer(&root, request).map_err(crate::ipc::IpcError::from)?;
    if crate::profiles::load(&root).active != app.state::<crate::state::AppState>().profile_id() {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(QUIT_DELAY).await;
            if let Err(e) = crate::instance::relaunch(&app) {
                log::warn!("Couldn't restart on the other profile: {e}");
            }
        });
    } else if let Err(e) = app.emit(crate::events::PROFILES_CHANGED, ()) {
        log::warn!("Couldn't announce the changed profiles: {e}");
    }
    Ok(answer)
}

fn quit_after_reply(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(QUIT_DELAY).await;
        app.exit(0);
    });
}

/// Handles one message from another process and returns what its sender prints.
pub async fn handle(app: AppHandle, message: IpcMessage) -> HandlerResult {
    log::info!("Received {} from another process", message.kind());
    match message {
        IpcMessage::Show { focus } => {
            if focus {
                window::show_main(&app);
            } else {
                window::show_main_in_background(&app);
            }
            acknowledged()
        }
        IpcMessage::Hide => {
            window::hide(&app, window::MAIN);
            acknowledged()
        }
        IpcMessage::Quit => {
            quit_after_reply(app);
            acknowledged()
        }
        IpcMessage::Status => json(StatusReport {
            version: app.package_info().version.to_string(),
            window_visible: window::main_visible(&app),
        }),
        IpcMessage::Run { id } => {
            crate::commands::run(&app, &id).map_err(crate::ipc::IpcError::from)?;
            acknowledged()
        }
        IpcMessage::Profile { request } => profile(app, &request),
        queued @ (IpcMessage::Activate { .. } | IpcMessage::Error { .. }) => {
            launch::queue(&app, queued);
            acknowledged()
        }
        #[cfg(debug_assertions)]
        IpcMessage::Eval { js } => {
            crate::dev::eval(&app, &js);
            acknowledged()
        }
    }
}
