//! `AppState`: what Tauri manages for the app's lifetime, opened from the app folder and the active profile's folder.

use std::path::Path;

use crate::db::Db;
use crate::documents::add::AddQueue;
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
            db: Db::open(profile_dir)?,
            notifier: Notifier::default(),
            imports: ImportQueue::default(),
            adds: AddQueue::default(),
            notices: Default::default(),
        })
    }
}
