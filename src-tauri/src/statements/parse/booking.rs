//! Printed dates, and what a booking's text and detail lines say: card, FX, counterparty and its IBAN.

use chrono::{Datelike, NaiveDate};

use super::amount;
use super::model::{Card, CardKind, Fx, ParsedLine};
use crate::accounts::normalize_iban;

/// A `dd.mm.yyyy` date.
pub fn date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%d.%m.%Y").ok()
}

/// The last whitespace-separated token of a line and the rest before it.
pub fn split_last(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_end();
    let at = line.rfind(char::is_whitespace)?;
    Some((line[..at].trim_end(), &line[at + 1..]))
}

/// Value dates may run a few days past the period's end (bookings dated into the next statement).
const SLACK_DAYS: u64 = 7;

/// A `dd.mm.` date in the statement's period; December dates in a January statement belong to the year before.
pub fn day_in_period(text: &str, from: NaiveDate, to: NaiveDate) -> Option<NaiveDate> {
    let (day, month) = text.strip_suffix('.')?.split_once('.')?;
    let (day, month): (u32, u32) = (day.parse().ok()?, month.parse().ok()?);
    let in_year = |year| NaiveDate::from_ymd_opt(year, month, day);
    match in_year(to.year()) {
        Some(d) if d <= to + chrono::Days::new(SLACK_DAYS) => Some(d),
        _ => in_year(from.year()).or_else(|| in_year(to.year() - 1)),
    }
}

/// Fills in what the text and the detail lines say: card, FX, counterparty and its IBAN.
pub fn details(line: &mut ParsedLine, from: NaiveDate, to: NaiveDate) {
    line.card = card(&line.text, line.extra.first(), from, to);
    line.fx = line.extra.iter().find_map(|e| fx(e));
    if line.card.is_none() {
        line.counterparty = line
            .extra
            .first()
            .filter(|e| normalize_iban(e).is_none())
            .cloned();
        line.counterparty_iban = line.extra.iter().find_map(|e| normalize_iban(e));
    }
}

/// `E-COMM 24,00 USD K1 08.11. LUMEN AI SAN FRANCISCO 840`, `POS 58,30 AT K1 04.11. 17:40 TANKSTELLE ...` or a
/// terminal's name and number, `BAECKEREI 2373 K1 30.05. 10:54`. A merchant missing from the text is on the
/// first detail line.
fn card(text: &str, first_extra: Option<&String>, from: NaiveDate, to: NaiveDate) -> Option<Card> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let k = words.iter().position(|w| is_card_number(w))?;
    let prefix = prefix(&words[..k])?;
    let date = day_in_period(words.get(k + 1)?, from, to)?;
    let (time, rest) = match &words[k + 2..] {
        // Online payments print midnight when the bank has no time.
        [first, tail @ ..] if is_clock_time(first) => (
            (prefix.kind == CardKind::Terminal || *first != "00:00").then(|| first.to_string()),
            tail,
        ),
        rest => (None, rest),
    };
    let mut merchant = without_country_code(rest);
    if merchant.is_empty() {
        merchant = prefix
            .terminal_name
            .or_else(|| first_extra.map(|e| merchant_from_detail(e)))
            .unwrap_or_default();
    }
    Some(Card {
        kind: prefix.kind,
        date,
        time,
        merchant,
        original: prefix.original,
    })
}

/// What the words before the card number say.
struct Prefix {
    kind: CardKind,
    /// The amount in a foreign currency, with its code.
    original: Option<(i64, String)>,
    terminal_name: Option<String>,
}

fn prefix(words: &[&str]) -> Option<Prefix> {
    match *words {
        ["E-COMM", original_amount, currency] => paid(CardKind::Online, original_amount, currency),
        ["POS", original_amount, currency] => paid(CardKind::Terminal, original_amount, currency),
        [ref name @ .., terminal] if !name.is_empty() && is_number(terminal) => Some(Prefix {
            kind: CardKind::Terminal,
            original: None,
            terminal_name: Some(name.join(" ")),
        }),
        _ => None,
    }
}

/// `24,00 USD`; a country code (`AT`) or `EUR` in place of the currency means euros.
fn paid(kind: CardKind, original_amount: &str, currency: &str) -> Option<Prefix> {
    let original_amount = amount(original_amount)?;
    let foreign = currency.len() == 3 && currency != "EUR";
    Some(Prefix {
        kind,
        original: foreign.then(|| (original_amount, currency.to_string())),
        terminal_name: None,
    })
}

fn merchant_from_detail(line: &str) -> String {
    without_country_code(&line.split_whitespace().collect::<Vec<_>>())
}

/// The words without the numeric country code card lines end with (040, 840).
fn without_country_code(words: &[&str]) -> String {
    let words = match words {
        [rest @ .., last] if last.len() == 3 && is_number(last) => rest,
        _ => words,
    };
    words.join(" ")
}

/// The card's number on the account, `K1`.
fn is_card_number(word: &str) -> bool {
    word.strip_prefix('K').is_some_and(is_number)
}

fn is_number(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| c.is_ascii_digit())
}

fn is_clock_time(word: &str) -> bool {
    word.len() == 5 && word.as_bytes()[2] == b':'
}

fn token_after<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.split_once(label)?.1.split_whitespace().next()
}

/// `SPESEN: 0,49 KURS: 1,0912 VOM: 06.11.2026`; a line without `SPESEN:` charged no fee.
fn fx(text: &str) -> Option<Fx> {
    let fee = match token_after(text, "SPESEN:") {
        Some(fee) => amount(fee)?,
        None => 0,
    };
    let rate = token_after(text, "KURS:")?;
    let day = token_after(text, "VOM:")?;
    Some(Fx {
        fee,
        rate: rate.to_string(),
        rate_date: date(day)?,
    })
}
