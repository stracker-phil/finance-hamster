//! Requests the CLI sends over the socket, each mapped to its app action.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::cli::reply::StatusReport;
use crate::error::AppError;
use crate::ipc::{HandlerResult, IpcMessage};
use crate::profiles::cli::ProfileRequest;
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

/// The running app's side of `hamster profile`: a switch opens the profile in place; any other change refreshes
/// the switcher.
fn profile(app: AppHandle, request: ProfileRequest) -> HandlerResult {
    let root = crate::config_dir();
    let into_ipc = crate::ipc::IpcError::from;
    match request {
        ProfileRequest::Switch { name } => {
            let profile =
                crate::profiles::named(&crate::profiles::load(&root), &name).map_err(into_ipc)?;
            crate::profiles::switch::switch(&app, &profile.id).map_err(into_ipc)?;
            json(profile)
        }
        ProfileRequest::Add { name, switch } => {
            let profile = crate::profiles::add(&root, &name).map_err(into_ipc)?;
            if switch {
                crate::profiles::switch::switch(&app, &profile.id).map_err(into_ipc)?;
            } else {
                refresh(&app);
            }
            json(profile)
        }
        other => crate::profiles::cli::answer(&root, &other)
            .inspect(|_| refresh(&app))
            .map_err(into_ipc),
    }
}

fn refresh(app: &AppHandle) {
    if let Err(e) = app.emit(crate::events::PROFILES_CHANGED, ()) {
        log::warn!("Couldn't announce the changed profiles: {e}");
    }
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
        IpcMessage::Profile { request } => profile(app, request),
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
