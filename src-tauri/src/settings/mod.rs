//! The owner's preferences in `settings.json`: appearance, accent, number format and original-currency display
//! (ADR 0013, ADR 0026).

pub mod commands;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;
use crate::lock::MutexExt;
use crate::store::json_file::JsonFile;

pub const FILE_NAME: &str = "settings.json";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Accent {
    #[default]
    System,
    Orange,
    Gold,
    Red,
    Magenta,
    Purple,
    Indigo,
    Blue,
    Teal,
    Graphite,
}

/// How amounts read: `1,234.56` or `1.234,56`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum NumberFormat {
    #[default]
    DotDecimal,
    CommaDecimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Settings {
    pub appearance: Appearance,
    pub accent: Accent,
    pub number_format: NumberFormat,
    /// Shows a card payment's original amount (`USD 24.00`) beside the euro amount.
    pub show_original_currency: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::default(),
            accent: Accent::default(),
            number_format: NumberFormat::default(),
            show_original_currency: true,
        }
    }
}

/// The fields to change; absent ones keep their value.
#[derive(Deserialize, Clone, Debug, Default, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts", optional_fields)]
pub struct SettingsPatch {
    pub appearance: Option<Appearance>,
    pub accent: Option<Accent>,
    pub number_format: Option<NumberFormat>,
    pub show_original_currency: Option<bool>,
}

impl Settings {
    pub fn apply(&mut self, patch: SettingsPatch) {
        if let Some(appearance) = patch.appearance {
            self.appearance = appearance;
        }
        if let Some(accent) = patch.accent {
            self.accent = accent;
        }
        if let Some(format) = patch.number_format {
            self.number_format = format;
        }
        if let Some(show) = patch.show_original_currency {
            self.show_original_currency = show;
        }
    }
}

/// The settings file and its current value; every change is saved before it is answered.
pub struct SettingsStore {
    file: JsonFile,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn open(file: JsonFile) -> Self {
        let current = file.load::<Settings>();
        Self {
            file,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.lock_unpoisoned().clone()
    }

    pub fn update(&self, patch: SettingsPatch) -> AppResult<Settings> {
        let mut current = self.current.lock_unpoisoned();
        let mut next = current.clone();
        next.apply(patch);
        self.file.save(&next)?;
        *current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use serde_json::json;

    fn store_in(dir: &tempfile::TempDir) -> SettingsStore {
        SettingsStore::open(JsonFile::new(dir.path().join(FILE_NAME)))
    }

    #[test]
    fn defaults_are_system_look_dot_decimal_and_original_currency_shown() {
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap(),
            json!({
                "appearance": "system",
                "accent": "system",
                "numberFormat": "dot_decimal",
                "showOriginalCurrency": true
            })
        );
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let dir = temp_config_dir();
        assert_eq!(store_in(&dir).get(), Settings::default());
    }

    #[test]
    fn an_old_file_with_fewer_fields_fills_the_rest_with_defaults() {
        let dir = temp_config_dir();
        std::fs::write(dir.path().join(FILE_NAME), r#"{"accent":"teal"}"#).unwrap();

        assert_eq!(
            store_in(&dir).get(),
            Settings {
                accent: Accent::Teal,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn an_unknown_value_in_the_file_gives_the_defaults_and_moves_the_file_aside() {
        let dir = temp_config_dir();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, r#"{"accent":"chartreuse"}"#).unwrap();

        assert_eq!(store_in(&dir).get(), Settings::default());
        assert!(!path.exists());
    }

    #[test]
    fn update_changes_only_the_patched_fields_and_returns_the_new_settings() {
        let dir = temp_config_dir();
        let store = store_in(&dir);

        let first = store
            .update(SettingsPatch {
                appearance: Some(Appearance::Dark),
                show_original_currency: Some(false),
                ..SettingsPatch::default()
            })
            .unwrap();
        let expected = Settings {
            appearance: Appearance::Dark,
            show_original_currency: false,
            ..Settings::default()
        };
        assert_eq!(first, expected);
        assert_eq!(store.get(), expected);

        let second = store
            .update(SettingsPatch {
                number_format: Some(NumberFormat::CommaDecimal),
                ..SettingsPatch::default()
            })
            .unwrap();
        assert_eq!(
            second,
            Settings {
                number_format: NumberFormat::CommaDecimal,
                ..expected
            }
        );
    }

    #[test]
    fn an_empty_patch_changes_nothing() {
        let dir = temp_config_dir();
        let store = store_in(&dir);
        assert_eq!(
            store.update(SettingsPatch::default()).unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn updates_survive_reopening_the_file() {
        let dir = temp_config_dir();
        store_in(&dir)
            .update(SettingsPatch {
                accent: Some(Accent::Graphite),
                number_format: Some(NumberFormat::CommaDecimal),
                ..SettingsPatch::default()
            })
            .unwrap();

        assert_eq!(
            store_in(&dir).get(),
            Settings {
                accent: Accent::Graphite,
                number_format: NumberFormat::CommaDecimal,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn a_patch_reads_its_fields_in_camel_case_and_leaves_absent_ones_out() {
        let patch: SettingsPatch =
            serde_json::from_value(json!({ "numberFormat": "comma_decimal" })).unwrap();
        assert_eq!(patch.number_format, Some(NumberFormat::CommaDecimal));
        assert_eq!(patch.appearance, None);
        assert_eq!(patch.accent, None);
        assert_eq!(patch.show_original_currency, None);
    }
}
