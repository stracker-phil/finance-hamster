//! Tax rules files (ADR 0040): their state beside the tax years, Reload with a preview, Import and Duplicate.
//! An open year applies the rules text stored in its row; the file takes effect only through Reload.

pub mod check;
pub mod commands;
pub mod diff;
pub mod files;
pub mod model;

#[cfg(test)]
mod check_tests;
#[cfg(test)]
mod diff_tests;
#[cfg(test)]
mod files_tests;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::taxyears::{self, TaxYearRow, YearState};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum RulesState {
    /// A future year's file, not used yet.
    Draft,
    Open,
    Closed,
}

/// One row of Settings → Tax Rules.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesFile {
    pub file_name: String,
    pub year: i32,
    pub state: RulesState,
    /// The file's ruleset name, or the applied one when the file can't be read.
    pub ruleset: Option<String>,
    pub based_on: Option<String>,
    pub items: u32,
    pub unverified: u32,
    /// Why the file can't be used; empty when it is valid.
    pub problems: Vec<String>,
    /// An open year's file differs from what the year applies, so Reload has something to do.
    pub edited: bool,
    /// The year exists but its file is gone; a closed year still has its frozen copy.
    pub missing: bool,
    /// When a closed year was closed, as a date label.
    pub closed_on: Option<String>,
    /// When an open year last loaded its file, as a date label.
    pub loaded_on: Option<String>,
}

/// One open year's pending Reload.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ReloadPreview {
    pub year: i32,
    pub from: String,
    pub to: Option<String>,
    pub changes: Vec<String>,
    pub problems: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Imported {
    pub file_name: String,
    pub year: i32,
    /// The replaced file's new name, kept as a backup.
    pub backup: Option<String>,
    /// The year is open, so a Reload applies the imported file.
    pub needs_reload: bool,
}

fn year_problem(file_year: i32, year: i32) -> Option<String> {
    (file_year != year).then(|| {
        crate::tr!(
            "year: the file says {file_year}, its name says {year}",
            "year: Die Datei sagt {file_year}, ihr Name sagt {year}"
        )
    })
}

fn ruleset_name(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    Some(value.get("ruleset")?.as_str()?.to_string())
}

/// Every rules file and every tax year, newest year first.
pub fn overview(
    db: &Db,
    config_dir: &Path,
    date: &impl Fn(i64) -> String,
) -> AppResult<Vec<RulesFile>> {
    let years: BTreeMap<i32, TaxYearRow> = db
        .read(taxyears::rows)?
        .into_iter()
        .map(|r| (r.year, r))
        .collect();
    let mut all: BTreeSet<i32> = years.keys().copied().collect();
    all.extend(files::list(config_dir)?.into_keys());
    let mut out = Vec::new();
    for year in all.into_iter().rev() {
        let row = years.get(&year);
        let text = files::read(config_dir, year)?;
        out.push(describe_file(year, row, text.as_deref(), date));
    }
    Ok(out)
}

fn describe_file(
    year: i32,
    row: Option<&TaxYearRow>,
    text: Option<&str>,
    date: &impl Fn(i64) -> String,
) -> RulesFile {
    let state = match row.map(|r| r.state) {
        None => RulesState::Draft,
        Some(YearState::Open) => RulesState::Open,
        Some(YearState::Closed) => RulesState::Closed,
    };
    // A closed year shows its frozen copy; the file beside it no longer matters.
    let shown = match (state, row) {
        (RulesState::Closed, Some(r)) => Some(r.rules.as_str()),
        _ => text,
    };
    let checked = shown.map(|t| check::read(t, &BTreeSet::new()));
    let ruleset = checked.as_ref().and_then(|c| c.as_ref().ok());
    let mut problems = checked
        .as_ref()
        .and_then(|c| c.as_ref().err().cloned())
        .unwrap_or_default();
    problems.extend(ruleset.and_then(|r| year_problem(r.year, year)));
    RulesFile {
        file_name: files::file_name(year),
        year,
        state,
        ruleset: ruleset
            .map(|r| r.ruleset.clone())
            .or_else(|| row.map(|r| r.ruleset.clone())),
        based_on: ruleset.and_then(|r| r.based_on.clone()),
        items: ruleset.map_or(0, |r| r.items.len() as u32),
        unverified: ruleset.map_or(0, |r| r.unverified_count() as u32),
        problems,
        edited: state == RulesState::Open
            && text.is_some_and(|t| row.is_some_and(|r| r.rules != t)),
        missing: text.is_none(),
        closed_on: row.and_then(|r| r.closed_at).map(date),
        loaded_on: row
            .filter(|r| r.state == YearState::Open)
            .map(|r| date(r.rules_loaded_at)),
    }
}

/// What Reload would change in each open year whose file differs from the applied rules.
pub fn preview(db: &Db, config_dir: &Path) -> AppResult<Vec<ReloadPreview>> {
    let mut out = Vec::new();
    for row in db.read(taxyears::rows)? {
        if row.state != YearState::Open {
            continue;
        }
        let Some(text) = files::read(config_dir, row.year)? else {
            continue;
        };
        if text == row.rules {
            continue;
        }
        out.push(preview_year(&row, &text));
    }
    out.sort_by_key(|p| p.year);
    Ok(out)
}

