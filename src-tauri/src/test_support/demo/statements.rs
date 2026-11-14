//! The demo statements: a year of made-up months for each demo account, imported through the real pipeline.
//! Each account has its own bookings: Private the salary and card payments, Household the shared costs paid
//! from Private's standing order, Business client income and its costs. Household misses February, so the
//! Statements page shows a gap.

use std::path::Path;

use chrono::{Datelike, Days, Months, NaiveDate};

use crate::db::{Db, Source};
use crate::error::AppResult;
use crate::statements::import;
use crate::test_support::statements::{Statement, StatementLine, demo_month};

#[derive(Clone, Copy, PartialEq)]
enum Bookings {
    Private,
    Household,
    Business,
}

struct DemoAccount {
    holder: &'static str,
    /// As printed.
    iban: &'static str,
    first_month: u32,
    never_imported: &'static [u32],
    bookings: Bookings,
}

/// The standing order from Private to Household, on both statements.
const HOUSEHOLD_SHARE: i64 = 50_000;

const ACCOUNTS: [DemoAccount; 3] = [
    DemoAccount {
        holder: "Private",
        iban: "AT24 1999 9000 0000 0001",
        first_month: 1,
        never_imported: &[],
        bookings: Bookings::Private,
    },
    DemoAccount {
        holder: "Household",
        iban: "AT94 1999 9000 0000 0002",
        first_month: 1,
        never_imported: &[2],
        bookings: Bookings::Household,
    },
    DemoAccount {
        holder: "Business",
        iban: "AT67 1999 9000 0000 0003",
        first_month: 9,
        never_imported: &[],
        bookings: Bookings::Business,
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

fn line(text: &str, extra: &[&str], value_date: NaiveDate, cents: i64) -> StatementLine {
    StatementLine {
        text: text.into(),
        extra: extra.iter().map(|e| (*e).to_string()).collect(),
        value_date,
        cents,
    }
}

/// `POS 85,40 AT K1 06.10. 18:05` with the merchant on the line below, as the bank prints a card payment.
fn card(
    kind: &str,
    cents: i64,
    day: NaiveDate,
    time: &str,
    merchant: &str,
    booked: u64,
) -> StatementLine {
    let paid = format!("{},{:02}", cents / 100, cents % 100);
    let text = format!(
        "{kind:<6} {paid:>8} AT  K1   {}. {time}",
        day.format("%d.%m")
    );
    line(&text, &[merchant], day + Days::new(booked), -cents)
}

/// Shared costs, paid from Private's standing order.
fn household_lines(first: NaiveDate) -> Vec<StatementLine> {
    let d = |day: u64| first + Days::new(day - 1);
    let month = first.format("%m/%Y");
    vec![
        line("Haushalt", &["Alex Beispiel"], d(2), HOUSEHOLD_SHARE),
        card(
            "POS",
            8_540,
            d(6),
            "18:05",
            "SPAR MARKT DORNBIRN 6850 040",
            1,
        ),
        line(
            &format!("Strom {month}"),
            &[
                "STADTWERKE MUSTER",
                "MDID:000412345",
                &format!("Strom {month}"),
            ],
            d(15),
            -6_400,
        ),
        card(
            "POS",
            6_215,
            d(20),
            "11:30",
            "SPAR MARKT DORNBIRN 6850 040",
            1,
        ),
        line(
            "Polizze Nr. 4400-1122",
            &["MUSTER VERSICHERUNG AG", "Polizze Nr. 4400-1122"],
            d(25),
            -3_890,
        ),
    ]
}

/// Client income and the business's own costs.
fn business_lines(first: NaiveDate) -> Vec<StatementLine> {
    let d = |day: u64| first + Days::new(day - 1);
    let month = first.month();
    vec![
        line(
            &format!("RE 2026-{month:03}"),
            &["KUNDE BEISPIEL GMBH", &format!("RE 2026-{month:03}")],
            d(4),
            240_000,
        ),
        card("E-COMM", 2_900, d(9), "00:00", "CLOUDDESK DUBLIN 372", 1),
        line("Kontoführung", &[], d(28), -990),
    ]
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
    match account.bookings {
        Bookings::Private => {
            // The bank prints the receiving account's name, not its IBAN.
            s.lines.push(line(
                "Haushalt",
                &["HOUSEHOLD"],
                first + Days::new(1),
                -HOUSEHOLD_SHARE,
            ));
        }
        Bookings::Household => s.lines = household_lines(first),
        Bookings::Business => s.lines = business_lines(first),
    }
    s.lines.sort_by_key(|l| l.value_date);
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
            // A statement prints its period from its bookings, so a month with none yet has no statement.
            if s.lines.is_empty() {
                continue;
            }
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
