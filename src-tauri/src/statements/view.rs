//! What the Statements page shows: coverage per account, with dates already formatted (ADR 0010).

use chrono::{Datelike, Days, Months, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use super::store::{self, StatementRow};
use crate::accounts::{self, AccountRow};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::tr;

const MONTHS_SHOWN: u32 = 12;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Cover {
    Full,
    Part,
    Missing,
    /// Before the account's first statement, so nothing is missing.
    Before,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum AccountState {
    Ok,
    Behind,
    /// A statement in the chain was never imported.
    Gap,
    /// Up to date, with lines waiting in Review.
    Review,
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
    /// Lines neither a rule nor the owner decided.
    pub to_review: u32,
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

pub(crate) fn follows(previous: &StatementRow, s: &StatementRow) -> bool {
    (s.year == previous.year && s.number == previous.number + 1)
        || (s.year == previous.year + 1 && s.number == 1)
}

/// Each statement's days with its label. A statement runs from its first to its last booking, so one that
/// directly follows the previous statement also covers the quiet days since that one ended.
fn ranges(statements: &[StatementRow]) -> Vec<(NaiveDate, NaiveDate, String)> {
    let mut sorted: Vec<&StatementRow> = statements.iter().collect();
    sorted.sort_by_key(|s| (s.year, s.number));
    let mut out = Vec::new();
    for (i, s) in sorted.iter().enumerate() {
        let (Some(from), Some(to)) = (date::parse(&s.from_date), date::parse(&s.to_date)) else {
            continue;
        };
        let from = i
            .checked_sub(1)
            .map(|p| sorted[p])
            .filter(|previous| follows(previous, s))
            .and_then(|previous| date::parse(&previous.to_date))
            .map_or(from, |end| from.min(end + Days::new(1)));
        out.push((from, to, format!("Nr. {:03}/{}", s.number, s.year)));
    }
    out
}

fn month_cover(first: NaiveDate, today: NaiveDate, statements: &[StatementRow]) -> MonthCover {
    let next = first.checked_add_months(Months::new(1)).unwrap_or(first);
    let last = (next - Days::new(1)).min(today);
    let ranges = ranges(statements);
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
    let first_statement = ranges.iter().map(|(from, _, _)| *from).min();
    if first_statement.is_none_or(|start| last < start) {
        return MonthCover {
            cover: Cover::Before,
            tip: tr!(
                "{heading} · before the first statement",
                "{heading} · vor dem ersten Kontoauszug"
            ),
        };
    }
    let (cover, tip) = match covered {
        0 => (
            Cover::Missing,
            tr!("{heading} · no statement", "{heading} · kein Kontoauszug"),
        ),
        n if n == days => (Cover::Full, format!("{heading} · {}", names.join(", "))),
        _ => (
            Cover::Part,
            tr!(
                "{heading} · partly, {}",
                "{heading} · teilweise, {}",
                names.join(", ")
            ),
        ),
    };
    MonthCover { cover, tip }
}

/// The first break in the chain of statements, oldest first.
fn chain_note(statements: &[StatementRow], order: DateOrder) -> Option<String> {
    let mut sorted: Vec<&StatementRow> = statements.iter().collect();
    sorted.sort_by_key(|s| (s.year, s.number));
    for pair in sorted.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if !follows(a, b) {
            let (year, number) = if b.year == a.year {
                (a.year, a.number + 1)
            } else {
                (a.year + 1, 1)
            };
            let day = |text: &str| date::parse(text).map(|d| date::day_month(d, order));
            let between = match (day(&a.to_date), day(&b.from_date)) {
                (Some(end), Some(start)) => tr!(
                    ": no lines between {end} and {start}",
                    ": keine Buchungen zwischen {end} und {start}"
                ),
                _ => String::new(),
            };
            return Some(tr!(
                "Statement Nr. {number:03}/{year} was never imported{between}.",
                "Kontoauszug Nr. {number:03}/{year} wurde nie importiert{between}."
            ));
        }
        if b.old_balance != a.new_balance {
            return Some(tr!(
                "The balances of Nr. {:03}/{} and Nr. {:03}/{} don't connect.",
                "Die Salden von Nr. {:03}/{} und Nr. {:03}/{} schließen nicht aneinander an.",
                a.number,
                a.year,
                b.number,
                b.year
            ));
        }
    }
    None
}

fn account_coverage(
    account: &AccountRow,
    statements: &[StatementRow],
    to_review: u32,
    today: NaiveDate,
    order: DateOrder,
    import_after_days: u32,
) -> AccountCoverage {
    let note = chain_note(statements, order);
    let newest = statements.iter().max_by_key(|s| (s.year, s.number));
    let (last, state, state_label) =
        match newest.and_then(|s| date::parse(&s.to_date).map(|end| (s, end))) {
            None => (
                tr!("No statement yet", "Noch kein Kontoauszug"),
                AccountState::Behind,
                tr!("No statements", "Keine Kontoauszüge"),
            ),
            Some((s, end)) => {
                let behind = (today - end).num_days();
                let last = tr!(
                    "Nr. {:03} · until {}",
                    "Nr. {:03} · bis {}",
                    s.number,
                    date::day_month(end, order)
                );
                if behind > i64::from(import_after_days) {
                    let label = tr!("{behind} days behind", "{behind} Tage im Rückstand");
                    (last, AccountState::Behind, label)
                } else if note.is_some() {
                    let label = tr!("Statement missing", "Kontoauszug fehlt");
                    (last, AccountState::Gap, label)
                } else if to_review > 0 {
                    let label = tr!("{to_review} to review", "{to_review} zu prüfen");
                    (last, AccountState::Review, label)
                } else {
                    (last, AccountState::Ok, tr!("Up to date", "Aktuell"))
                }
            }
        };
    AccountCoverage {
        id: account.id.clone(),
        name: account.name.clone(),
        last,
        state,
        state_label,
        to_review,
        months: months(today)
            .into_iter()
            .map(|m| month_cover(m, today, statements))
            .collect(),
        note,
    }
}

/// `query` keeps only the accounts whose name contains it; an account is behind `import_after_days` after its
/// last statement ends.
pub fn coverage(
    db: &Db,
    today: NaiveDate,
    query: &str,
    order: DateOrder,
    import_after_days: u32,
) -> AppResult<Coverage> {
    let needle = query.trim().to_lowercase();
    let mut accounts = Vec::new();
    let waiting = db.read(crate::review::undecided_by_account)?;
    for account in db
        .read(accounts::rows)?
        .into_iter()
        .filter(|a| a.name.to_lowercase().contains(&needle))
    {
        let statements = db.read(|c| store::statements_of(c, &account.id))?;
        let to_review = waiting.get(&account.id).copied().unwrap_or(0);
        accounts.push(account_coverage(
            &account,
            &statements,
            to_review,
            today,
            order,
            import_after_days,
        ));
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
