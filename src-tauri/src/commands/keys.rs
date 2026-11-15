//! The owner's own shortcuts in `keybindings.json`: `{ "commandId": "CmdOrCtrl+Shift+K" | null }`, null for a
//! command without a shortcut. A command not listed keeps its default.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use super::{COMMANDS, CommandDef, find, shortcut_label};
use crate::error::{AppError, AppResult};
use crate::store::json_file::JsonFile;

const FILE_NAME: &str = "keybindings.json";

pub type Overrides = BTreeMap<String, Option<String>>;

const MODIFIERS: [&str; 4] = ["CmdOrCtrl", "Shift", "Alt", "Ctrl"];

/// Ids the commands had before they took the family's colon form (`go.today` → `go:today`).
const RENAMED: &[(&str, &str)] = &[
    ("edit.undo", "edit:undo"),
    ("edit.redo", "edit:redo"),
    ("go.today", "go:today"),
    ("go.subscriptions", "go:subscriptions"),
    ("go.statements", "go:statements"),
    ("go.documents", "go:documents"),
    ("go.review", "go:review"),
    ("go.back", "go:back"),
    ("go.forward", "go:forward"),
    ("palette.open", "palette:open"),
    ("statement.import", "statement:import"),
    ("document.add", "document:add"),
    ("subscription.new", "subscription:new"),
    ("filter.focus", "filter:focus"),
    ("settings.open", "app:settings"),
];

fn file(config_dir: &Path) -> JsonFile {
    JsonFile::new(config_dir.join(FILE_NAME))
}

/// The overrides on disk, old ids carried over to their new names; none when the file is missing, and a
/// broken file is moved aside so the defaults still work.
pub fn load(config_dir: &Path) -> Overrides {
    let mut overrides: Overrides = file(config_dir).load();
    for (old, new) in RENAMED {
        if let Some(own) = overrides.remove(*old) {
            overrides.entry((*new).to_string()).or_insert(own);
        }
    }
    overrides
}

fn save(config_dir: &Path, overrides: &Overrides) -> AppResult<()> {
    file(config_dir).save(overrides)
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

fn holder(
    command: &CommandDef,
    accelerator: &str,
    overrides: &Overrides,
) -> Option<&'static CommandDef> {
    COMMANDS.iter().find(|c| {
        c.id != command.id
            && effective(c, overrides).is_some_and(|theirs| same(&theirs, accelerator))
    })
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
    if let Some(other) = holder(command, accelerator, overrides) {
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
    let mut overrides = load(config_dir);
    if let Some(accelerator) = accelerator {
        ensure_available(command, accelerator, &overrides)?;
    }
    assign(&mut overrides, command, accelerator);
    save(config_dir, &overrides)
}

fn assign(overrides: &mut Overrides, command: &CommandDef, accelerator: Option<&str>) {
    if accelerator == command.accelerator {
        overrides.remove(command.id);
    } else {
        overrides.insert(command.id.to_string(), accelerator.map(str::to_string));
    }
}

/// A shortcut another command already holds.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct KeyConflict {
    /// The shortcut as macOS draws it (`⇧⌘K`).
    pub shortcut: String,
    /// The title of the command holding it.
    pub holder: String,
}

/// The other command that holds `accelerator`, if any, so the page can ask before taking it.
pub fn conflict(config_dir: &Path, id: &str, accelerator: &str) -> AppResult<Option<KeyConflict>> {
    let command = command_for(id)?;
    Ok(
        holder(command, accelerator, &load(config_dir)).map(|other| KeyConflict {
            shortcut: shortcut_label(accelerator),
            holder: other.title.to_string(),
        }),
    )
}

