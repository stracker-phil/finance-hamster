//! `AppState`: what Tauri manages for the app's lifetime, opened from the config folder.

use std::path::Path;

use crate::db::Db;
use crate::error::AppResult;
use crate::launch::LaunchQueue;
use crate::notify::Notifier;
use crate::settings::{self, SettingsStore};
use crate::statements::import::ImportQueue;
use crate::store::json_file::JsonFile;

pub struct AppState {
    pub launches: LaunchQueue,
    pub settings: SettingsStore,
    pub db: Db,
    pub notifier: Notifier,
    pub imports: ImportQueue,
}

impl AppState {
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        Ok(Self {
            launches: LaunchQueue::default(),
            settings: SettingsStore::open(JsonFile::new(config_dir.join(settings::FILE_NAME))),
            db: Db::open(config_dir)?,
            notifier: Notifier::default(),
            imports: ImportQueue::default(),
        })
    }
}