fn preview_year(row: &TaxYearRow, text: &str) -> ReloadPreview {
    let mut preview = ReloadPreview {
        year: row.year,
        from: row.ruleset.clone(),
        to: None,
        changes: Vec::new(),
        problems: Vec::new(),
    };
    match check::read(text, &BTreeSet::new()) {
        Err(problems) => preview.problems = problems,
        Ok(next) if next.year != row.year => {
            preview.problems.extend(year_problem(next.year, row.year));
        }
        Ok(next) => {
            preview.changes = match check::read(&row.rules, &BTreeSet::new()) {
                Ok(current) => diff::describe(&current, &next),
                Err(_) => vec![crate::tr!(
                    "The applied rules can't be read; the file replaces them",
                    "Die angewendeten Regeln sind nicht lesbar; die Datei ersetzt sie"
                )],
            };
            if preview.changes.is_empty() {
                preview.changes.push(crate::tr!(
                    "Only the formatting changed",
                    "Nur die Formatierung hat sich geändert"
                ));
            }
            preview.to = Some(next.ruleset);
        }
    }
    preview
}

/// Applies every pending Reload as one undoable change; refused while any file has a problem.
pub fn apply(db: &Db, config_dir: &Path, now: i64) -> AppResult<Option<Written>> {
    let previews = preview(db, config_dir)?;
    if let Some(bad) = previews.iter().find(|p| !p.problems.is_empty()) {
        return Err(AppError::InvalidInput(crate::tr!(
            "{} can't be loaded: {}",
            "{} lässt sich nicht laden: {}",
            files::file_name(bad.year),
            bad.problems.join("; ")
        )));
    }
    let mut changes = Vec::new();
    for p in &previews {
        let (Some(before), Some(text)) = (
            db.read(|c| taxyears::row(c, p.year))?,
            files::read(config_dir, p.year)?,
        ) else {
            continue;
        };
        let after = TaxYearRow {
            ruleset: p.to.clone().unwrap_or_else(|| before.ruleset.clone()),
            rules: text,
            rules_loaded_at: now,
            ..before.clone()
        };
        changes.push(Change::update(taxyears::TABLE, &before, &after)?);
    }
    if changes.is_empty() {
        return Ok(None);
    }
    Ok(Some(db.write(
        Op::new(
            crate::tr!("Reload Tax Rules", "Steuerregeln neu laden"),
            changes,
        ),
        Source::Ui,
    )?))
}

/// Puts a checked rules file in place of its year's file, keeping the old one; a closed year refuses.
pub fn import(db: &Db, config_dir: &Path, text: &str) -> AppResult<Imported> {
    let ruleset = check::read(text, &BTreeSet::new()).map_err(|problems| {
        AppError::InvalidInput(crate::tr!(
            "The file can't be used: {}",
            "Die Datei ist nicht verwendbar: {}",
            problems.join("; ")
        ))
    })?;
    let year = ruleset.year;
    let row = db.read(|c| taxyears::row(c, year))?;
    if row.as_ref().is_some_and(|r| r.state == YearState::Closed) {
        return Err(AppError::InvalidInput(crate::tr!(
            "Tax year {year} is closed; its rules are frozen",
            "Das Steuerjahr {year} ist abgeschlossen; seine Regeln sind eingefroren"
        )));
    }
    let old_ruleset = files::read(config_dir, year)?.and_then(|old| ruleset_name(&old));
    let backup = files::replace(config_dir, year, text, old_ruleset.as_deref())?;
    Ok(Imported {
        file_name: files::file_name(year),
        year,
        backup,
        needs_reload: row.is_some(),
    })
}

/// Reads and imports the file at `path`.
pub fn import_file(db: &Db, config_dir: &Path, path: &Path) -> AppResult<Imported> {
    import(db, config_dir, &std::fs::read_to_string(path)?)
}

/// The file to show for a year: its rules file, or for a closed year a read-only copy of its frozen rules.
pub fn open_target(db: &Db, config_dir: &Path, year: i32) -> AppResult<PathBuf> {
    match db.read(|c| taxyears::row(c, year))? {
        Some(row) if row.state == YearState::Closed => frozen_copy(year, &row.rules),
        _ => Ok(files::dir(config_dir).join(files::file_name(year))),
    }
}

fn frozen_copy(year: i32, rules: &str) -> AppResult<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("rules-{year} (closed).json"));
    // An earlier copy is read-only, so it is replaced rather than written over.
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::write(&path, rules)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400))?;
    Ok(path)
}

/// Copies the newest open year's file to the next year as a draft; returns the new file's name.
pub fn duplicate(db: &Db, config_dir: &Path) -> AppResult<String> {
    let newest = db
        .read(taxyears::rows)?
        .into_iter()
        .find(|r| r.state == YearState::Open)
        .ok_or_else(|| {
            AppError::InvalidInput(crate::tr!(
                "There is no open tax year to copy",
                "Es gibt kein offenes Steuerjahr zum Kopieren"
            ))
        })?;
    let text = files::read(config_dir, newest.year)?.unwrap_or(newest.rules);
    let next = files::next_year_text(&text)?;
    files::create(config_dir, newest.year + 1, &next)?;
    Ok(files::file_name(newest.year + 1))
}
