//! The command bus (ADR 0008): every action the menus, the palette, shortcuts and the CLI run, by id. Rust runs
//! the app-wide ones; the rest are page commands, sent to the main window to carry out.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::events::RUN_COMMAND_EVENT;
use crate::i18n::Text;
use crate::window;

pub mod handlers;
pub mod keys;

/// Who carries a command out.
#[derive(Clone, Copy)]
pub enum Runs {
    /// The main window: it is shown, then handles the id.
    Page,
    Settings,
    /// Rust alone, with the window left as it is.
    Rust(fn(&AppHandle) -> AppResult<()>),
}

pub struct CommandDef {
    pub id: &'static str,
    pub title: Text,
    /// The menu accelerator, in Tauri's notation (`CmdOrCtrl+Shift+K`).
    pub accelerator: Option<&'static str>,
    pub runs: Runs,
}

const fn page(id: &'static str, title: Text, accelerator: Option<&'static str>) -> CommandDef {
    CommandDef {
        id,
        title,
        accelerator,
        runs: Runs::Page,
    }
}

pub const COMMANDS: &[CommandDef] = &[
    page(
        "edit:undo",
        Text::new("Undo", "Widerrufen"),
        Some("CmdOrCtrl+Z"),
    ),
    page(
        "edit:redo",
        Text::new("Redo", "Wiederholen"),
        Some("CmdOrCtrl+Shift+Z"),
    ),
    page("go:today", Text::new("Today", "Heute"), Some("CmdOrCtrl+1")),
    page(
        "go:subscriptions",
        Text::new("Subscriptions", "Abos"),
        Some("CmdOrCtrl+2"),
    ),
    page(
        "go:statements",
        Text::new("Accounts", "Konten"),
        Some("CmdOrCtrl+3"),
    ),
    page(
        "go:documents",
        Text::new("Documents", "Dokumente"),
        Some("CmdOrCtrl+4"),
    ),
    page(
        "go:review",
        Text::new("Review Lines", "Buchungen prüfen"),
        None,
    ),
    page(
        "go:year:newest",
        Text::new("Newest Tax Year", "Neuestes Steuerjahr"),
        Some("CmdOrCtrl+5"),
    ),
    page(
        "go:year:second",
        Text::new("Second Newest Tax Year", "Zweitneuestes Steuerjahr"),
        Some("CmdOrCtrl+6"),
    ),
    page(
        "go:year:third",
        Text::new("Third Newest Tax Year", "Drittneuestes Steuerjahr"),
        Some("CmdOrCtrl+7"),
    ),
    page(
        "go:year:fourth",
        Text::new("Fourth Newest Tax Year", "Viertneuestes Steuerjahr"),
        Some("CmdOrCtrl+8"),
    ),
    page(
        "go:year:fifth",
        Text::new("Fifth Newest Tax Year", "Fünftneuestes Steuerjahr"),
        Some("CmdOrCtrl+9"),
    ),
    page("go:back", Text::new("Back", "Zurück"), Some("CmdOrCtrl+[")),
    page(
        "go:forward",
        Text::new("Forward", "Vorwärts"),
        Some("CmdOrCtrl+]"),
    ),
    page(
        "palette:open",
        Text::new("Command Palette…", "Befehlspalette …"),
        Some("CmdOrCtrl+K"),
    ),
    page(
        "statement:import",
        Text::new("Import Statement…", "Kontoauszug importieren …"),
        Some("CmdOrCtrl+I"),
    ),
    page(
        "document:add",
        Text::new("Add Invoice…", "Beleg hinzufügen …"),
        Some("CmdOrCtrl+O"),
    ),
    page(
        "payslip:add",
        Text::new("Add Payslip…", "Lohnzettel hinzufügen …"),
        None,
    ),
    page(
        "other:add",
        Text::new("Add Other Document…", "Anderes Dokument hinzufügen …"),
        None,
    ),
    page(
        "subscription:new",
        Text::new("New Subscription", "Neues Abo"),
        Some("CmdOrCtrl+N"),
    ),
    page(
        "filter:focus",
        Text::new("Filter This Page", "Diese Seite filtern"),
        Some("CmdOrCtrl+F"),
    ),
    page(
        "view:payslips",
        Text::new("Show or Hide Payslips", "Lohnzettel ein- oder ausblenden"),
        Some("CmdOrCtrl+Alt+L"),
    ),
    CommandDef {
        id: "app:settings",
        title: Text::new("Settings…", "Einstellungen …"),
        accelerator: Some("CmdOrCtrl+,"),
        runs: Runs::Settings,
    },
    CommandDef {
        id: "app:logs",
        title: Text::new("Open Log Folder", "Log-Ordner öffnen"),
        accelerator: None,
        runs: Runs::Rust(handlers::open_log_folder),
    },
    CommandDef {
        id: "app:data",
        title: Text::new("Show Data Folder", "Datenordner anzeigen"),
        accelerator: None,
        runs: Runs::Rust(handlers::show_data_folder),
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
    /// The owner changed this command's shortcut.
    pub custom: bool,
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

/// Every command with its shortcut, the owner's own ones applied.
pub fn list() -> Vec<CommandInfo> {
    let overrides = keys::load(&crate::config_dir());
    COMMANDS
        .iter()
        .map(|command| CommandInfo {
            id: command.id.to_string(),
            title: command.title.get().to_string(),
            shortcut: keys::effective(command, &overrides).map(|a| shortcut_label(&a)),
            custom: overrides.contains_key(command.id),
        })
        .collect()
}

pub fn run(app: &AppHandle, id: &str) -> AppResult<()> {
    let command = find(id).ok_or_else(|| AppError::NotFound(format!("No command {id}")))?;
    match command.runs {
        Runs::Settings => window::settings::open(app, None),
        Runs::Rust(handler) => handler(app)?,
        Runs::Page => {
            window::show_main(app);
            app.emit_to(window::MAIN, RUN_COMMAND_EVENT, command.id)
                .map_err(|e| {
                    AppError::Internal(crate::tr!(
                        "Couldn't send {id} to the window: {e}",
                        "{id} ließ sich nicht ans Fenster senden: {e}"
                    ))
                })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::with_lang;
    use crate::settings::Language;
    use std::collections::HashSet;

    #[test]
    fn titles_are_listed_in_the_current_language() {
        let title = |language, id: &str| {
            with_lang(language, list)
                .into_iter()
                .find(|c| c.id == id)
                .map(|c| c.title)
        };
        assert_eq!(title(Language::En, "edit:undo").as_deref(), Some("Undo"));
        assert_eq!(
            title(Language::De, "edit:undo").as_deref(),
            Some("Widerrufen")
        );
        assert_eq!(
            title(Language::De, "app:settings").as_deref(),
            Some("Einstellungen …")
        );
    }

    #[test]
    fn every_command_has_a_title_in_both_languages() {
        for command in COMMANDS {
            assert!(!command.title.en.is_empty(), "{}", command.id);
            assert!(!command.title.de.is_empty(), "{}", command.id);
        }
    }

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
            find("palette:open").map(|c| c.title.en),
            Some("Command Palette…")
        );
        assert!(find("no.such.command").is_none());
    }

    #[test]
    fn list_has_one_entry_per_command_with_its_shortcut_label() {
        let listed = with_lang(Language::En, list);
        assert_eq!(listed.len(), COMMANDS.len());
        for (info, command) in listed.iter().zip(COMMANDS) {
            assert_eq!(info.id, command.id);
            assert_eq!(info.title, command.title.en);
            assert_eq!(
                info.shortcut,
                command.accelerator.map(shortcut_label),
                "{}",
                command.id
            );
        }
        let palette = listed.iter().find(|c| c.id == "palette:open").unwrap();
        assert_eq!(palette.shortcut.as_deref(), Some("⌘K"));
    }

    #[test]
    fn undo_and_redo_are_listed_with_their_shortcuts() {
        let listed = with_lang(Language::En, list);
        let shortcut = |id: &str| {
            listed
                .iter()
                .find(|c| c.id == id)
                .map(|c| (c.title.as_str(), c.shortcut.as_deref()))
        };
        assert_eq!(shortcut("edit:undo"), Some(("Undo", Some("⌘Z"))));
        assert_eq!(shortcut("edit:redo"), Some(("Redo", Some("⇧⌘Z"))));
    }

    #[test]
    fn import_statement_is_listed_with_its_shortcut() {
        let listed = with_lang(Language::En, list);
        let import = listed.iter().find(|c| c.id == "statement:import").unwrap();
        assert_eq!(import.title, "Import Statement…");
        assert_eq!(import.shortcut.as_deref(), Some("⌘I"));
    }

    #[test]
    fn add_invoice_is_listed_with_its_shortcut() {
        let listed = with_lang(Language::En, list);
        let add = listed.iter().find(|c| c.id == "document:add").unwrap();
        assert_eq!(add.title, "Add Invoice…");
        assert_eq!(add.shortcut.as_deref(), Some("⌘O"));
    }

    #[test]
    fn the_documents_pages_and_adders_are_titled_in_both_languages() {
        for (id, english, german, shortcut) in [
            ("go:statements", "Accounts", "Konten", Some("⌘3")),
            ("go:documents", "Documents", "Dokumente", Some("⌘4")),
            (
                "document:add",
                "Add Invoice…",
                "Beleg hinzufügen …",
                Some("⌘O"),
            ),
            (
                "payslip:add",
                "Add Payslip…",
                "Lohnzettel hinzufügen …",
                None,
            ),
            (
                "other:add",
                "Add Other Document…",
                "Anderes Dokument hinzufügen …",
                None,
            ),
        ] {
            for (language, expected) in [(Language::En, english), (Language::De, german)] {
                let listed = with_lang(language, list);
                let command = listed.iter().find(|c| c.id == id).unwrap();

                assert_eq!(command.title, expected, "{id} {language:?}");
                assert_eq!(command.shortcut.as_deref(), shortcut, "{id}");
            }
            assert!(matches!(find(id).unwrap().runs, Runs::Page), "{id}");
        }
    }

    #[test]
    fn the_tax_year_pages_are_titled_in_both_languages_with_shortcuts_5_to_9() {
        for (id, english, german, shortcut) in [
            (
                "go:year:newest",
                "Newest Tax Year",
                "Neuestes Steuerjahr",
                "⌘5",
            ),
            (
                "go:year:second",
                "Second Newest Tax Year",
                "Zweitneuestes Steuerjahr",
                "⌘6",
            ),
            (
                "go:year:third",
                "Third Newest Tax Year",
                "Drittneuestes Steuerjahr",
                "⌘7",
            ),
            (
                "go:year:fourth",
                "Fourth Newest Tax Year",
                "Viertneuestes Steuerjahr",
                "⌘8",
            ),
            (
                "go:year:fifth",
                "Fifth Newest Tax Year",
                "Fünftneuestes Steuerjahr",
                "⌘9",
            ),
        ] {
            for (language, expected) in [(Language::En, english), (Language::De, german)] {
                let listed = with_lang(language, list);
                let command = listed.iter().find(|c| c.id == id).unwrap();

                assert_eq!(command.title, expected, "{id} {language:?}");
                assert_eq!(command.shortcut.as_deref(), Some(shortcut), "{id}");
            }
            assert!(matches!(find(id).unwrap().runs, Runs::Page), "{id}");
        }
    }

    #[test]
    fn review_lines_is_a_page_command_without_a_shortcut() {
        let listed = with_lang(Language::En, list);
        let review = listed.iter().find(|c| c.id == "go:review").unwrap();
        assert_eq!(review.title, "Review Lines");
        assert_eq!(review.shortcut, None);
    }

    #[test]
    fn the_folder_commands_are_listed_without_a_shortcut() {
        let listed = with_lang(Language::En, list);
        for (id, title) in [
            ("app:logs", "Open Log Folder"),
            ("app:data", "Show Data Folder"),
        ] {
            let command = listed.iter().find(|c| c.id == id).unwrap();
            assert_eq!(command.title, title, "{id}");
            assert_eq!(command.shortcut, None, "{id}");
        }
    }

    #[test]
    fn each_kind_of_command_is_run_by_its_own_runner() {
        for id in ["app:logs", "app:data"] {
            assert!(matches!(find(id).unwrap().runs, Runs::Rust(_)), "{id}");
        }
        assert!(matches!(find("app:settings").unwrap().runs, Runs::Settings));
        assert!(matches!(find("go:today").unwrap().runs, Runs::Page));
    }

    #[test]
    fn every_command_id_is_lower_case_words_joined_by_colons() {
        let word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c == '-');
        for command in COMMANDS {
            let (head, rest) = command.id.split_once(':').unwrap_or((command.id, ""));
            assert!(
                word(head) && head.chars().all(|c| c != '-') && rest.split(':').all(word),
                "{}",
                command.id
            );
        }
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
            custom: false,
        };
        assert_eq!(
            serde_json::to_value(&info).unwrap(),
            serde_json::json!({ "id": "x", "title": "X", "shortcut": null, "custom": false })
        );
    }
}
