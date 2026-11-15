//! Importing a rules file for any year (Settings → Tax Rules → Import JSON…). The file names its year: an open
//! year takes it as its rules, a missing year starts with it, even across a gap; a closed year refuses it.

use std::collections::BTreeSet;

use serde::Serialize;
use ts_rs::TS;

use super::{check, diff, files, unusable, used_keys, with_rules};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::taxyears::{self, RulesSource, TaxYearRow, YearState};

/// What importing a file would do, shown before it happens.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesImport {
    /// The year the file names; None when it can't be read.
    pub year: Option<i32>,
    pub text: String,
    /// The year exists; importing replaces its rules.
    pub replaces: bool,
    pub problems: Vec<String>,
    pub changes: Vec<String>,
}

fn frozen(year: i32) -> String {
    crate::tr!(
        "Tax year {year} is closed; its rules are frozen",
        "Das Steuerjahr {year} ist abgeschlossen; seine Regeln sind eingefroren"
    )
}

/// Item keys the rules must keep: those recorded anywhere when they replace a year's rules; none for a new year.
fn kept_keys(db: &Db, existing: Option<&TaxYearRow>) -> AppResult<BTreeSet<String>> {
    match existing {
        Some(_) => db.read(used_keys),
        None => Ok(BTreeSet::new()),
    }
}

pub fn preview(db: &Db, text: String) -> AppResult<RulesImport> {
    let unreadable = |problems| RulesImport {
        year: None,
        text: String::new(),
        replaces: false,
        problems,
        changes: Vec::new(),
    };
    let year = match check::read(&text, &BTreeSet::new()) {
        Ok(ruleset) => ruleset.year,
        Err(problems) => return Ok(unreadable(problems)),
    };
    let row = db.read(|c| taxyears::row(c, year))?;
    let next = match check::read(&text, &kept_keys(db, row.as_ref())?) {
        Ok(next) => next,
        Err(problems) => {
            return Ok(RulesImport {
                year: Some(year),
                replaces: row.is_some(),
                ..unreadable(problems)
            });
        }
    };
    let problems = match &row {
        Some(r) if r.state == YearState::Closed => vec![frozen(year)],
        _ => Vec::new(),
    };
    let changes = row
        .as_ref()
        .and_then(|r| check::read(&r.rules, &BTreeSet::new()).ok())
        .map(|current| diff::describe(&current, &next))
        .unwrap_or_default();
    Ok(RulesImport {
        year: Some(year),
        text,
        replaces: row.is_some(),
        problems,
        changes,
    })
}

/// Imports `text` for the year it names, as one undoable change. A file identical to the built-in rules keeps
/// the year following them; anything else makes it the owner's version.
pub fn import(db: &Db, text: &str, now: i64) -> AppResult<Written> {
    let year = check::read(text, &BTreeSet::new())
        .map_err(|p| unusable(&p))?
        .year;
    let existing = db.read(|c| taxyears::row(c, year))?;
    let ruleset =
        check::read(text, &kept_keys(db, existing.as_ref())?).map_err(|p| unusable(&p))?;
    let source = if files::bundled(year) == Some(text) {
        RulesSource::Builtin
    } else {
        RulesSource::Own
    };
    let changes = match existing {
        Some(row) if row.state == YearState::Closed => {
            return Err(AppError::InvalidInput(frozen(year)));
        }
        Some(row) => {
            let after = with_rules(&row, ruleset.ruleset, text.to_string(), source, now);
            vec![Change::update(taxyears::TABLE, &row, &after)?]
        }
        None => {
            let row = TaxYearRow {
                rules_source: source,
                ..TaxYearRow::new(year, ruleset.ruleset, text.to_string(), now)
            };
            let mut changes = vec![Change::insert(taxyears::TABLE, &row)?];
            changes.extend(db.read(|c| crate::year::hidden::carried_over(c, year - 1, year))?);
            changes
        }
    };
    db.write(
        Op::new(
            crate::tr!(
                "Import the Tax Rules for {year}",
                "Steuerregeln für {year} importieren"
            ),
            changes,
        ),
        Source::Ui,
    )
}
