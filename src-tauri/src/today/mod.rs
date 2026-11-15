//! The Today page: the single most useful next step, the other open ones, the next charges, the newest open
//! tax year and how far the statements reach. Every fact comes from the page that owns it.

pub mod commands;
mod steps;
#[cfg(test)]
mod tests;

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::statements::view::{self as coverage, AccountState, Coverage};
use crate::subscriptions::Book;
use crate::subscriptions::view::{Format, letter};
use crate::taxyears::{self, YearState};
use crate::tr;
use crate::year::page::{self as year_page, AttentionKind, YearPage};
use steps::{attach_step, draft_step, import_step, joined, questions_step, review_step};

/// Where a step's button leads.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum StepTarget {
    Review,
    /// With `attention`, the tax year shows only what needs attention of that kind.
    Year {
        year: i32,
        attention: Option<AttentionKind>,
    },
    Import,
    Document {
        id: String,
        year: i32,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Step {
    /// `review`, `attach`, `import`, `gap`, `check`, `questions`, `draft`: also the dot's colour.
    pub key: String,
    /// `12 lines to review`.
    pub title: String,
    /// The quiet row's right side: `about 2 minutes`.
    pub detail: String,
    /// The headline's line under it.
    pub line: String,
    pub button: String,
    pub target: StepTarget,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct UpcomingCharge {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub tone: i64,
    pub anv: bool,
    /// `tomorrow`, `in 12 days`, `9 Feb 2027`.
    pub when: String,
    /// Within three days.
    pub soon: bool,
    pub amount: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct YearSummary {
    pub year: i32,
    pub total: String,
    pub progress: String,
    pub progress_percent: u32,
}

/// One part of the statements line: `Private and Business until 10 Nov`.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementReach {
    pub text: String,
    pub behind: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct TodayView {
    /// `Friday, 13 November`.
    pub date: String,
    /// The headline; None when nothing is open.
    pub next: Option<Step>,
    /// The other open steps as quiet rows.
    pub also: Vec<Step>,
    pub upcoming: Vec<UpcomingCharge>,
    pub year: Option<YearSummary>,
    pub statements: Vec<StatementReach>,
}

const UPCOMING: usize = 5;
const SOON_DAYS: i64 = 3;

pub(super) fn last_statement_end(db: &Db, account_id: &str) -> AppResult<Option<NaiveDate>> {
    Ok(db
        .read(|c| statements::statements_of(c, account_id))?
        .iter()
        .filter_map(|s| date::parse(&s.to_date))
        .max())
}

/// Accounts grouped by how far their statements reach; the ones behind come last.
fn reach(db: &Db, view: &Coverage, order: DateOrder) -> AppResult<Vec<StatementReach>> {
    let mut groups: Vec<(Option<NaiveDate>, bool, Vec<String>)> = Vec::new();
    for account in &view.accounts {
        let end = last_statement_end(db, &account.id)?;
        let behind = account.state == AccountState::Behind;
        match groups
            .iter_mut()
            .find(|(e, b, _)| *e == end && *b == behind)
        {
            Some((_, _, names)) => names.push(account.name.clone()),
            None => groups.push((end, behind, vec![account.name.clone()])),
        }
    }
    groups.sort_by_key(|(end, behind, _)| (*behind, std::cmp::Reverse(*end)));
    Ok(groups
        .into_iter()
        .map(|(end, behind, names)| StatementReach {
            text: match end {
                Some(end) => {
                    let names = joined(&names);
                    let day = date::day_month(end, order);
                    tr!("{names} until {day}", "{names} bis {day}")
                }
                None => {
                    let names = joined(&names);
                    tr!("{names}: none yet", "{names}: noch keiner")
                }
            },
            behind,
        })
        .collect())
}

fn upcoming(db: &Db, today: NaiveDate, format: &Format) -> AppResult<Vec<UpcomingCharge>> {
    let book = db.read(Book::load)?;
    let mut charges: Vec<(NaiveDate, UpcomingCharge)> = book
        .subscriptions
        .iter()
        .filter_map(|sub| {
            let running = book.running(sub);
            let next = running.next_charge(today)?;
            let days = (next - today).num_days();
            Some((
                next,
                UpcomingCharge {
                    id: sub.id.clone(),
                    name: sub.name.clone(),
                    letter: letter(&sub.name),
                    tone: sub.tone,
                    anv: sub.item_key.is_some(),
                    when: match days {
                        i64::MIN..=0 => tr!("today", "heute"),
                        1 => tr!("tomorrow", "morgen"),
                        2..=31 => tr!("in {days} days", "in {days} Tagen"),
                        _ => date::day_month_year(next, format.order),
                    },
                    soon: days <= SOON_DAYS,
                    amount: running
                        .charge_cents(today)
                        .map_or_else(String::new, |c| format.eur(c)),
                },
            ))
        })
        .collect();
    charges.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(&b.1.name)));
    Ok(charges.into_iter().take(UPCOMING).map(|(_, c)| c).collect())
}

pub fn today_view(
    db: &Db,
    today: NaiveDate,
    number: NumberFormat,
    order: DateOrder,
    import_after_days: u32,
) -> AppResult<TodayView> {
    let format = Format { number, order };
    let statements_view = coverage::coverage(db, today, "", order, import_after_days)?;
    let newest_open = db
        .read(taxyears::rows)?
        .into_iter()
        .find(|y| y.state == YearState::Open)
        .map(|y| y.year);
    let page = newest_open
        .map(|year| year_page::page(db, year, "", number, order))
        .transpose()?;
    let from_year = |step: fn(&YearPage) -> Option<Step>| page.as_ref().and_then(step);
    let mut steps = [
        review_step(db)?,
        from_year(attach_step),
        import_step(db, &statements_view, order)?,
        from_year(questions_step),
        from_year(draft_step),
    ]
    .into_iter()
    .flatten();
    Ok(TodayView {
        date: date::weekday_long(today, order),
        next: steps.next(),
        also: steps.collect(),
        upcoming: upcoming(db, today, &format)?,
        year: page.map(|p| YearSummary {
            year: p.year,
            total: p.total,
            progress: p.progress,
            progress_percent: p.progress_percent,
        }),
        statements: reach(db, &statements_view, order)?,
    })
}
