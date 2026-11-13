//! The Today page: the single most useful next step, the other open ones, the next charges, the newest open
//! tax year and how far the statements reach. Every fact comes from the page that owns it.

pub mod commands;
#[cfg(test)]
mod tests;

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::statements::view::{self as coverage, AccountState};
use crate::subscriptions::Book;
use crate::subscriptions::view::{Format, letter};
use crate::taxyears::{self, YearState};
use crate::year::page::{self as year_page, AttentionKind, YearPage};

/// Where a step's button leads.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum StepTarget {
    Review,
    Year { year: i32 },
    Import,
    Document { id: String, year: i32 },
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Step {
    /// `review`, `attach`, `import`, `questions`, `draft`: also the dot's colour.
    pub key: String,
    /// `12 new transactions to review`.
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

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `A`, `A and B`, `A, B and C`.
fn joined(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn review_step(db: &Db) -> AppResult<Option<Step>> {
    let waiting = db.read(crate::review::undecided_by_account)?;
    let count: u32 = waiting.values().sum();
    if count == 0 {
        return Ok(None);
    }
    let names: Vec<String> = db
        .read(accounts::rows)?
        .into_iter()
        .filter(|a| waiting.get(&a.id).is_some_and(|n| *n > 0))
        .map(|a| a.name)
        .collect();
    let minutes = (count as usize).div_ceil(6).max(1);
    Ok(Some(Step {
        key: "review".into(),
        title: plural(
            count as usize,
            "new transaction to review",
            "new transactions to review",
        ),
        detail: format!("about {}", plural(minutes, "minute", "minutes")),
        line: format!("From {}. Rules decided the rest.", joined(&names)),
        button: "Start Review".into(),
        target: StepTarget::Review,
    }))
}

fn import_step(db: &Db, today: NaiveDate, order: DateOrder) -> AppResult<Option<Step>> {
    let view = coverage::coverage(db, today, "", order)?;
    let Some(behind) = view
        .accounts
        .iter()
        .find(|a| a.state == AccountState::Behind)
    else {
        return Ok(None);
    };
    let (title, detail, line) = match last_statement_end(db, &behind.id)? {
        Some(end) => {
            let ends = date::day_month(end, order);
            (
                format!(
                    "{} statement is {}",
                    behind.name,
                    behind.state_label.to_lowercase()
                ),
                format!("last one ends {ends}"),
                format!("Its last statement ends on {ends}."),
            )
        }
        None => (
            format!("{} has no statement yet", behind.name),
            "import its first one".to_string(),
            "Import its first statement to see its lines.".to_string(),
        ),
    };
    Ok(Some(Step {
        key: "import".into(),
        title,
        detail,
        line,
        button: "Import Statement…".into(),
        target: StepTarget::Import,
    }))
}

/// The first two distinct names, in order.
fn first_names(names: impl Iterator<Item = String>) -> String {
    let mut distinct: Vec<String> = Vec::new();
    for name in names {
        if !distinct.contains(&name) {
            distinct.push(name);
        }
        if distinct.len() == 2 {
            break;
        }
    }
    joined(&distinct)
}

fn attach_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let missing: Vec<_> = page
        .attention
        .iter()
        .filter(|a| a.kind == AttentionKind::MissingInvoice)
        .collect();
    (!missing.is_empty()).then(|| Step {
        key: "attach".into(),
        title: plural(missing.len(), "invoice to attach", "invoices to attach"),
        // An attention title reads `<payee> · <amount>`.
        detail: first_names(
            missing
                .iter()
                .map(|a| a.title.split(" · ").next().unwrap_or(&a.title).to_string()),
        ),
        line: "They count once their PDF is attached.".into(),
        button: format!("Show in Tax Year {year}"),
        target: StepTarget::Year { year },
    })
}

fn questions_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let open: Vec<&str> = page
        .questions
        .iter()
        .filter(|q| !q.answered)
        .map(|q| q.label.as_str())
        .collect();
    (!open.is_empty()).then(|| Step {
        key: "questions".into(),
        title: format!("{} for Tax year {year}", plural(open.len(), "question", "questions")),
        detail: open.join(" · "),
        line: "Numbers from your year, such as the Lohnzettel. They decide what the tax office accepts.".into(),
        button: format!("Answer in Tax Year {year}"),
        target: StepTarget::Year { year },
    })
}

fn draft_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let drafts: Vec<_> = page
        .attention
        .iter()
        .filter(|a| a.kind == AttentionKind::Draft)
        .collect();
    let first = drafts.first()?;
    Some(Step {
        key: "draft".into(),
        title: plural(drafts.len(), "draft to finish", "drafts to finish"),
        detail: first_names(drafts.iter().map(|d| d.title.clone())),
        line: "Finish it to lock its values for the tax year.".into(),
        button: "Open Draft".into(),
        target: first
            .document_id
            .clone()
            .map_or(StepTarget::Year { year }, |id| StepTarget::Document {
                id,
                year,
            }),
    })
}

fn last_statement_end(db: &Db, account_id: &str) -> AppResult<Option<NaiveDate>> {
    Ok(db
        .read(|c| statements::statements_of(c, account_id))?
        .iter()
        .filter_map(|s| date::parse(&s.to_date))
        .max())
}

/// Accounts grouped by how far their statements reach; the ones behind come last.
fn reach(db: &Db, today: NaiveDate, order: DateOrder) -> AppResult<Vec<StatementReach>> {
    let view = coverage::coverage(db, today, "", order)?;
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
                Some(end) => format!("{} until {}", joined(&names), date::day_month(end, order)),
                None => format!("{}: none yet", joined(&names)),
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
                        i64::MIN..=0 => "today".into(),
                        1 => "tomorrow".into(),
                        2..=31 => format!("in {days} days"),
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
) -> AppResult<TodayView> {
    let format = Format { number, order };
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
        import_step(db, today, order)?,
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
        statements: reach(db, today, order)?,
    })
}
