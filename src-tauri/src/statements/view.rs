//! What the Statements and Account pages show, with dates and amounts already formatted (ADR 0010).

use chrono::{Datelike, Days, Months, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use super::store::{self, StatementRow, TransactionRow};
use crate::accounts::{self, AccountRow};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money, format_signed};
use crate::settings::NumberFormat;

/// Days after its last statement's end an account still counts as up to date.
const UP_TO_DATE_DAYS: i64 = 7;
const MONTHS_SHOWN: u32 = 12;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Cover {
    Full,
    Part,
    Missing,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum AccountState {
    Ok,
    Behind,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct MonthHead {
    pub label: String,
    /// The first month of a year, which gets a stronger mark.
    pub january: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct MonthCover {
    pub cover: Cover,
    pub tip: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AccountCoverage {
    pub id: String,
    pub name: String,
    /// `Nr. 003 · until 10 Nov`.
    pub last: String,
    pub state: AccountState,
    /// `Up to date`, `13 days behind`.
    pub state_label: String,
    pub months: Vec<MonthCover>,
    /// A gap in the chain of statements, in words.
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Coverage {
    pub months: Vec<MonthHead>,
    pub accounts: Vec<AccountCoverage>,
}

/// The last `MONTHS_SHOWN` months up to `today`'s, oldest first.
fn months(today: NaiveDate) -> Vec<NaiveDate> {
    let this = today.with_day(1).unwrap_or(today);
    (0..MONTHS_SHOWN)
        .rev()
        .filter_map(|back| this.checked_sub_months(Months::new(back)))
        .collect()
}

fn month_cover(first: NaiveDate, today: NaiveDate, statements: &[StatementRow]) -> MonthCover {
    let next = first.checked_add_months(Months::new(1)).unwrap_or(first);
    let last = (next - Days::new(1)).min(today);
    let ranges: Vec<(NaiveDate, NaiveDate, String)> = statements
        .iter()
        .filter_map(|s| {
            Some((
                date::parse(&s.from_date)?,
                date::parse(&s.to_date)?,
                format!("Nr. {:03}/{}", s.number, s.year),
            ))
        })
        .collect();
    let days = (last - first).num_days().max(-1) + 1;
    let covered = (0..days)
        .map(|i| first + Days::new(i as u64))
        .filter(|d| ranges.iter().any(|(from, to, _)| (*from..=*to).contains(d)))
        .count() as i64;
    let names: Vec<&str> = ranges
        .iter()
        .filter(|(from, to, _)| *from <= last && *to >= first)
        .map(|(_, _, label)| label.as_str())
        .collect();
    let heading = date::month_year(first);
    let (cover, tip) = match covered {
        0 => (Cover::Missing, format!("{heading} · no statement")),
        n if n == days => (Cover::Full, format!("{heading} · {}", names.join(", "))),
        _ => (
            Cover::Part,
            format!("{heading} · partly, {}", names.join(", ")),
        ),
    };
    MonthCover { cover, tip }
}

/// The first break in the chain of statements, oldest first.
fn chain_note(statements: &[StatementRow]) -> Option<String> {
    let mut sorted: Vec<&StatementRow> = statements.iter().collect();
    sorted.sort_by_key(|s| (s.year, s.number));
    for pair in sorted.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (year, number) = if b.year == a.year {
            (a.year, a.number + 1)
        } else {
            (a.year + 1, 1)
        };
        if (b.year, b.number) != (year, number) {
            let month = date::parse(&a.to_date)
                .map(|d| date::month(d + Days::new(1)))
                .unwrap_or("A month");
            return Some(format!(
                "{month} is missing: statement Nr. {number:03}/{year} was never imported."
            ));
        }
        if b.old_balance != a.new_balance {
            return Some(format!(
                "The balances of Nr. {:03}/{} and Nr. {:03}/{} don't connect.",
                a.number, a.year, b.number, b.year
            ));
        }
    }
    None
}

fn account_coverage(
    account: &AccountRow,
    statements: &[StatementRow],
    today: NaiveDate,
    order: DateOrder,
) -> AccountCoverage {
    let newest = statements.iter().max_by_key(|s| (s.year, s.number));
    let (last, state, state_label) =
        match newest.and_then(|s| date::parse(&s.to_date).map(|end| (s, end))) {
            None => (
                "No statement yet".to_string(),
                AccountState::Behind,
                "No statements".to_string(),
            ),
            Some((s, end)) => {
                let behind = (today - end).num_days();
                let last = format!(
                    "Nr. {:03} · until {}",
                    s.number,
                    date::day_month(end, order)
                );
                if behind > UP_TO_DATE_DAYS {
                    (last, AccountState::Behind, format!("{behind} days behind"))
                } else {
                    (last, AccountState::Ok, "Up to date".to_string())
                }
            }
        };
    AccountCoverage {
        id: account.id.clone(),
        name: account.name.clone(),
        last,
        state,
        state_label,
        months: months(today)
            .into_iter()
            .map(|m| month_cover(m, today, statements))
            .collect(),
        note: chain_note(statements),
    }
}

pub fn coverage(db: &Db, today: NaiveDate, order: DateOrder) -> AppResult<Coverage> {
    let mut accounts = Vec::new();
    for account in db.read(accounts::rows)? {
        let statements = db.read(|c| store::statements_of(c, &account.id))?;
        accounts.push(account_coverage(&account, &statements, today, order));
    }
    Ok(Coverage {
        months: months(today)
            .into_iter()
            .map(|m| MonthHead {
                label: date::short_month(m).to_string(),
                january: m.month() == 1,
            })
            .collect(),
        accounts,
    })
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct LineView {
    pub id: String,
    /// `10 Nov`.
    pub date: String,
    /// The merchant, the counterparty or the booking text.
    pub title: String,
    /// The foreign amount of a card payment, `USD 24.00`.
    pub original: Option<String>,
    /// `−€22.48`, `+€2,850.00`.
    pub amount: String,
    pub credit: bool,
    pub transfer: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementGroup {
    pub id: String,
    /// `Nr. 003/2026`.
    pub label: String,
    /// `7 Sep – 10 Nov`.
    pub range: String,
    /// `62 lines`.
    pub count: String,
    pub lines: Vec<LineView>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AccountLines {
    pub id: String,
    pub name: String,
    pub iban_masked: String,
    pub groups: Vec<StatementGroup>,
}

fn line_view(t: TransactionRow, format: NumberFormat, order: DateOrder) -> LineView {
    let title = t.merchant.or(t.counterparty).unwrap_or(t.text);
    let original = match (t.original_cents, t.original_currency) {
        (Some(cents), Some(currency)) => Some(format_money(&Money { cents, currency }, format)),
        _ => None,
    };
    LineView {
        date: date::parse(&t.value_date)
            .map(|d| date::day_month(d, order))
            .unwrap_or_default(),
        title,
        original,
        amount: format_signed(&Money::eur(t.cents), format),
        credit: t.cents > 0,
        transfer: t.transfer,
        id: t.id,
    }
}

/// The account's lines grouped by statement, newest first.
pub fn account_lines(
    db: &Db,
    account_id: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<AccountLines> {
    let account = db
        .read(accounts::rows)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::NotFound(format!("No account {account_id}")))?;
    let mut groups = Vec::new();
    for s in db.read(|c| store::statements_of(c, account_id))? {
        let lines = db.read(|c| store::transactions_of(c, &s.id))?;
        let range = match (date::parse(&s.from_date), date::parse(&s.to_date)) {
            (Some(from), Some(to)) => format!(
                "{} – {}",
                date::day_month(from, order),
                date::day_month(to, order)
            ),
            _ => String::new(),
        };
        groups.push(StatementGroup {
            label: format!("Nr. {:03}/{}", s.number, s.year),
            range,
            count: format!(
                "{} {}",
                lines.len(),
                if lines.len() == 1 { "line" } else { "lines" }
            ),
            lines: lines
                .into_iter()
                .map(|t| line_view(t, format, order))
                .collect(),
            id: s.id,
        });
    }
    Ok(AccountLines {
        iban_masked: accounts::mask_iban(&account.iban),
        id: account.id,
        name: account.name,
        groups,
    })
}
