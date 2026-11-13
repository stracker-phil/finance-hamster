//! Hamster's modules and the `run` entry point.

pub mod accounts;
mod app;
pub mod cli;
pub mod commands;
pub mod date;
pub mod db;
#[cfg(debug_assertions)]
mod dev;
pub mod documents;
pub mod error;
pub mod events;
mod handlers;
mod instance;
pub mod ipc;
pub mod launch;
pub mod lock;
mod menu;
pub mod money;
pub mod notify;
pub mod platform;
pub mod preview;
mod remote;
pub mod review;
pub mod rules;
pub mod search;
pub mod settings;
pub mod state;
pub mod statements;
pub mod store;
pub mod subscriptions;
pub mod taxrules;
pub mod taxyears;
#[cfg(any(test, debug_assertions, feature = "test-util"))]
pub mod test_support;
pub mod today;
pub mod window;
pub mod year;

use std::path::PathBuf;

use crate::cli::{CliOutcome, CliRequest};
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::store::json_file::create_private_dir;

/// The bundle identifier and config folder name; the debug build's own keeps its data apart (ADR 0022).
const IDENTIFIER: &str = if cfg!(debug_assertions) {
    "com.philippstracker.hamster.debug"
} else {
    "com.philippstracker.hamster"
};
/// Debug builds go by their own name in the menu bar and the Dock.
pub const APP_NAME: &str = if cfg!(debug_assertions) {
    "Hamster (Debug)"
} else {
    "Hamster"
};
const SOCKET_NAME: &str = "hamster.sock";
/// Debug builds claim their own scheme (`tauri.debug.conf.json`), so links reach the installed app.
pub const LINK_SCHEME: &str = if cfg!(debug_assertions) {
    "hamster-debug"
} else {
    "hamster"
};

/// Tauri's `app_config_dir`, resolvable before the app starts; it holds all data (ADR 0012).
pub fn config_dir() -> PathBuf {
    let Some(home) = std::env::home_dir() else {
        fail("Cannot locate the home directory for Hamster's data");
    };
    home.join("Library/Application Support").join(IDENTIFIER)
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}

/// Quits the running debug app (its lock is released on exit), then removes its data.
#[cfg(debug_assertions)]
fn reset_demo(config_dir: &std::path::Path, socket: &std::path::Path) -> ! {
    if instance::ask(socket, &IpcMessage::Quit, false).is_some() {
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
    match test_support::demo::reset(config_dir) {
        Ok(()) => {
            println!("Removed the debug app's data; the next launch seeds the demo data");
            std::process::exit(0);
        }
        Err(e) => fail(&format!("Couldn't remove the debug app's data: {e}")),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_dir = config_dir();
    if let Err(e) = create_private_dir(&config_dir) {
        eprintln!("Cannot create {}: {e}", config_dir.display());
    }
    let socket = config_dir.join(SOCKET_NAME);

    let (message, log_level) = match cli::parse(std::env::args_os()) {
        CliOutcome::Exit { message, code } => {
            if code == 0 {
                print!("{message}");
            } else {
                eprint!("{message}");
            }
            std::process::exit(code);
        }
        CliOutcome::Run { hidden, log_level } => (IpcMessage::Activate { hidden }, log_level),
        CliOutcome::Ask(CliRequest { message, json }) => {
            match instance::ask(&socket, &message, json) {
                Some(code) => std::process::exit(code),
                None => fail("Hamster isn't running"),
            }
        }
        #[cfg(debug_assertions)]
        CliOutcome::ResetDemo => reset_demo(&config_dir, &socket),
        CliOutcome::Remote(CliRequest { message, json }) => {
            if let Some(code) = instance::ask(&socket, &message, json) {
                std::process::exit(code);
            }
            (message, None)
        }
    };

    let hidden = matches!(message, IpcMessage::Activate { hidden: true });
    let _instance_lock = instance::forward_or_lock(&config_dir, &socket, &message);
    let state = match AppState::open(&config_dir) {
        Ok(state) => state,
        Err(e) => fail(&format!("Hamster can't open its data: {e}")),
    };
    app::start(
        state,
        socket,
        message,
        log_level.unwrap_or_default(),
        hidden,
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use super::*;

    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn read_json(relative: &str) -> Value {
        let path = Path::new(MANIFEST_DIR).join(relative);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"))
    }

    fn release_config() -> Value {
        read_json("tauri.conf.json")
    }

    fn debug_config() -> Value {
        read_json("tauri.debug.conf.json")
    }

    fn active_config() -> Value {
        if cfg!(debug_assertions) {
            debug_config()
        } else {
            release_config()
        }
    }

    fn scheme_of(config: &Value) -> &str {
        config["plugins"]["deep-link"]["desktop"]["schemes"][0]
            .as_str()
            .expect("a deep-link scheme")
    }

    #[test]
    fn identifier_matches_the_tauri_config_of_this_build() {
        assert_eq!(IDENTIFIER, active_config()["identifier"].as_str().unwrap());
    }

    #[test]
    fn app_name_matches_the_tauri_config_of_this_build() {
        assert_eq!(APP_NAME, active_config()["productName"].as_str().unwrap());
    }

    #[test]
    fn link_scheme_matches_the_tauri_config_of_this_build() {
        assert_eq!(LINK_SCHEME, scheme_of(&active_config()));
    }

    #[test]
    fn the_release_config_uses_the_plain_names() {
        let config = release_config();
        assert_eq!(config["productName"], "Hamster");
        assert_eq!(config["identifier"], "com.philippstracker.hamster");
        assert_eq!(scheme_of(&config), "hamster");
    }

    #[test]
    fn the_debug_config_keeps_its_name_identifier_and_scheme_apart_from_release() {
        let release = release_config();
        let debug = debug_config();
        assert_eq!(debug["productName"], "Hamster (Debug)");
        assert_eq!(
            debug["identifier"].as_str().unwrap(),
            format!("{}.debug", release["identifier"].as_str().unwrap())
        );
        assert_eq!(scheme_of(&debug), "hamster-debug");
    }

    #[test]
    fn the_debug_config_overrides_nothing_but_name_identifier_and_plugins() {
        let debug = debug_config();
        let mut keys: Vec<&str> = debug
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["identifier", "plugins", "productName"]);
    }

    #[test]
    fn the_version_is_the_same_in_cargo_tauri_config_and_package_json() {
        let version = env!("CARGO_PKG_VERSION");
        assert_eq!(release_config()["version"].as_str().unwrap(), version);
        assert_eq!(
            read_json("../package.json")["version"].as_str().unwrap(),
            version
        );
    }

    #[test]
    fn the_version_has_three_numeric_parts() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3, "{parts:?}");
        for part in parts {
            assert!(part.parse::<u32>().is_ok(), "{part}");
        }
    }
}
