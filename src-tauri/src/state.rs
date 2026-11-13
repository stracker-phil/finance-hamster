//! `AppState`: what Tauri manages for the app's lifetime, opened from the config folder.

use std::path::Path;

use crate::launch::LaunchQueue;

pub struct AppState {
    pub launches: LaunchQueue,
}

impl AppState {
    pub fn open(_config_dir: &Path) -> Self {
        Self {
            launches: LaunchQueue::default(),
        }
    }
}
