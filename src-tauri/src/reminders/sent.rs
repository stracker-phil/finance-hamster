//! The reminders already told, by key, in `reminders.json` beside the database. Not journaled: it records
//! notifications, not the owner's data.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

pub const FILE_NAME: &str = "reminders.json";

#[derive(Serialize, Deserialize, Default)]
struct SentFile {
    sent: BTreeSet<String>,
}

fn path(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE_NAME)
}

/// The keys told so far; none when the file is missing or unreadable.
pub fn load(config_dir: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(path(config_dir))
        .ok()
        .and_then(|text| serde_json::from_str::<SentFile>(&text).ok())
        .map(|file| file.sent)
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, sent: &BTreeSet<String>) -> AppResult<()> {
    let text = serde_json::to_string_pretty(&SentFile { sent: sent.clone() })?;
    std::fs::write(path(config_dir), text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    fn keys(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|k| (*k).to_string()).collect()
    }

    #[test]
    fn a_missing_file_has_told_nothing() {
        let dir = temp_config_dir();
        assert_eq!(load(dir.path()), BTreeSet::new());
    }

    #[test]
    fn an_unreadable_file_has_told_nothing() {
        for (content, why) in [
            ("not json at all", "garbage"),
            ("", "empty"),
            (r#"{"sent": 3}"#, "wrong shape"),
        ] {
            let dir = temp_config_dir();
            std::fs::write(dir.path().join(FILE_NAME), content).unwrap();

            assert_eq!(load(dir.path()), BTreeSet::new(), "{why}");
        }
    }

    #[test]
    fn saved_keys_load_again() {
        let dir = temp_config_dir();
        let told = keys(&["renewal:s1:2026-11-16", "filing:2025"]);

        save(dir.path(), &told).unwrap();

        assert_eq!(load(dir.path()), told);
    }

    #[test]
    fn saving_replaces_the_keys_told_before() {
        let dir = temp_config_dir();
        save(dir.path(), &keys(&["a", "b"])).unwrap();
        save(dir.path(), &keys(&["c"])).unwrap();

        assert_eq!(load(dir.path()), keys(&["c"]));
    }

    #[test]
    fn the_keys_are_kept_in_reminders_json_under_sent() {
        let dir = temp_config_dir();
        save(dir.path(), &keys(&["filing:2025"])).unwrap();

        let text = std::fs::read_to_string(dir.path().join("reminders.json")).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            serde_json::json!({ "sent": ["filing:2025"] })
        );
    }
}
