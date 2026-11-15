//! Tax rules (ADR 0051): every year starts from the rules built into the app and follows newer built-in
//! versions until the owner edits it in Settings; then it keeps the owner's version. A closed year's rules
//! are frozen in its row.

pub mod check;
pub mod commands;
pub mod diff;
pub mod edit;
pub mod files;
pub mod import;
pub mod model;
pub mod view;

#[cfg(test)]
mod check_tests;
#[cfg(test)]
mod diff_tests;
#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod files_tests;
#[cfg(test)]
mod import_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod view_tests;

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::taxyears::{self, RulesSource, TaxYearRow, YearState};

/// One year in Settings → Tax Rules.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesYear {
    pub year: i32,
    pub state: YearState,
    pub source: RulesSource,
    pub ruleset: String,
    /// The ruleset the owner's version started from.
    pub based_on: Option<String>,
    /// A built-in version the year doesn't use yet: newer than the owner's version, or one that can't be
    /// taken on (see `problems`).
    pub builtin_newer: Option<String>,
    /// Why the built-in version can't be taken on.
    pub problems: Vec<String>,
    /// The app has built-in rules for the year, so "Back to built-in" can work.
    pub has_builtin: bool,
    pub unverified: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesOverview {
    /// Newest first.
    pub years: Vec<RulesYear>,
    /// The year Start would begin, when it doesn't exist yet.
    pub next_year: Option<i32>,
    /// Start takes the built-in rules rather than a copy of the newest year.
    pub next_is_builtin: bool,
}

/// Item keys anything recorded uses; a ruleset can't drop them.
pub fn used_keys(conn: &Connection) -> AppResult<BTreeSet<String>> {
    let mut query = conn.prepare(
        "SELECT item_key FROM documents WHERE item_key IS NOT NULL
         UNION SELECT item_key FROM entries
         UNION SELECT item_key FROM decisions WHERE item_key IS NOT NULL
         UNION SELECT item_key FROM subscriptions WHERE item_key IS NOT NULL
         UNION SELECT default_item_key FROM vendors WHERE default_item_key IS NOT NULL",
    )?;
    let keys = query.query_map([], |r| r.get::<_, String>(0))?;
    Ok(keys.collect::<Result<_, _>>()?)
}

fn unusable(problems: &[String]) -> AppError {
    AppError::InvalidInput(crate::tr!(
        "These rules can't be used: {}",
        "Diese Regeln sind nicht verwendbar: {}",
        problems.join("; ")
    ))
}

/// The problem when rules meant for `found` are given to `year`.
pub(crate) fn wrong_year(found: i32, year: i32) -> Option<String> {
    (found != year).then(|| {
        crate::tr!(
            "year: the rules say {found}, not {year}",
            "year: Die Regeln sagen {found}, nicht {year}"
        )
    })
}

/// The built-in text a year would move to, with its check; None when it already uses it.
fn pending_builtin(
    row: &TaxYearRow,
    used: &BTreeSet<String>,
) -> Option<(&'static str, Result<model::Ruleset, Vec<String>>)> {
    let text = files::bundled(row.year)?;
    if text == row.rules {
        return None;
    }
    Some((
        text,
        check::read(text, used).and_then(|r| match wrong_year(r.year, row.year) {
            Some(problem) => Err(vec![problem]),
            None => Ok(r),
        }),
    ))
}

/// Every year, newest first, with what Start would add.
pub fn overview(db: &Db) -> AppResult<RulesOverview> {
    let rows = db.read(taxyears::rows)?;
    let used = db.read(used_keys)?;
    let years = rows.iter().map(|row| describe(row, &used)).collect();
    let next_year = rows
        .first()
        .map(|r| r.year + 1)
        .or_else(|| files::bundled_years().last());
    Ok(RulesOverview {
        years,
        next_year,
        next_is_builtin: next_year.is_some_and(|y| files::bundled(y).is_some()),
    })
}

fn describe(row: &TaxYearRow, used: &BTreeSet<String>) -> RulesYear {
    let applied = check::read(&row.rules, &BTreeSet::new()).ok();
    let bundled = files::bundled(row.year);
    let builtin = bundled.and_then(|t| check::read(t, &BTreeSet::new()).ok());
    let (builtin_newer, problems) = match (row.state, row.rules_source) {
        (YearState::Open, RulesSource::Builtin) => match pending_builtin(row, used) {
            Some((_, Err(problems))) => (builtin.map(|b| b.ruleset), problems),
            _ => (None, Vec::new()),
        },
        (YearState::Open, RulesSource::Own) => {
            let base = applied.as_ref().and_then(|a| a.based_on.as_deref());
            let newer =
                builtin.filter(|b| Some(b.ruleset.as_str()) != base && b.ruleset != row.ruleset);
            (newer.map(|b| b.ruleset), Vec::new())
        }
        (YearState::Closed, _) => (None, Vec::new()),
    };
    RulesYear {
        year: row.year,
        state: row.state,
        source: row.rules_source,
        ruleset: row.ruleset.clone(),
        based_on: applied.as_ref().and_then(|a| a.based_on.clone()),
        builtin_newer,
        problems,
        has_builtin: bundled.is_some(),
        unverified: applied.map_or(0, |a| a.unverified_count() as u32),
    }
}

