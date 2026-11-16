//! Opening a profile in the running app: at launch, and when switching to another one in place (ADR 0061).

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::{Db, Source};
use crate::error::AppResult;
use crate::events::{PROFILE_SWITCHED, SETTINGS_CHANGED_EVENT};
use crate::state::AppState;
use crate::store::json_file::create_private_dir;

/// What the app does to a profile it opens, before anything shows it: the demo data (debug build, only the
/// first profile), the built-in rules and the current tax year. Returns the notices for the window.
pub fn prepare(
    #[cfg_attr(not(debug_assertions), allow(unused_variables))] root: &Path,
    db: &Db,
) -> Vec<String> {
    #[cfg(debug_assertions)]
    seed_demo(root, db);
    let now = chrono::Utc::now().timestamp_millis();
    if let Err(e) = crate::rules::ensure_builtins(db, now, Source::App) {
        log::warn!("Couldn't add the built-in rules: {e}");
    }
    match crate::taxyears::start_on_launch(db, chrono::Local::now()) {
        Ok(written) => written
            .into_iter()
            .filter(|w| w.tables.iter().any(|t| t == crate::taxyears::TABLE))
            .map(|w| w.label)
            .collect(),
        Err(e) => {
            log::warn!("Couldn't start the tax years: {e}");
            Vec::new()
        }
    }
}

#[cfg(debug_assertions)]
fn seed_demo(root: &Path, db: &Db) {
    let id = db.dir().file_name().unwrap_or_default().to_string_lossy();
    if !super::is_first(root, &id) {
        return;
    }
    match crate::test_support::demo::seed_if_empty(db, db.dir(), chrono::Local::now().date_naive())
    {
        Ok(0) => {}
        Ok(count) => log::info!("Seeded the demo data ({count} ops)"),
        Err(e) => log::warn!("Couldn't seed the demo data: {e}"),
    }
}

#[cfg(test)]
#[path = "switch_tests.rs"]
mod tests;

/// Switches the running app to the profile `id` in place; the new database opens and `profiles.json` changes
/// before anything is swapped, so a failure keeps the open profile. What waited in the Import or Add sheet is
/// let go; its files stay where they were dropped.
pub fn switch(app: &AppHandle, id: &str) -> AppResult<()> {
    let state = app.state::<AppState>();
    if id == state.profile_id() {
        return Ok(());
    }
    let root = crate::config_dir();
    if !super::load(&root).profiles.iter().any(|p| p.id == id) {
        return Err(super::not_found());
    }
    let dir = super::dir(&root, id);
    create_private_dir(&dir)?;
    let db = Db::open(&dir)?;
    let notices = prepare(&root, &db);
    super::activate(&root, id)?;
    if let Err(e) = crate::preview::commands::hide(app) {
        log::warn!("Couldn't close the PDF preview: {e}");
    }
    state.open_profile(db, notices);
    app.state::<crate::integrity::LastCheck>().clear();
    log::info!("Switched to another profile");
    for event in [PROFILE_SWITCHED, SETTINGS_CHANGED_EVENT] {
        if let Err(e) = app.emit(event, ()) {
            log::warn!("Couldn't announce the switched profile: {e}");
        }
    }
    crate::integrity::check_in_background(app.clone());
    Ok(())
}
