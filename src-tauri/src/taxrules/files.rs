//! The `rules/` folder: one `rules-<year>.json` per tax year, next to the schema and README the app ships
//! (ADR 0040). Hamster writes a file only when asked (seed, Import, Duplicate); editing happens outside.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};
use crate::store::json_file::create_private_dir;

pub const DIR: &str = "rules";
const SCHEMA: (&str, &str) = (
    "rules.schema.json",
    include_str!("../../resources/rules/rules.schema.json"),
);
const README: (&str, &str) = ("README.md", include_str!("../../resources/rules/README.md"));
/// The year to file and the year under way when Hamster was built; a first launch starts both.
const SEEDS: [(&str, &str); 2] = [
    (
        "rules-2025.json",
        include_str!("../../resources/rules/rules-2025.json"),
    ),
    (
        "rules-2026.json",
        include_str!("../../resources/rules/rules-2026.json"),
    ),
];

pub fn dir(config_dir: &Path) -> PathBuf {
    config_dir.join(DIR)
}

pub fn file_name(year: i32) -> String {
    format!("rules-{year}.json")
}

/// The year of `rules-<year>.json`; backups (`rules-2026.v1.json`) and other files have none.
pub fn year_of(file_name: &str) -> Option<i32> {
    let year = file_name.strip_prefix("rules-")?.strip_suffix(".json")?;
    if year.len() != 4 || !year.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    year.parse().ok()
}

/// Keeps the schema and README current and writes the seeds when the folder holds no rules file yet.
pub fn install(config_dir: &Path) -> AppResult<()> {
    let dir = dir(config_dir);
    create_private_dir(&dir)?;
    for (name, text) in [SCHEMA, README] {
        let path = dir.join(name);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text) {
            write_atomic(&path, text)?;
        }
    }
    if list(config_dir)?.is_empty() {
        for (name, text) in SEEDS {
            write_atomic(&dir.join(name), text)?;
        }
    }
    Ok(())
}

/// Every `rules-<year>.json` in the folder, by year.
pub fn list(config_dir: &Path) -> AppResult<BTreeMap<i32, PathBuf>> {
    let mut files = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir(config_dir)) else {
        return Ok(files);
    };
    for entry in entries {
        let path = entry?.path();
        if let Some(year) = path.file_name().and_then(|n| n.to_str()).and_then(year_of) {
            files.insert(year, path);
        }
    }
    Ok(files)
}

pub fn read(config_dir: &Path, year: i32) -> AppResult<Option<String>> {
    let path = dir(config_dir).join(file_name(year));
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Replaces the year's file with `text`; an existing file is first kept as `rules-<year>.v<N>.json`, `N` from
/// its ruleset name. Returns the backup's file name.
pub fn replace(
    config_dir: &Path,
    year: i32,
    text: &str,
    old_ruleset: Option<&str>,
) -> AppResult<Option<String>> {
    let dir = dir(config_dir);
    let path = dir.join(file_name(year));
    let backup = if path.exists() {
        let version = old_ruleset
            .and_then(|r| r.split_once(" v"))
            .map_or("0", |(_, v)| v);
        let name = free_name(&dir, &format!("rules-{year}.v{version}"));
        std::fs::rename(&path, dir.join(&name))?;
        Some(name)
    } else {
        None
    };
    write_atomic(&path, text)?;
    Ok(backup)
}

/// Writes a year's first file; an existing one is never overwritten.
pub fn create(config_dir: &Path, year: i32, text: &str) -> AppResult<()> {
    let path = dir(config_dir).join(file_name(year));
    if path.exists() {
        return Err(AppError::InvalidInput(crate::tr!(
            "{} already exists",
            "{} gibt es schon",
            file_name(year)
        )));
    }
    write_atomic(&path, text)
}

fn free_name(dir: &Path, stem: &str) -> String {
    let mut name = format!("{stem}.json");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem}-{n}.json");
        n += 1;
    }
    name
}

fn write_atomic(path: &Path, text: &str) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

/// Next year's draft from an open year's file: its year, `<year> v1`, `basedOn` the old ruleset and the
/// Kilometergeld ranges moved by one year. Everything else is copied as it is.
pub fn next_year_text(text: &str) -> AppResult<String> {
    let Value::Object(old) = serde_json::from_str::<Value>(text)? else {
        return Err(AppError::InvalidInput(crate::tr!(
            "The rules file isn't an object",
            "Die Regeldatei ist kein Objekt"
        )));
    };
    let year = old.get("year").and_then(Value::as_i64).ok_or_else(|| {
        AppError::InvalidInput(crate::tr!(
            "The rules file has no year",
            "Die Regeldatei hat kein Jahr"
        ))
    })?;
    let ruleset = old.get("ruleset").cloned().unwrap_or(Value::Null);
    let mut next = Map::new();
    for (key, value) in old {
        match key.as_str() {
            "basedOn" => continue,
            "year" => {
                next.insert(key, Value::from(year + 1));
                next.insert("basedOn".into(), ruleset.clone());
            }
            "ruleset" => {
                next.insert(key, Value::from(format!("{} v1", year + 1)));
            }
            "rates" => {
                next.insert(key, shift_rates(value));
            }
            _ => {
                next.insert(key, value);
            }
        }
    }
    let mut out = serde_json::to_string_pretty(&Value::Object(next))?;
    out.push('\n');
    Ok(out)
}

fn shift_rates(mut rates: Value) -> Value {
    let ranges = rates
        .get_mut("kilometergeld")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten();
    for range in ranges {
        for field in ["from", "to"] {
            if let Some(Value::String(day)) = range.get_mut(field)
                && let Some(next) = next_year_day(day)
            {
                *day = next;
            }
        }
    }
    rates
}

fn next_year_day(day: &str) -> Option<String> {
    use chrono::{Datelike, NaiveDate};
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    // 29 February has no counterpart in the next year; the range then ends a day earlier.
    let next = date
        .with_year(date.year() + 1)
        .or_else(|| NaiveDate::from_ymd_opt(date.year() + 1, date.month(), 28))?;
    Some(next.format("%Y-%m-%d").to_string())
}