/// Takes `accelerator` from the command that holds it and gives it to `id`, in one save.
pub fn reassign(config_dir: &Path, id: &str, accelerator: &str) -> AppResult<()> {
    let command = command_for(id)?;
    let mut overrides = load(config_dir);
    if let Some(other) = holder(command, accelerator, &overrides) {
        assign(&mut overrides, other, None);
    }
    ensure_available(command, accelerator, &overrides)?;
    assign(&mut overrides, command, Some(accelerator));
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

    fn corrupt_copies(dir: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let name = p.file_name().unwrap().to_string_lossy();
                name.starts_with("keybindings.json.") && name.ends_with(".corrupt")
            })
            .collect()
    }

    fn write_file(dir: &Path, value: Value) {
        std::fs::write(dir.join(FILE_NAME), value.to_string()).unwrap();
    }

    #[test]
    fn an_unreadable_file_counts_as_no_overrides_and_is_moved_aside() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();

        assert!(load(dir.path()).is_empty());

        assert!(!dir.path().join(FILE_NAME).exists());
        let kept = corrupt_copies(dir.path());
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read_to_string(&kept[0]).unwrap(), "{ not json");
    }

    #[test]
    fn changing_a_shortcut_works_after_an_unreadable_file_was_moved_aside() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();

        set(dir.path(), "go:today", Some("CmdOrCtrl+Alt+T")).unwrap();

        assert_eq!(
            file_of(dir.path()),
            json!({ "go:today": "CmdOrCtrl+Alt+T" })
        );
        let kept = corrupt_copies(dir.path());
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read_to_string(&kept[0]).unwrap(), "{ not json");
    }

    #[test]
    fn shortcuts_saved_under_the_old_dotted_ids_carry_over_to_the_colon_ids() {
        let dir = temp_config_dir();
        write_file(
            dir.path(),
            json!({ "go.today": "CmdOrCtrl+Alt+T", "settings.open": null, "go.review": "CmdOrCtrl+Shift+R" }),
        );

        let overrides = load(dir.path());

        assert_eq!(
            serde_json::to_value(&overrides).unwrap(),
            json!({ "go:today": "CmdOrCtrl+Alt+T", "app:settings": null, "go:review": "CmdOrCtrl+Shift+R" })
        );
        assert_eq!(
            shortcut(dir.path(), "go:today").as_deref(),
            Some("CmdOrCtrl+Alt+T")
        );
        assert_eq!(shortcut(dir.path(), "app:settings"), None);
    }

    #[test]
    fn an_entry_under_the_new_id_wins_over_one_under_the_old_id() {
        let dir = temp_config_dir();
        write_file(
            dir.path(),
            json!({ "go.today": "CmdOrCtrl+Alt+T", "go:today": "CmdOrCtrl+Alt+Y" }),
        );

        assert_eq!(
            serde_json::to_value(load(dir.path())).unwrap(),
            json!({ "go:today": "CmdOrCtrl+Alt+Y" })
        );
    }

    #[test]
    fn the_next_save_writes_carried_over_entries_under_their_new_ids() {
        let dir = temp_config_dir();
        write_file(dir.path(), json!({ "go.today": "CmdOrCtrl+Alt+T" }));

        set(dir.path(), "go:review", Some("CmdOrCtrl+Shift+R")).unwrap();

        assert_eq!(
            file_of(dir.path()),
            json!({ "go:today": "CmdOrCtrl+Alt+T", "go:review": "CmdOrCtrl+Shift+R" })
        );
    }

    #[test]
    fn every_old_id_leads_to_a_command_and_none_is_still_a_command() {
        for (old, new) in RENAMED {
            assert!(find(new).is_some(), "{new}");
            assert!(find(old).is_none(), "{old}");
        }
    }

    #[test]
    fn conflict_names_the_holder_of_a_taken_shortcut() {
        let dir = temp_config_dir();

        assert_eq!(
            conflict(dir.path(), "go:review", "CmdOrCtrl+K").unwrap(),
            Some(KeyConflict {
                shortcut: "⌘K".into(),
                holder: "Command Palette…".into()
            })
        );
        assert_eq!(
            conflict(dir.path(), "go:review", "Shift+CmdOrCtrl+Z")
                .unwrap()
                .map(|c| (c.shortcut, c.holder)),
            Some(("⇧⌘Z".into(), "Redo".into()))
        );
    }

    #[test]
    fn conflict_is_none_for_a_free_shortcut_and_for_the_commands_own() {
        let dir = temp_config_dir();

        assert_eq!(
            conflict(dir.path(), "go:review", "CmdOrCtrl+Alt+Q").unwrap(),
            None
        );
        assert_eq!(
            conflict(dir.path(), "palette:open", "CmdOrCtrl+K").unwrap(),
            None
        );
    }

    #[test]
    fn conflict_follows_the_owners_own_shortcuts() {
        let dir = temp_config_dir();
        set(dir.path(), "palette:open", None).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+Alt+R")).unwrap();

        assert_eq!(
            conflict(dir.path(), "go:today", "CmdOrCtrl+K").unwrap(),
            None
        );
        assert_eq!(
            conflict(dir.path(), "go:today", "CmdOrCtrl+Alt+R")
                .unwrap()
                .map(|c| c.holder),
            Some("Review Lines".to_string())
        );
    }

    #[test]
    fn reassigning_a_taken_shortcut_moves_it_in_one_save() {
        let dir = temp_config_dir();

        reassign(dir.path(), "go:review", "CmdOrCtrl+K").unwrap();

        assert_eq!(
            file_of(dir.path()),
            json!({ "palette:open": null, "go:review": "CmdOrCtrl+K" })
        );
        assert_eq!(
            shortcut(dir.path(), "go:review").as_deref(),
            Some("CmdOrCtrl+K")
        );
        assert_eq!(shortcut(dir.path(), "palette:open"), None);
    }

    #[test]
    fn reassigning_matches_the_holder_whatever_the_order_or_case_of_the_keys() {
        let dir = temp_config_dir();

        reassign(dir.path(), "go:review", "Shift+CmdOrCtrl+z").unwrap();

        assert_eq!(shortcut(dir.path(), "edit:redo"), None);
        assert_eq!(
            shortcut(dir.path(), "go:review").as_deref(),
            Some("Shift+CmdOrCtrl+z")
        );
    }

    #[test]
    fn reassigning_a_free_shortcut_just_sets_it() {
        let dir = temp_config_dir();

        reassign(dir.path(), "go:review", "CmdOrCtrl+Alt+Q").unwrap();

        assert_eq!(
            file_of(dir.path()),
            json!({ "go:review": "CmdOrCtrl+Alt+Q" })
        );
    }

    #[test]
    fn reassigning_an_invalid_shortcut_is_refused_and_changes_nothing() {
        let dir = temp_config_dir();

        assert_eq!(
            refusal(reassign(dir.path(), "go:review", "Shift+K")),
            "⇧K needs ⌘, ⌃ or ⌥ and one key"
        );

        assert!(load(dir.path()).is_empty());
        assert!(!dir.path().join(FILE_NAME).exists());
    }

    #[test]
    fn reassigning_an_unknown_command_is_not_found() {
        let dir = temp_config_dir();
        assert!(matches!(
            reassign(dir.path(), "no.such.command", "CmdOrCtrl+Alt+Q"),
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            conflict(dir.path(), "no.such.command", "CmdOrCtrl+Alt+Q"),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn a_new_shortcut_is_stored_and_replaces_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "go:today", Some("CmdOrCtrl+Alt+T")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "go:today": "CmdOrCtrl+Alt+T" })
        );
        assert_eq!(
            shortcut(dir.path(), "go:today").as_deref(),
            Some("CmdOrCtrl+Alt+T")
        );
        assert_eq!(
            shortcut(dir.path(), "go:subscriptions").as_deref(),
            Some("CmdOrCtrl+2")
        );
    }

    #[test]
    fn a_command_can_be_given_a_shortcut_it_never_had() {
        let dir = temp_config_dir();
        set(dir.path(), "go:review", Some("CmdOrCtrl+Shift+R")).unwrap();
        assert_eq!(
            shortcut(dir.path(), "go:review").as_deref(),
            Some("CmdOrCtrl+Shift+R")
        );
    }

    #[test]
    fn no_shortcut_is_stored_as_null_and_removes_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "edit:undo", None).unwrap();
        assert_eq!(file_of(dir.path()), json!({ "edit:undo": null }));
        assert_eq!(shortcut(dir.path(), "edit:undo"), None);
    }

    #[test]
    fn setting_the_default_again_removes_the_override() {
        let dir = temp_config_dir();
        set(dir.path(), "go:today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go:today", Some("CmdOrCtrl+1")).unwrap();
        assert_eq!(file_of(dir.path()), json!({}));
    }

    #[test]
    fn reset_brings_back_the_default_or_no_shortcut() {
        let dir = temp_config_dir();
        set(dir.path(), "go:today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+Shift+R")).unwrap();
        reset(dir.path(), "go:today").unwrap();
        reset(dir.path(), "go:review").unwrap();
        assert_eq!(file_of(dir.path()), json!({}));
        assert_eq!(
            shortcut(dir.path(), "go:today").as_deref(),
            Some("CmdOrCtrl+1")
        );
        assert_eq!(shortcut(dir.path(), "go:review"), None);
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
            let message = refusal(set(dir.path(), "go:review", Some(accelerator)));
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
            set(dir.path(), "go:review", Some(accelerator)).unwrap_or_else(|e| {
                panic!("{accelerator}: {e:?}");
            });
            assert_eq!(
                shortcut(dir.path(), "go:review").as_deref(),
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
                refusal(set(dir.path(), "go:review", Some(accelerator))),
                message,
                "{accelerator}"
            );
        }
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn a_shortcut_freed_by_another_override_can_be_taken() {
        let dir = temp_config_dir();
        set(dir.path(), "palette:open", None).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+K")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "palette:open": null, "go:review": "CmdOrCtrl+K" })
        );
    }

    #[test]
    fn a_default_an_override_moved_away_from_can_be_taken() {
        let dir = temp_config_dir();
        set(dir.path(), "go:today", Some("CmdOrCtrl+Alt+T")).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+1")).unwrap();
        assert_eq!(
            shortcut(dir.path(), "go:review").as_deref(),
            Some("CmdOrCtrl+1")
        );
    }

    #[test]
    fn a_command_may_keep_its_own_shortcut_when_set_again() {
        let dir = temp_config_dir();
        set(dir.path(), "go:review", Some("CmdOrCtrl+Shift+R")).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+Shift+R")).unwrap();
        assert_eq!(
            file_of(dir.path()),
            json!({ "go:review": "CmdOrCtrl+Shift+R" })
        );
    }

    #[test]
    fn reset_is_refused_while_another_command_uses_the_default() {
        let dir = temp_config_dir();
        set(dir.path(), "palette:open", None).unwrap();
        set(dir.path(), "go:review", Some("CmdOrCtrl+K")).unwrap();
        assert_eq!(
            refusal(reset(dir.path(), "palette:open")),
            "⌘K is already used by Review Lines"
        );
        assert_eq!(shortcut(dir.path(), "palette:open"), None);
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
