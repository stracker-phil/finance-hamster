//! `AppState`: what Tauri manages for the app's lifetime, opened from the config folder.

use std::path::Path;

use crate::launch::LaunchQueue;
use crate::settings::{self, SettingsStore};
use crate::store::json_file::JsonFile;

pub struct AppState {
    pub launches: LaunchQueue,
    pub settings: SettingsStore,
}

impl AppState {
    pub fn open(config_dir: &Path) -> Self {
        Self {
            launches: LaunchQueue::default(),
            settings: SettingsStore::open(JsonFile::new(config_dir.join(settings::FILE_NAME))),
        }
    }
}
