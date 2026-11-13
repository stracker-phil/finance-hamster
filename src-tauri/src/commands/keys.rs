//! The owner's own shortcuts in `keybindings.json`: `{ "commandId": "CmdOrCtrl+Shift+K" | null }`, null for a
//! command without a shortcut. A command not listed keeps its default.

use std::collections::BTreeMap;
use std::path::Path;

use super::{COMMANDS, CommandDef, find, shortcut_label};
use crate::error::{AppError, AppResult};

const FILE_NAME: &str = "keybindings.json";

pub type Overrides = BTreeMap<String, Option<String>>;

const MODIFIERS: [&str; 4] = ["CmdOrCtrl", "Shift", "Alt", "Ctrl"];

/// The overrides on disk; none when the file is missing or unreadable, so the defaults still work.
pub fn load(config_dir: &Path) -> Overrides {
    std::fs::read_to_string(config_dir.join(FILE_NAME))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The overrides to change; a file that exists but can't be read is refused rather than overwritten.
fn load_for_change(config_dir: &Path) -> AppResult<Overrides> {
    match std::fs::read_to_string(config_dir.join(FILE_NAME)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Overrides::new()),
        Err(e) => Err(e.into()),
        Ok(text) => serde_json::from_str(&text).map_err(|_| {
            AppError::InvalidInput(format!("{FILE_NAME} can't be read; fix or remove it first"))
        }),
    }
}

fn save(config_dir: &Path, overrides: &Overrides) -> AppResult<()> {
    std::fs::write(
        config_dir.join(FILE_NAME),
        serde_json::to_string_pretty(overrides)?,
    )?;
    Ok(())
}

/// None when the command has no shortcut.
pub fn effective(command: &CommandDef, overrides: &Overrides) -> Option<String> {
    match overrides.get(command.id) {
        Some(own) => own.clone(),
        None => command.accelerator.map(str::to_string),
    }
}

/// A shortcut needs ⌘, ⌃ or ⌥ and one key; Shift alone would take a typed letter.
fn valid(accelerator: &str) -> bool {
    let parts: Vec<&str> = accelerator.split('+').collect();
    parts.split_last().is_some_and(|(key, modifiers)| {
        let key_ok = !key.is_empty()
            && !MODIFIERS.contains(key)
            && (key.chars().count() == 1 || key.chars().all(|c| c.is_ascii_alphanumeric()));
        let modifiers_ok = modifiers.iter().all(|m| MODIFIERS.contains(m))
            && modifiers.iter().any(|m| *m != "Shift");
        key_ok && modifiers_ok
    })
}

fn same(a: &str, b: &str) -> bool {
    let normal = |s: &str| {
        let mut parts: Vec<String> = s.split('+').map(str::to_lowercase).collect();
        parts.sort();
        parts
    };
    normal(a) == normal(b)
}

fn command_for(id: &str) -> AppResult<&'static CommandDef> {
    find(id).ok_or_else(|| AppError::NotFound(format!("No command {id}")))
}

/// A well-formed shortcut that no other command uses.
fn ensure_available(
    command: &CommandDef,
    accelerator: &str,
    overrides: &Overrides,
) -> AppResult<()> {
    if !valid(accelerator) {
        return Err(AppError::InvalidInput(format!(
            "{} needs ⌘, ⌃ or ⌥ and one key",
            shortcut_label(accelerator)
        )));
    }
    if let Some(other) = COMMANDS.iter().find(|c| {
        c.id != command.id
            && effective(c, overrides).is_some_and(|theirs| same(&theirs, accelerator))
    }) {
        return Err(AppError::InvalidInput(format!(
            "{} is already used by {}",
            shortcut_label(accelerator),
            other.title
        )));
    }
    Ok(())
}

/// Gives `id` its own shortcut (None: no shortcut); refuses one another command already uses.
pub fn set(config_dir: &Path, id: &str, accelerator: Option<&str>) -> AppResult<()> {
    let command = command_for(id)?;
    let mut overrides = load_for_change(config_dir)?;
    if let Some(accelerator) = accelerator {
        ensure_available(command, accelerator, &overrides)?;
    }
    let own = accelerator.map(str::to_string);
    if own.as_deref() == command.accelerator {
        overrides.remove(id);
    } else {
        overrides.insert(id.to_string(), own);
    }
    save(config_dir, &overrides)
}

