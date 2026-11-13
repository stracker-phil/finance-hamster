use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::cli::LogLevel;
use crate::handlers::handlers;
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::{ipc, launch, remote, window};

const MAX_LOG_BYTES: u128 = 5 * 1024 * 1024;

pub fn start(
    state: AppState,
    socket: PathBuf,
    initial: IpcMessage,
    log_level: LogLevel,
    hidden: bool,
) {
    tauri::Builder::default()
        .manage(state)
        .manage(crate::integrity::LastCheck::default())
        .plugin(log_plugin())
        .plugin(init_logging(log_level))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .register_asynchronous_uri_scheme_protocol(
            crate::preview::scheme::SCHEME,
            crate::preview::scheme::handle,
        )
        .plugin(window::navigation::guard())
        .plugin(tauri_plugin_deep_link::init())
        .menu(crate::menu::build)
        .on_menu_event(crate::menu::on_event)
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. }
                if window::close_requested(window.app_handle(), window.label()) =>
            {
                api.prevent_close();
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. })
                if window.label() == window::MAIN =>
            {
                crate::statements::commands::dropped(window.app_handle(), paths.clone());
            }
            _ => {}
        })
        .setup(move |app| {
            log::info!("Hamster {} started", env!("CARGO_PKG_VERSION"));
            #[cfg(debug_assertions)]
            seed_demo(app);
            add_builtin_rules(app);
            start_tax_years(app);
            check_files(app.handle().clone());
            start_reminders(app.handle().clone());
            take_initial(app.handle(), initial);
            #[cfg(debug_assertions)]
            apply_dev_frame(app);
            start_ipc(app.handle().clone(), &socket);
            // The window starts invisible (tauri.conf.json), so a hidden start never flashes it.
            if !hidden {
                window::show_main(app.handle());
            }
            Ok(())
        })
        .invoke_handler(handlers())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // Hiding the last window must not end the app; only Quit does (with an exit code).
            RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            RunEvent::Reopen { .. } => window::show_main(app),
            _ => {}
        });
}

/// A launch for the page waits for it; a request that started the app (`hamster show`) runs now, and its
/// failure shows in the window, as no terminal waits for the answer.
fn take_initial(app: &AppHandle, initial: IpcMessage) {
    if matches!(
        initial,
        IpcMessage::Activate { .. } | IpcMessage::Error { .. } | IpcMessage::Show { .. }
    ) {
        app.state::<AppState>().launches.push(initial);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = remote::handle(app.clone(), initial).await {
            launch::queue(&app, IpcMessage::Error { message: e.message });
        }
    });
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .max_file_size(MAX_LOG_BYTES)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        // The dispatch caps every later `log::set_max_level`, so it lets everything through; `init_logging`
        // sets the effective level.
        .level(log::LevelFilter::Trace)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .level_for("tauri_runtime_wry", log::LevelFilter::Info)
        .build()
}

/// Runs after `log_plugin`'s setup, which raises the max level to `Trace`, and applies the effective level.
fn init_logging(level: LogLevel) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("log-level")
        .setup(move |_, _| {
            log::set_max_level(level.into());
            Ok(())
        })
        .build()
}

fn add_builtin_rules(app: &tauri::App) {
    let state = app.state::<AppState>();
    let now = chrono::Utc::now().timestamp_millis();
    if let Err(e) = crate::rules::ensure_builtins(&state.db, now, crate::db::Source::App) {
        log::warn!("Couldn't add the built-in rules: {e}");
    }
}

/// Checks the stored PDFs in the background; a problem shows as the main window's banner.
fn check_files(app: AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        match crate::integrity::verify(
            &state.db,
            &crate::config_dir(),
            chrono::Utc::now().timestamp_millis(),
        ) {
            Ok(check) => {
                if !check.problems.is_empty() {
                    log::warn!(
                        "{} stored files are missing or changed",
                        check.problems.len()
                    );
                }
                app.state::<crate::integrity::LastCheck>().set(check);
                let _ = tauri::Emitter::emit(&app, crate::events::FILES_CHECKED, ());
            }
            Err(e) => log::warn!("Couldn't check the stored files: {e}"),
        }
    });
}

/// How often the running app looks for due reminders.
const REMINDER_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Tells due reminders now and every half hour while the app runs (ADR 0042).
fn start_reminders(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            let state = app.state::<AppState>();
            let told = crate::reminders::tell(
                &state.db,
                &crate::config_dir(),
                &state.settings.get(),
                chrono::Local::now().date_naive(),
                crate::date::DateOrder::system(),
                |notification| state.notifier.deliver(&app, notification),
            );
            match told {
                Ok(0) => {}
                Ok(n) => log::info!("Told {n} reminders"),
                Err(e) => log::warn!("Couldn't tell the reminders: {e}"),
            }
            std::thread::sleep(REMINDER_INTERVAL);
        }
    });
}

fn start_tax_years(app: &tauri::App) {
    let state = app.state::<AppState>();
    let started =
        crate::taxyears::start_on_launch(&state.db, &crate::config_dir(), chrono::Local::now());
    if let Err(e) = started {
        log::warn!("Couldn't start the tax years: {e}");
    }
}

#[cfg(debug_assertions)]
fn seed_demo(app: &tauri::App) {
    let state = app.state::<AppState>();
    match crate::test_support::demo::seed_if_empty(
        &state.db,
        &crate::config_dir(),
        chrono::Local::now().date_naive(),
    ) {
        Ok(0) => {}
        Ok(count) => log::info!("Seeded the demo data ({count} ops)"),
        Err(e) => log::warn!("Couldn't seed the demo data: {e}"),
    }
}

#[cfg(debug_assertions)]
fn apply_dev_frame(app: &tauri::App) {
    let Some(window) = app.get_webview_window(window::MAIN) else {
        return;
    };
    crate::dev::keep_painting_when_covered(&window);
    let Some((x, y, w, h)) = crate::dev::window_frame() else {
        return;
    };
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
}

/// Without the socket the app still runs, only without single-instance handoff.
fn start_ipc(handle: AppHandle, socket: &Path) {
    let listener = match ipc::bind(socket) {
        Ok(listener) => listener,
        Err(e) => {
            log::warn!("Cannot listen on {}: {e}", socket.display());
            return;
        }
    };
    tauri::async_runtime::spawn(ipc::serve(listener, move |message| {
        remote::handle(handle.clone(), message)
    }));
}
