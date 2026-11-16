//! `AppState`: what Tauri manages for the app's lifetime, opened from the app folder and the active profile's folder.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::db::Db;
use crate::documents::add::AddQueue;
use crate::error::AppResult;
use crate::launch::LaunchQueue;
use crate::lock::RwLockExt;
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

    /// The open profile's id, the name of its folder.
    pub fn profile_id(&self) -> String {
        self.db()
            .dir()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
