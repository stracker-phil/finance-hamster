//! Profiles: each person's data in its own folder under `profiles/`, listed in `profiles.json` in the app
//! folder. The app opens the active profile at launch; a switch starts the app anew on another (ADR 0060).

pub mod cli;
pub mod commands;
pub mod switch;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::store::json_file::{JsonFile, create_private_dir};
use crate::tr;

pub const FILE_NAME: &str = "profiles.json";
pub const FOLDER: &str = "profiles";

/// What belonged to the one person before profiles existed; the first profile takes it over. The lock, the
/// socket, the shortcuts and the app-wide half of the settings stay in the app folder.
const PROFILE_DATA: [&str; 9] = [
    crate::db::FILE_NAME,
    "hamster.sqlite-wal",
    "hamster.sqlite-shm",
    crate::db::journal::FILE_NAME,
    "docs",
    "statements",
    "snapshots",
    "rules",
    "reminders.json",
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Profile {
    /// Also the name of its folder.
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Profiles {
    /// The id of the profile the app opens.
    pub active: String,
    /// In the order they were made.
    pub profiles: Vec<Profile>,
}

/// The profile `open` opened.
pub struct Active {
    pub id: String,
    pub dir: PathBuf,
    /// The profile made first; only that one gets the debug build's demo data.
    pub first: bool,
}

/// Whether `id` is the profile made first.
pub fn is_first(root: &Path, id: &str) -> bool {
    load(root).profiles.first().is_some_and(|p| p.id == id)
}

pub fn dir(root: &Path, id: &str) -> PathBuf {
    root.join(FOLDER).join(id)
}

fn file(root: &Path) -> JsonFile {
    JsonFile::new(root.join(FILE_NAME))
}

pub fn load(root: &Path) -> Profiles {
    file(root).load()
}

/// Opens the active profile, creating its folder; an unknown active id falls back to the first profile.
pub fn open(root: &Path, first_name: &str) -> AppResult<Active> {
    let mut profiles = load(root);
    if profiles.profiles.is_empty() {
        profiles = adopt_existing_data(root, first_name)?;
    } else if !profiles.profiles.iter().any(|p| p.id == profiles.active) {
        profiles.active = profiles.profiles[0].id.clone();
        file(root).save(&profiles)?;
    }
    let dir = dir(root, &profiles.active);
    create_private_dir(&dir)?;
    Ok(Active {
        first: profiles.profiles[0].id == profiles.active,
        id: profiles.active,
        dir,
    })
}

/// Makes the first profile, named `first_name`, from the data in `root`.
fn adopt_existing_data(root: &Path, first_name: &str) -> AppResult<Profiles> {
    let first = new_profile(first_name);
    move_into(root, &dir(root, &first.id))?;
    let profiles = Profiles {
        active: first.id.clone(),
        profiles: vec![first],
    };
    file(root).save(&profiles)?;
    Ok(profiles)
}

fn new_profile(name: &str) -> Profile {
    Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
    }
}

/// Moves the one person's data from `root` into `target`; the app's own settings stay and are copied, as
/// each folder keeps only its half of them.
fn move_into(root: &Path, target: &Path) -> AppResult<()> {
    create_private_dir(target)?;
    for name in PROFILE_DATA {
        let from = root.join(name);
        if from.exists() {
            std::fs::rename(&from, target.join(name))?;
        }
    }
    let settings = root.join(crate::settings::FILE_NAME);
    if settings.exists() {
        std::fs::copy(&settings, target.join(crate::settings::FILE_NAME))?;
    }
    Ok(())
}

fn valid_name(name: &str, profiles: &Profiles, except: Option<&str>) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput(tr!(
            "A profile needs a name",
            "Ein Profil braucht einen Namen"
        )));
    }
    let wanted = name.to_lowercase();
    let taken = profiles
        .profiles
        .iter()
        .filter(|p| Some(p.id.as_str()) != except)
        .any(|p| p.name.to_lowercase() == wanted);
    if taken {
        return Err(AppError::InvalidInput(tr!(
            "There is already a profile named {name}",
            "Es gibt schon ein Profil namens {name}"
        )));
    }
    Ok(name.to_string())
}

/// Adds an empty profile with its own folder and makes it the one the next launch opens.
pub fn create(root: &Path, name: &str) -> AppResult<Profile> {
    insert(root, name, true)
}

/// Adds an empty profile with its own folder; the active profile stays.
pub fn add(root: &Path, name: &str) -> AppResult<Profile> {
    insert(root, name, false)
}

fn insert(root: &Path, name: &str, activate: bool) -> AppResult<Profile> {
    let mut profiles = load(root);
    let profile = new_profile(&valid_name(name, &profiles, None)?);
    create_private_dir(&dir(root, &profile.id))?;
    if activate {
        profiles.active = profile.id.clone();
    }
    profiles.profiles.push(profile.clone());
    file(root).save(&profiles)?;
    Ok(profile)
}

/// The profile called `name`, ignoring case and surrounding space.
pub fn named(profiles: &Profiles, name: &str) -> AppResult<Profile> {
    let wanted = name.trim().to_lowercase();
    profiles
        .profiles
        .iter()
        .find(|p| p.name.to_lowercase() == wanted)
        .cloned()
        .ok_or_else(|| {
            let name = name.trim();
            AppError::NotFound(tr!(
                "There is no profile named {name}",
                "Es gibt kein Profil namens {name}"
            ))
        })
}

/// Makes `id` the profile the next launch opens.
pub fn activate(root: &Path, id: &str) -> AppResult<()> {
    let mut profiles = load(root);
    if !profiles.profiles.iter().any(|p| p.id == id) {
        return Err(not_found());
    }
    profiles.active = id.to_string();
    file(root).save(&profiles)
}

pub fn rename(root: &Path, id: &str, name: &str) -> AppResult<Profiles> {
    let mut profiles = load(root);
    let name = valid_name(name, &profiles, Some(id))?;
    let profile = profiles
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(not_found)?;
    profile.name = name;
    file(root).save(&profiles)?;
    Ok(profiles)
}

fn not_found() -> AppError {
    AppError::NotFound(tr!(
        "This profile no longer exists",
        "Dieses Profil gibt es nicht mehr"
    ))
}

#[cfg(test)]
mod tests;
