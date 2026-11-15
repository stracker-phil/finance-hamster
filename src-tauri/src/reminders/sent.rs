//! The reminders already told, by key, in `reminders.json` beside the database. Not journaled: it records
//! notifications, not the owner's data.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::store::json_file::JsonFile;

pub const FILE_NAME: &str = "reminders.json";

#[derive(Serialize, Deserialize, Default)]
struct SentFile {
    sent: BTreeSet<String>,
}

fn file(config_dir: &Path) -> JsonFile {
    JsonFile::new(config_dir.join(FILE_NAME))
}

/// The keys told so far; none when the file is missing, and a broken file is moved aside.
pub fn load(config_dir: &Path) -> BTreeSet<String> {
    file(config_dir).load::<SentFile>().sent
}

pub fn save(config_dir: &Path, sent: &BTreeSet<String>) -> AppResult<()> {
    file(config_dir).save(&SentFile { sent: sent.clone() })
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
    fn a_broken_file_is_kept_beside_the_new_one_as_a_corrupt_copy() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), "not json at all").unwrap();

        assert_eq!(load(dir.path()), BTreeSet::new());

        assert!(!dir.path().join(FILE_NAME).exists());
        let kept: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let name = p.file_name().unwrap().to_string_lossy();
                name.starts_with("reminders.json.") && name.ends_with(".corrupt")
            })
            .collect();
        assert_eq!(kept.len(), 1);
        assert_eq!(
            std::fs::read_to_string(&kept[0]).unwrap(),
            "not json at all"
        );
    }

    #[test]
    fn saving_after_a_broken_file_starts_a_fresh_one() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), "not json at all").unwrap();
        assert_eq!(load(dir.path()), BTreeSet::new());

        save(dir.path(), &keys(&["filing:2025"])).unwrap();

        assert_eq!(load(dir.path()), keys(&["filing:2025"]));
    }

    #[test]
    fn the_saved_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_config_dir();
        save(dir.path(), &keys(&["filing:2025"])).unwrap();

        let mode = std::fs::metadata(dir.path().join(FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
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