/// The row after taking on `text` as the year's rules from `source`.
fn with_rules(
    row: &TaxYearRow,
    ruleset: String,
    text: String,
    source: RulesSource,
    now: i64,
) -> TaxYearRow {
    TaxYearRow {
        ruleset,
        rules: text,
        rules_source: source,
        rules_loaded_at: now,
        ..row.clone()
    }
}

/// Moves every open year that follows the built-in rules to a newer built-in version, as one undoable change.
/// A version that fails the checks is left for Settings to explain.
pub fn adopt_builtin(db: &Db, now: i64) -> AppResult<Option<Written>> {
    let used = db.read(used_keys)?;
    let mut changes = Vec::new();
    let mut names = Vec::new();
    for row in db.read(taxyears::rows)? {
        if row.state != YearState::Open || row.rules_source != RulesSource::Builtin {
            continue;
        }
        let Some((text, Ok(next))) = pending_builtin(&row, &used) else {
            continue;
        };
        let after = with_rules(
            &row,
            next.ruleset.clone(),
            text.to_string(),
            RulesSource::Builtin,
            now,
        );
        changes.push(Change::update(taxyears::TABLE, &row, &after)?);
        names.push(next.ruleset);
    }
    if changes.is_empty() {
        return Ok(None);
    }
    // Rows come newest first; the label names the years in calendar order.
    names.reverse();
    let names = names.join(", ");
    Ok(Some(db.write(
        Op::new(
            crate::tr!(
                "Update Tax Rules to {names}",
                "Steuerregeln auf {names} aktualisieren"
            ),
            changes,
        ),
        Source::App,
    )?))
}

fn open_row(db: &Db, year: i32) -> AppResult<TaxYearRow> {
    let row = db
        .read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(crate::tr!("Tax year {year}", "Steuerjahr {year}")))?;
    if row.state == YearState::Closed {
        return Err(AppError::InvalidInput(crate::tr!(
            "Tax year {year} is closed; its rules are frozen",
            "Das Steuerjahr {year} ist abgeschlossen; seine Regeln sind eingefroren"
        )));
    }
    Ok(row)
}

/// Makes `text` the owner's version of the year's rules, after the same checks as any rules.
pub fn set_own(db: &Db, year: i32, text: String, label: String, now: i64) -> AppResult<Written> {
    let row = open_row(db, year)?;
    let used = db.read(used_keys)?;
    let ruleset = check::read(&text, &used).map_err(|p| unusable(&p))?;
    if let Some(problem) = wrong_year(ruleset.year, year) {
        return Err(unusable(&[problem]));
    }
    let after = with_rules(&row, ruleset.ruleset, text, RulesSource::Own, now);
    db.write(
        Op::new(label, vec![Change::update(taxyears::TABLE, &row, &after)?]),
        Source::Ui,
    )
}

pub fn use_builtin(db: &Db, year: i32, now: i64) -> AppResult<Written> {
    let row = open_row(db, year)?;
    let text = files::bundled(year).ok_or_else(|| {
        AppError::InvalidInput(crate::tr!(
            "Hamster has no built-in rules for {year}",
            "Hamster hat keine eingebauten Regeln für {year}"
        ))
    })?;
    let ruleset = check::read(text, &db.read(used_keys)?).map_err(|p| unusable(&p))?;
    let after = with_rules(
        &row,
        ruleset.ruleset,
        text.to_string(),
        RulesSource::Builtin,
        now,
    );
    db.write(
        Op::new(
            crate::tr!(
                "Use the Built-in Tax Rules for {year}",
                "Eingebaute Steuerregeln für {year} verwenden"
            ),
            vec![Change::update(taxyears::TABLE, &row, &after)?],
        ),
        Source::Ui,
    )
}

/// Starts the year after the newest: from its built-in rules when the app has them, else as a copy of the
/// newest year's rules (the owner's version).
pub fn start_next(db: &Db, now: i64) -> AppResult<Written> {
    let rows = db.read(taxyears::rows)?;
    let Some(newest) = rows.first() else {
        return Err(AppError::InvalidInput(crate::tr!(
            "There is no tax year to continue from",
            "Es gibt kein Steuerjahr, an das sich anschließen lässt"
        )));
    };
    let year = newest.year + 1;
    let (text, source) = match files::bundled(year) {
        Some(text) => (text.to_string(), RulesSource::Builtin),
        None => (files::next_year_text(&newest.rules)?, RulesSource::Own),
    };
    let ruleset = check::read(&text, &BTreeSet::new()).map_err(|p| unusable(&p))?;
    let row = TaxYearRow {
        rules_source: source,
        ..TaxYearRow::new(year, ruleset.ruleset, text, now)
    };
    let mut changes = vec![Change::insert(taxyears::TABLE, &row)?];
    changes.extend(db.read(|c| crate::year::hidden::carried_over(c, newest.year, year))?);
    db.write(
        Op::new(
            crate::tr!("Start Tax Year {year}", "Steuerjahr {year} beginnen"),
            changes,
        ),
        Source::Ui,
    )
}