/// Back to the default shortcut; refuses when another command now uses it.
pub fn reset(config_dir: &Path, id: &str) -> AppResult<()> {
    set(config_dir, id, command_for(id)?.accelerator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use serde_json::{Value, json};

    fn file_of(dir: &Path) -> Value {
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn shortcut(dir: &Path, id: &str) -> Option<String> {
        effective(find(id).unwrap(), &load(dir))
    }

    fn refusal(result: AppResult<()>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn without_a_file_every_command_keeps_its_default() {
        let dir = temp_config_dir();
        assert!(load(dir.path()).is_empty());
        for command in COMMANDS {
            assert_eq!(
                effective(command, &load(dir.path())),
                command.accelerator.map(str::to_string),
                "{}",
                command.id
            );
        }
    }

    #[test]
    fn an_unreadable_file_counts_as_no_overrides() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn changing_shortcuts_is_refused_over_an_unreadable_file_and_leaves_it_untouched() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, "{ not json").unwrap();
        let message = "keybindings.json can't be read; fix or remove it first";
        assert_eq!(
            refusal(set(dir.path(), "go.today", Some("CmdOrCtrl+Alt+T"))),
            message
        );
        assert_eq!(refusal(set(dir.path(), "go.today", None)), message);
        assert_eq!(refusal(reset(dir.path(), "go.today")), message);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
    }

    #[test]
    fn a_new_shortcut_is_stored_and_replaces_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "go.today", Some("CmdOrCtrl+Alt+T")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "go.today": "CmdOrCtrl+Alt+T" })
        );
        assert_eq!(
            shortcut(dir.path(), "go.today").as_deref(),
            Some("CmdOrCtrl+Alt+T")
        );
        assert_eq!(
            shortcut(dir.path(), "go.subscriptions").as_deref(),
            Some("CmdOrCtrl+2")
        );
    }

    #[test]
    fn a_command_can_be_given_a_shortcut_it_never_had() {
        let dir = temp_config_dir();
        set(dir.path(), "go.review", Some("CmdOrCtrl+Shift+R")).unwrap();
        assert_eq!(
            shortcut(dir.path(), "go.review").as_deref(),
            Some("CmdOrCtrl+Shift+R")
        );
    }

    #[test]
    fn no_shortcut_is_stored_as_null_and_removes_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "edit.undo", None).unwrap();
        assert_eq!(file_of(dir.path()), json!({ "edit.undo": null }));
        assert_eq!(shortcut(dir.path(), "edit.undo"), None);
    }

    #[test]
    fn setting_the_default_again_removes_the_override() {
        let dir = temp_config_dir();
        set(dir.path(), "go.today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go.today", Some("CmdOrCtrl+1")).unwrap();
        assert_eq!(file_of(dir.path()), json!({}));
    }

    #[test]
    fn reset_brings_back_the_default_or_no_shortcut() {
        let dir = temp_config_dir();
        set(dir.path(), "go.today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go.review", Some("CmdOrCtrl+Shift+R")).unwrap();
        reset(dir.path(), "go.today").unwrap();
        reset(dir.path(), "go.review").unwrap();
        assert_eq!(file_of(dir.path()), json!({}));
        assert_eq!(
            shortcut(dir.path(), "go.today").as_deref(),
            Some("CmdOrCtrl+1")
        );
        assert_eq!(shortcut(dir.path(), "go.review"), None);
    }

    #[test]
    fn a_shortcut_needs_a_modifier_other_than_shift_and_one_key() {
        let dir = temp_config_dir();
        for accelerator in [
            "K",
            "Shift+K",
            "CmdOrCtrl",
            "CmdOrCtrl+Shift",
            "CmdOrCtrl+",
            "",
            "Hyper+K",
            "CmdOrCtrl+K+L",
        ] {
            let message = refusal(set(dir.path(), "go.review", Some(accelerator)));
            assert_eq!(
                message,
                format!(
                    "{} needs ⌘, ⌃ or ⌥ and one key",
                    shortcut_label(accelerator)
                ),
                "{accelerator:?}"
            );
        }
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn accepted_shortcuts_combine_modifiers_with_one_key() {
        let dir = temp_config_dir();
        for accelerator in [
            "CmdOrCtrl+Shift+R",
            "Alt+R",
            "Ctrl+R",
            "Ctrl+Alt+Shift+CmdOrCtrl+R",
            "CmdOrCtrl+F5",
            "CmdOrCtrl+;",
        ] {
            set(dir.path(), "go.review", Some(accelerator)).unwrap_or_else(|e| {
                panic!("{accelerator}: {e:?}");
            });
            assert_eq!(
                shortcut(dir.path(), "go.review").as_deref(),
                Some(accelerator)
            );
        }
    }

    #[test]
    fn a_shortcut_another_command_uses_is_refused_whatever_the_order_or_case() {
        let dir = temp_config_dir();
        for (accelerator, message) in [
            ("CmdOrCtrl+K", "⌘K is already used by Command Palette…"),
            ("CmdOrCtrl+k", "⌘k is already used by Command Palette…"),
            ("Shift+CmdOrCtrl+Z", "⇧⌘Z is already used by Redo"),
        ] {
            assert_eq!(
                refusal(set(dir.path(), "go.review", Some(accelerator))),
                message,
                "{accelerator}"
            );
        }
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn a_shortcut_freed_by_another_override_can_be_taken() {
        let dir = temp_config_dir();
        set(dir.path(), "palette.open", None).unwrap();
        set(dir.path(), "go.review", Some("CmdOrCtrl+K")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "palette.open": null, "go.review": "CmdOrCtrl+K" })
        );
    }

    #[test]
    fn a_default_an_override_moved_away_from_can_be_taken() {
        let dir = temp_config_dir();
        set(dir.path(), "go.today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go.review", Some("CmdOrCtrl+1")).unwrap();
        assert_eq!(
            shortcut(dir.path(), "go.review").as_deref(),
            Some("CmdOrCtrl+1")
        );
    }

    #[test]
    fn a_command_may_keep_its_own_shortcut_when_set_again() {
        let dir = temp_config_dir();
        set(dir.path(), "go.review", Some("CmdOrCtrl+Shift+R")).unwrap();
        set(dir.path(), "go.review", Some("CmdOrCtrl+Shift+R")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "go.review": "CmdOrCtrl+Shift+R" })
        );
    }

    #[test]
    fn reset_is_refused_while_another_command_uses_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "palette.open", None).unwrap();
        set(dir.path(), "go.review", Some("CmdOrCtrl+K")).unwrap();
        assert_eq!(
            refusal(reset(dir.path(), "palette.open")),
            "⌘K is already used by Review Lines"
        );
        assert_eq!(shortcut(dir.path(), "palette.open"), None);
    }

    #[test]
    fn an_unknown_command_is_not_found() {
        let dir = temp_config_dir();
        assert!(matches!(
            set(dir.path(), "no.such.command", Some("CmdOrCtrl+Alt+Q")),
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            reset(dir.path(), "no.such.command"),
            Err(AppError::NotFound(_))
        ));
    }
}
