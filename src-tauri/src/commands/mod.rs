//! The command bus (ADR 0008): every action the menus, the palette, shortcuts and the CLI run, by id. Rust runs
//! the app-wide ones; the rest are page commands, sent to the main window to carry out.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::events::RUN_COMMAND_EVENT;
use crate::window;

pub mod handlers;

/// Who carries a command out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runs {
    /// The main window: it is shown, then handles the id.
    Page,
    Settings,
}

pub struct CommandDef {
    pub id: &'static str,
    pub title: &'static str,
    /// The menu accelerator, in Tauri's notation (`CmdOrCtrl+Shift+K`).
    pub accelerator: Option<&'static str>,
    pub runs: Runs,
}

const fn page(
    id: &'static str,
    title: &'static str,
    accelerator: Option<&'static str>,
) -> CommandDef {
    CommandDef {
        id,
        title,
        accelerator,
        runs: Runs::Page,
    }
}

pub const COMMANDS: &[CommandDef] = &[
    page("go.today", "Today", Some("CmdOrCtrl+1")),
    page("go.subscriptions", "Subscriptions", Some("CmdOrCtrl+2")),
    page("go.statements", "Statements", Some("CmdOrCtrl+3")),
    page("go.documents", "Documents", Some("CmdOrCtrl+4")),
    page("go.back", "Back", Some("CmdOrCtrl+[")),
    page("go.forward", "Forward", Some("CmdOrCtrl+]")),
    page("palette.open", "Command Palette…", Some("CmdOrCtrl+K")),
    page("filter.focus", "Filter This Page", Some("CmdOrCtrl+F")),
    CommandDef {
        id: "settings.open",
        title: "Settings…",
        accelerator: Some("CmdOrCtrl+,"),
        runs: Runs::Settings,
    },
];

pub fn find(id: &str) -> Option<&'static CommandDef> {
    COMMANDS.iter().find(|command| command.id == id)
}

/// A command as the palette and the Shortcuts page list it.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct CommandInfo {
    pub id: String,
    pub title: String,
    /// The shortcut as macOS draws it (`⇧⌘K`).
    pub shortcut: Option<String>,
}

/// `CmdOrCtrl+Shift+K` → `⇧⌘K`: modifiers in macOS's order (⌃⌥⇧⌘), then the key.
pub fn shortcut_label(accelerator: &str) -> String {
    let parts: Vec<&str> = accelerator.split('+').collect();
    let (key, modifiers) = parts.split_last().unwrap_or((&"", &[]));
    let has = |names: &[&str]| modifiers.iter().any(|m| names.contains(m));
    let mut label = String::new();
    for (names, symbol) in [
        (&["Ctrl", "Control"][..], "⌃"),
        (&["Alt", "Option"][..], "⌥"),
        (&["Shift"][..], "⇧"),
        (&["CmdOrCtrl", "Cmd", "Command", "Super"][..], "⌘"),
    ] {
        if has(names) {
            label.push_str(symbol);
        }
    }
    label.push_str(key);
    label
}

pub fn list() -> Vec<CommandInfo> {
    COMMANDS
        .iter()
        .map(|command| CommandInfo {
            id: command.id.to_string(),
            title: command.title.to_string(),
            shortcut: command.accelerator.map(shortcut_label),
        })
        .collect()
}

pub fn run(app: &AppHandle, id: &str) -> AppResult<()> {
    let command = find(id).ok_or_else(|| AppError::NotFound(format!("No command {id}")))?;
    match command.runs {
        Runs::Settings => window::settings::open(app, None),
        Runs::Page => {
            window::show_main(app);
            app.emit_to(window::MAIN, RUN_COMMAND_EVENT, command.id)
                .map_err(|e| {
                    AppError::Internal(format!("Couldn't send {id} to the window: {e}"))
                })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn command_ids_are_unique() {
        let ids: HashSet<&str> = COMMANDS.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), COMMANDS.len());
    }

    #[test]
    fn accelerators_are_unique() {
        let accelerators: Vec<&str> = COMMANDS.iter().filter_map(|c| c.accelerator).collect();
        let unique: HashSet<&str> = accelerators.iter().copied().collect();
        assert_eq!(unique.len(), accelerators.len());
    }

    #[test]
    fn find_returns_the_command_by_id_and_none_for_an_unknown_id() {
        assert_eq!(
            find("palette.open").map(|c| c.title),
            Some("Command Palette…")
        );
        assert!(find("no.such.command").is_none());
    }

    #[test]
    fn list_has_one_entry_per_command_with_its_shortcut_label() {
        let listed = list();
        assert_eq!(listed.len(), COMMANDS.len());
        for (info, command) in listed.iter().zip(COMMANDS) {
            assert_eq!(info.id, command.id);
            assert_eq!(info.title, command.title);
            assert_eq!(
                info.shortcut,
                command.accelerator.map(shortcut_label),
                "{}",
                command.id
            );
        }
        let palette = listed.iter().find(|c| c.id == "palette.open").unwrap();
        assert_eq!(palette.shortcut.as_deref(), Some("⌘K"));
    }

    #[test]
    fn shortcut_labels_use_macos_symbols_in_macos_order() {
        for (accelerator, label) in [
            ("CmdOrCtrl+K", "⌘K"),
            ("CmdOrCtrl+Shift+K", "⇧⌘K"),
            ("Ctrl+Alt+Shift+CmdOrCtrl+X", "⌃⌥⇧⌘X"),
            ("CmdOrCtrl+[", "⌘["),
            ("CmdOrCtrl+,", "⌘,"),
            ("Shift+CmdOrCtrl+K", "⇧⌘K"),
            ("K", "K"),
        ] {
            assert_eq!(shortcut_label(accelerator), label, "{accelerator}");
        }
    }

    #[test]
    fn command_info_serializes_with_a_null_shortcut_when_there_is_none() {
        let info = CommandInfo {
            id: "x".into(),
            title: "X".into(),
            shortcut: None,
        };
        assert_eq!(
            serde_json::to_value(&info).unwrap(),
            serde_json::json!({ "id": "x", "title": "X", "shortcut": null })
        );
    }
}
