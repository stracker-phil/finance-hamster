//! Requests the CLI sends over the socket, each mapped to its app action.

use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::cli::reply::StatusReport;
use crate::error::AppError;
use crate::ipc::{HandlerResult, IpcMessage};
use crate::{launch, window};

/// Lets the reply to `quit` reach the sender before the app exits.
const QUIT_DELAY: Duration = Duration::from_millis(200);

/// Nothing for the sender to print.
fn acknowledged() -> HandlerResult {
    Ok(serde_json::Value::Null)
}

fn json(value: impl Serialize) -> HandlerResult {
    serde_json::to_value(value).map_err(|e| AppError::from(e).into())
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
