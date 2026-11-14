//! The year check (ADR 0043): questions from the ruleset's `ask` blocks that remind of deductions. Only the time
//! of the last completed run is stored; a run's answers and its Later list live in the page.

pub mod commands;
#[cfg(test)]
mod tests;

use chrono::{Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::date::{self, DateOrder};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::settings::NumberFormat;
use crate::taxrules::model::{Ask, Item, ItemKind, Ruleset};
use crate::taxyears::{self, YearState};
use crate::year::answers;
use crate::year::page::AddKind;
use crate::year::summary::question_views;

pub const TABLE: &str = "year_checks";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct YearCheckRow {
    pub id: String,
    pub year: i32,
    pub completed_at: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct CheckQuestion {
    /// The item key or the question id.
    pub key: String,
    /// The item's Kennzahl, `721`; None for a question to answer with a number.
    pub kz: Option<String>,
    pub question: String,
    pub hint: Option<String>,
    /// What Yes opens; None for a number question.
    pub add: Option<AddKind>,
    pub unit: Option<String>,
    /// A number question's answer as the tax year page shows it; empty while unanswered.
    pub value: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct YearCheckView {
    pub year: i32,
    pub open: bool,
    /// In the order a run asks them.
    pub questions: Vec<CheckQuestion>,
    /// `14 Nov 2026`; None before the first completed run.
    pub last_run: Option<String>,
    /// 31 Dec of the year has passed.
    pub year_over: bool,
    /// `Year check: last run 2 Jun 2026 · run it again after 31 Dec`, for the tax year page.
    pub line: String,
    /// What Close Year asks first; None when a run after 31 Dec is done or there is nothing to ask.
    pub close_prompt: Option<String>,
}

/// The items and questions with an `ask`, by `order`, then as the file lists them (items after questions).
fn asked(ruleset: &Ruleset) -> Vec<(&Ask, Asked<'_>)> {
    let mut out: Vec<(&Ask, Asked)> = ruleset
        .questions
        .iter()
        .filter_map(|q| Some((q.ask.as_ref()?, Asked::Number(q.id.as_str()))))
        .chain(
            ruleset
                .items
                .iter()
                .filter(|i| i.visible && i.kind != ItemKind::ReportedByOthers)
                .filter_map(|i| Some((i.ask.as_ref()?, Asked::Item(i)))),
        )
        .collect();
    out.sort_by_key(|(ask, _)| ask.order);
    out
}

enum Asked<'a> {
    Number(&'a str),
    Item(&'a Item),
}

fn add_kind(kind: ItemKind) -> Option<AddKind> {
    match kind {
        ItemKind::Documents => Some(AddKind::Document),
        ItemKind::Trips => Some(AddKind::Trip),
        ItemKind::Amount => Some(AddKind::Amount),
        ItemKind::ReportedByOthers => None,
    }
}

pub fn questions(
    db: &Db,
    ruleset: &Ruleset,
    format: NumberFormat,
) -> AppResult<Vec<CheckQuestion>> {
    let given = answers::answers(db, ruleset.year)?;
    let views = question_views(ruleset, &given, format);
    Ok(asked(ruleset)
        .into_iter()
        .map(|(ask, asked)| {
            let base = CheckQuestion {
                key: String::new(),
                kz: None,
                question: ask.question.clone(),
                hint: ask.hint.clone(),
                add: None,
                unit: None,
                value: String::new(),
            };
            match asked {
                Asked::Item(item) => CheckQuestion {
                    key: item.key.clone(),
                    kz: Some(item.kz.clone()),
                    add: add_kind(item.kind),
                    ..base
                },
                Asked::Number(id) => {
                    let view = views.iter().find(|v| v.id == id);
                    CheckQuestion {
                        key: id.to_string(),
                        unit: view.and_then(|v| v.unit.clone()),
                        value: view.map(|v| v.value.clone()).unwrap_or_default(),
                        ..base
                    }
                }
            }
        })
        .collect())
}

fn row(db: &Db, year: i32) -> AppResult<Option<YearCheckRow>> {
    db.read(|c| {
        let mut query = c.prepare(&format!(
            "SELECT id, year, completed_at FROM {TABLE} WHERE year = ?"
        ))?;
        let mut rows = query.query_map([year], |r| {
            Ok(YearCheckRow {
                id: r.get(0)?,
                year: r.get(1)?,
                completed_at: r.get(2)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    })
}

fn local_date(ms: i64) -> Option<NaiveDate> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|t| t.date_naive())
}

fn year_end(year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or_default()
}

/// The local day of the year's last completed run; None before the first.
pub fn last_run(db: &Db, year: i32) -> AppResult<Option<NaiveDate>> {
    Ok(row(db, year)?.and_then(|r| local_date(r.completed_at)))
}

fn year_row(db: &Db, year: i32) -> AppResult<taxyears::TaxYearRow> {
    db.read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(format!("No tax year {year}")))
}

fn ruleset_of(db: &Db, year: i32) -> AppResult<(taxyears::TaxYearRow, Ruleset)> {
    let row = year_row(db, year)?;
    let ruleset = serde_json::from_str(&row.rules)?;
    Ok((row, ruleset))
}

/// A run after 31 Dec of `year`, the one Close Year asks for.
fn after_year_end(last: Option<NaiveDate>, year: i32) -> bool {
    last.is_some_and(|d| d > year_end(year))
}

/// Whether Close Year should ask for a run first: the ruleset asks something and no run was completed after
/// 31 Dec.
pub fn close_needs_run(db: &Db, year: i32) -> AppResult<bool> {
    let (_, ruleset) = ruleset_of(db, year)?;
    Ok(!asked(&ruleset).is_empty() && !after_year_end(last_run(db, year)?, year))
}

/// Whether Today shows "Year check for <year>": from 1 Nov of an open year that asks something, until a run in
/// November or December; after 31 Dec until a run after it.
pub fn due_on(db: &Db, year: i32, today: NaiveDate) -> AppResult<bool> {
    let (row, ruleset) = ruleset_of(db, year)?;
    let Some(november) = NaiveDate::from_ymd_opt(year, 11, 1) else {
        return Ok(false);
    };
    if row.state != YearState::Open || asked(&ruleset).is_empty() || today < november {
        return Ok(false);
    }
    let since = if today > year_end(year) {
        year_end(year) + chrono::Days::new(1)
    } else {
        november
    };
    Ok(!last_run(db, year)?.is_some_and(|d| d >= since))
}

pub fn view(
    db: &Db,
    year: i32,
    today: NaiveDate,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<YearCheckView> {
    let (row, ruleset) = ruleset_of(db, year)?;
    let checks = questions(db, &ruleset, format)?;
    let last = last_run(db, year)?;
    let year_over = today > year_end(year);
    let after_end = after_year_end(last, year);
    let label = last.map(|d| date::day_month_year(d, order));
    let line = match (&label, after_end, year_over) {
        _ if checks.is_empty() => format!("Year check: rules-{year}.json asks nothing yet"),
        (None, ..) => "Year check: not run yet".to_string(),
        (Some(l), true, _) => format!("Year check: last run {l}"),
        (Some(l), false, false) => format!("Year check: last run {l} · run it again after 31 Dec"),
        (Some(l), false, true) => {
            format!("Year check: last run {l} · purchases and trips since then may be missing")
        }
    };
    let count = checks.len();
    let close_prompt = (count > 0 && !after_end).then(|| {
        let minute = format!(
            "It takes a minute: {count} question{}, most answered with No.",
            if count == 1 { "" } else { "s" }
        );
        match &label {
            None => format!("The year check for {year} hasn't run yet. {minute}"),
            Some(l) => format!(
                "The last year check for {year} was on {l}, before the year ended. Purchases and trips since then may be missing. {minute}"
            ),
        }
    });
    Ok(YearCheckView {
        year,
        open: row.state == YearState::Open,
        questions: checks,
        last_run: label,
        year_over,
        line,
        close_prompt,
    })
}

/// Records a completed run of an open year: one op, undoable like any other.
pub fn complete(db: &Db, year: i32, now: i64, source: Source) -> AppResult<Written> {
    if year_row(db, year)?.state == YearState::Closed {
        return Err(AppError::InvalidInput(format!(
            "Tax year {year} is closed; reopen it to run the year check"
        )));
    }
    let after = YearCheckRow {
        id: year.to_string(),
        year,
        completed_at: now,
    };
    let change = match row(db, year)? {
        Some(before) => Change::update(TABLE, &before, &after)?,
        None => Change::insert(TABLE, &after)?,
    };
    db.write(
        Op::new(format!("Complete Year Check {year}"), vec![change]),
        source,
    )
}

/// The open years Today offers a year check for.
pub fn due_years(db: &Db, today: NaiveDate) -> AppResult<Vec<i32>> {
    let years = db.read(taxyears::rows)?;
    let mut out = Vec::new();
    for y in years.iter().filter(|y| y.state == YearState::Open) {
        if due_on(db, y.year, today)? {
            out.push(y.year);
        }
    }
    out.sort_unstable();
    Ok(out)
}
