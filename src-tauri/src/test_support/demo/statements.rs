//! The demo statements: a year of made-up months for each demo account, imported through the real pipeline.
//! Household misses February, so the Statements page shows a gap.

use std::path::Path;

use chrono::{Datelike, Days, Months, NaiveDate};

use crate::db::{Db, Source};
use crate::error::AppResult;
use crate::statements::import;
use crate::test_support::statements::{Statement, StatementLine, demo_month};

const HOUSEHOLD_IBAN: &str = "AT94 1999 9000 0000 0002";

struct DemoAccount {
    holder: &'static str,
    /// As printed.
    iban: &'static str,
    first_month: u32,
    never_imported: &'static [u32],
    /// A monthly standing order to the household account.
    pays_household: bool,
}

const ACCOUNTS: [DemoAccount; 3] = [
    DemoAccount {
        holder: "Private",
        iban: "AT24 1999 9000 0000 0001",
        first_month: 1,
        never_imported: &[],
        pays_household: true,
    },
    DemoAccount {
        holder: "Household",
        iban: HOUSEHOLD_IBAN,
        first_month: 1,
        never_imported: &[2],
        pays_household: false,
    },
    DemoAccount {
        holder: "Business",
        iban: "AT67 1999 9000 0000 0003",
        first_month: 9,
        never_imported: &[],
        pays_household: false,
    },
];

const OPENING_BALANCE: i64 = 141_230;
/// The demo's current month ends a few days before today, as a bank's latest statement does.
const POSTING_LAG_DAYS: u64 = 3;
const SEED_IMPORTED_AT: i64 = 0;

fn month_end(first: NaiveDate) -> NaiveDate {
    first
        .checked_add_months(Months::new(1))
        .map_or(first, |next| next - Days::new(1))
}

/// The month's statement for the account, cut off at `until`.
fn statement(
    account: &DemoAccount,
    number: u32,
    first: NaiveDate,
    until: NaiveDate,
    old_balance: i64,
) -> Statement {
    let mut s = demo_month(number, first.year(), first.month(), old_balance);
    s.holder = format!("Alex Beispiel ({})", account.holder);
    s.iban = account.iban.to_string();
    s.to = month_end(first).min(until);
    if account.pays_household {
        // The bank prints the receiving account's name, not its IBAN.
        s.lines.push(StatementLine {
            text: "Haushalt".into(),
            extra: vec!["HOUSEHOLD".into()],
            value_date: first + Days::new(1),
            cents: -50_000,
        });
        s.lines.sort_by_key(|l| l.value_date);
    }
    s.lines.retain(|l| l.value_date <= s.to);
    s
}

/// Imports the demo statements up to `today`; returns how many.
pub fn seed(db: &Db, config_dir: &Path, today: NaiveDate) -> AppResult<usize> {
    let year = today.year();
    let mut count = 0;
    for account in &ACCOUNTS {
        let mut balance = OPENING_BALANCE;
        for month in account.first_month..=today.month() {
            let first = NaiveDate::from_ymd_opt(year, month, 1).expect("month is 1..=12");
            let until = if month == today.month() {
                today - Days::new(POSTING_LAG_DAYS)
            } else {
                month_end(first)
            };
            if until < first {
                continue;
            }
            let s = statement(account, month, first, until, balance);
            balance = s.new_balance();
            if account.never_imported.contains(&month) {
                continue;
            }
            let pending = import::prepare(db, config_dir, s.pdf())?;
            import::commit(db, config_dir, pending, SEED_IMPORTED_AT, Source::Seed)?;
            count += 1;
        }
    }
    Ok(count)
}
