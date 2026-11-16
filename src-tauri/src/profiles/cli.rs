//! `hamster profile …`: answered the same way by the running app and, when none runs, by the CLI itself.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

/// What `hamster profile` asks; profiles are addressed by name.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ProfileRequest {
    List,
    Switch { name: String },
    Add { name: String, switch: bool },
    Rename { name: String, to: String },
}

/// Applies `request` to `profiles.json`: `list` answers with every profile and the active id, the others with
/// the profile they touched. A switch only names the profile the next launch opens.
pub fn answer(root: &Path, request: &ProfileRequest) -> AppResult<Value> {
    let profiles = super::load(root);
    match request {
        ProfileRequest::List => json(&profiles),
        ProfileRequest::Switch { name } => {
            let profile = super::named(&profiles, name)?;
            super::activate(root, &profile.id)?;
            json(&profile)
        }
        ProfileRequest::Add { name, switch } => {
            let profile = if *switch {
                super::create(root, name)?
            } else {
                super::add(root, name)?
            };
            json(&profile)
        }
        ProfileRequest::Rename { name, to } => {
            let profile = super::named(&profiles, name)?;
            let renamed = super::rename(root, &profile.id, to)?;
            json(&super::named(&renamed, to)?)
        }
    }
}

fn json(value: &impl Serialize) -> AppResult<Value> {
    serde_json::to_value(value).map_err(AppError::from)
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
