//! `AppState`: what Tauri manages for the app's lifetime, opened from the app folder and the active profile's folder.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::db::Db;
use crate::documents::add::AddQueue;
use crate::error::AppResult;
use crate::launch::LaunchQueue;
use crate::lock::{MutexExt, RwLockExt};
use crate::notify::Notifier;
use crate::settings::{self, SettingsStore};
use crate::statements::import::ImportQueue;
use crate::store::json_file::JsonFile;

pub struct AppState {
    pub launches: LaunchQueue,
    pub settings: SettingsStore,
    /// The open profile's database; a switch replaces it (ADR 0061).
    db: RwLock<Arc<Db>>,
    pub notifier: Notifier,
    pub imports: ImportQueue,
    pub adds: AddQueue,
    /// Messages for the window from work done before it opened.
    pub notices: std::sync::Mutex<Vec<String>>,
}

impl AppState {
    pub fn open(config_dir: &Path, profile_dir: &Path) -> AppResult<Self> {
        Ok(Self {
            launches: LaunchQueue::default(),
            settings: SettingsStore::open(
                JsonFile::new(config_dir.join(settings::FILE_NAME)),
                JsonFile::new(profile_dir.join(settings::FILE_NAME)),
            ),
            db: RwLock::new(Arc::new(Db::open(profile_dir)?)),
            notifier: Notifier::default(),
            imports: ImportQueue::default(),
            adds: AddQueue::default(),
            notices: Default::default(),
        })
    }

    /// The open profile's database; a command takes it once and uses it and its `dir` together, so a switch
    /// in between never splits a row from its PDF.
    pub fn db(&self) -> Arc<Db> {
        self.db.read_unpoisoned().clone()
    }

    /// Makes `db` the open profile: its settings, its notices, and nothing waiting from the profile before.
    pub fn open_profile(&self, db: Db, notices: Vec<String>) {
        self.settings
            .open_profile(JsonFile::new(db.dir().join(settings::FILE_NAME)));
        *self.notices.lock_unpoisoned() = notices;
        self.imports.clear();
        self.adds.clear();
        *self.db.write_unpoisoned() = Arc::new(db);
    }

    /// The open profile's id, the name of its folder.
    pub fn profile_id(&self) -> String {
        self.db()
            .dir()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{self, AccountRow};
    use crate::db::{Change, Op, Source};
    use crate::documents::add::AddPending;
    use crate::documents::guess::Guess;
    use crate::settings::{Accent, Appearance};
    use crate::statements::import;
    use crate::statements::import_tests::open_with_accounts;
    use crate::test_support::statements::demo_month;
    use crate::test_support::temp_config_dir;

    fn profile_folder(root: &Path, id: &str) -> std::path::PathBuf {
        let dir = root.join("profiles").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn add_account(db: &Db, id: &str) {
        let row = AccountRow {
            id: id.into(),
            name: format!("Account {id}"),
            bank: "Example Sparkasse".into(),
            iban: "AT611904300234573201".into(),
            own: true,
            created_at: 1,
            statement_name: None,
        };
        db.write(
            Op::new(
                "Add account",
                vec![Change::insert(accounts::TABLE, &row).unwrap()],
            ),
            Source::Seed,
        )
        .unwrap();
    }

    fn account_ids(db: &Db) -> Vec<String> {
        db.read(accounts::rows)
            .unwrap()
            .into_iter()
            .map(|a| a.id)
            .collect()
    }

    fn write_settings(dir: &Path, json: &str) {
        std::fs::write(dir.join(settings::FILE_NAME), json).unwrap();
    }

    fn open_state() -> (tempfile::TempDir, std::path::PathBuf, AppState) {
        let root = temp_config_dir();
        let first = profile_folder(root.path(), "first");
        let state = AppState::open(root.path(), &first).unwrap();
        (root, first, state)
    }

    #[test]
    fn the_state_opens_the_database_and_names_the_profile_after_its_folder() {
        let (_root, first, state) = open_state();

        assert_eq!(state.db().dir(), first);
        assert_eq!(state.profile_id(), "first");
    }

    #[test]
    fn opening_a_profile_swaps_the_database_and_leaves_earlier_handles_on_the_old_one() {
        let (root, _first, state) = open_state();
        let before = state.db();
        add_account(&before, "old-account");
        let second = profile_folder(root.path(), "second");

        state.open_profile(Db::open(&second).unwrap(), vec![]);
        add_account(&state.db(), "new-account");

        assert_eq!(state.db().dir(), second);
        assert_eq!(state.profile_id(), "second");
        assert_eq!(account_ids(&state.db()), ["new-account"]);
        assert_eq!(account_ids(&before), ["old-account"]);
    }

    #[test]
    fn opening_a_profile_replaces_the_notices() {
        let (root, _first, state) = open_state();
        *state.notices.lock_unpoisoned() = vec!["from before".into()];
        let second = profile_folder(root.path(), "second");

        state.open_profile(
            Db::open(&second).unwrap(),
            vec!["Start tax year".into(), "Another".into()],
        );

        assert_eq!(
            *state.notices.lock_unpoisoned(),
            ["Start tax year", "Another"]
        );
    }

    #[test]
    fn opening_a_profile_takes_its_settings_from_that_folder_and_keeps_the_app_wide_ones() {
        let root = temp_config_dir();
        write_settings(
            root.path(),
            r#"{"accent":"teal","appearance":"dark","archiveDays":9}"#,
        );
        let first = profile_folder(root.path(), "first");
        write_settings(&first, r#"{"archiveDays":3,"accent":"red"}"#);
        let second = profile_folder(root.path(), "second");
        write_settings(
            &second,
            r#"{"archiveDays":5,"accent":"red","hideOwnTransfers":false}"#,
        );
        let state = AppState::open(root.path(), &first).unwrap();
        assert_eq!(state.settings.get().archive_days, 3);

        state.open_profile(Db::open(&second).unwrap(), vec![]);

        let settings = state.settings.get();
        assert_eq!(settings.archive_days, 5);
        assert!(!settings.hide_own_transfers);
        assert_eq!(settings.accent, Accent::Teal);
        assert_eq!(settings.appearance, Appearance::Dark);
    }

    #[test]
    fn opening_a_profile_empties_the_import_and_add_queues() {
        let (root, _first, state) = open_state();
        let (statements_dir, statements_db) = open_with_accounts();
        let pending = import::prepare(
            &statements_db,
            statements_dir.path(),
            demo_month(1, 2026, 9, 100_000).pdf(),
        )
        .unwrap();
        let import_token = state.imports.put(pending);
        let add_token = state.adds.put(AddPending {
            bytes: b"%PDF-1.4 demo".to_vec(),
            hash: "hash".into(),
            original_name: "invoice.pdf".into(),
            guess: Guess::default(),
        });
        let second = profile_folder(root.path(), "second");

        state.open_profile(Db::open(&second).unwrap(), vec![]);

        assert!(state.imports.take(&import_token).is_none());
        assert!(state.adds.take(&add_token).is_none());
    }
}
