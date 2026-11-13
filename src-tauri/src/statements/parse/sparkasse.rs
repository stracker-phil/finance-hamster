//! The Sparkasse/Erste "Kontoauszug" layout: a fixed-width text layer with the header on every page, one line
//! per booking (text, value date `dd.mm.`, signed amount at the right edge) and indented detail lines below.

use chrono::{Datelike, NaiveDate};

use super::model::{Card, CardKind, Fx, ParsedLine, ParsedStatement};
use super::{ParseError, amount};
use crate::accounts::normalize_iban;

pub const LAYOUT: &str = "sparkasse-erste-1";

pub fn recognizes(pages: &[Vec<String>]) -> bool {
    pages.first().is_some_and(|page| {
        page.iter().any(|l| l.contains("Kontoauszug Nr."))
            && page.iter().any(|l| l.trim_start().starts_with("IBAN:"))
    })
}

fn date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%d.%m.%Y").ok()
}

/// Value dates may run a few days past the period's end (bookings dated into the next statement).
const SLACK_DAYS: u64 = 7;

/// The text after `label` on the first line that has it, across all pages.
fn field<'a>(pages: &'a [Vec<String>], label: &str) -> Result<&'a str, ParseError> {
    after(pages, label).ok_or_else(|| ParseError::Missing(label.into()))
}

/// The text after `label` on the first line that has it, across all pages.
fn after<'a>(pages: &'a [Vec<String>], label: &str) -> Option<&'a str> {
    pages
        .iter()
        .flatten()
        .find_map(|l| l.split_once(label).map(|(_, rest)| rest.trim()))
}

/// The last whitespace-separated token of a line and the rest before it.
fn split_last(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_end();
    let at = line.rfind(char::is_whitespace)?;
    Some((line[..at].trim_end(), &line[at + 1..]))
}

/// A `dd.mm.` value date in the statement's period; December lines in a January statement belong to the year
/// before.
fn day_in_period(text: &str, from: NaiveDate, to: NaiveDate) -> Option<NaiveDate> {
    let (day, month) = text.strip_suffix('.')?.split_once('.')?;
    let (day, month): (u32, u32) = (day.parse().ok()?, month.parse().ok()?);
    let in_year = |year| NaiveDate::from_ymd_opt(year, month, day);
    match in_year(to.year()) {
        Some(d) if d <= to + chrono::Days::new(SLACK_DAYS) => Some(d),
        _ => in_year(from.year()).or_else(|| in_year(to.year() - 1)),
    }
}

/// A summary line's amount: `Summe Gutschriften   +2.850,00`.
fn sum(pages: &[Vec<String>], label: &str) -> Result<(i64, usize), ParseError> {
    for (i, page) in pages.iter().enumerate() {
        for line in page {
            if line.trim_start().starts_with(label)
                && let Some((_, last)) = split_last(line)
                && let Some(cents) = amount(last)
            {
                return Ok((cents, i + 1));
            }
        }
    }
    Err(ParseError::Missing(label.into()))
}

pub fn parse(pages: &[Vec<String>]) -> Result<ParsedStatement, ParseError> {
    let number_text = field(pages, "Kontoauszug Nr.")?;
    let (number, year) = number_text
        .split_whitespace()
        .next()
        .and_then(|n| n.split_once('/'))
        .and_then(|(n, y)| Some((n.parse().ok()?, y.parse().ok()?)))
        .ok_or(ParseError::Unreadable(format!(
            "Kontoauszug Nr. {number_text}"
        )))?;
    let iban_text = after(pages, "IBAN:").ok_or_else(|| ParseError::Missing("IBAN".into()))?;
    let iban_text = iban_text
        .split_once("BIC:")
        .map_or(iban_text, |(iban, _)| iban);
    let iban = normalize_iban(iban_text)
        .ok_or(ParseError::Unreadable(format!("IBAN {}", iban_text.trim())))?;
    let period = after(pages, "Zeitraum:").ok_or_else(|| ParseError::Missing("Zeitraum".into()))?;
    let (from, to) = period
        .split_once(" - ")
        .and_then(|(f, t)| Some((date(f)?, date(t)?)))
        .ok_or(ParseError::Unreadable(format!("Zeitraum {period}")))?;
    let (old_balance, _) = sum(pages, "Alter Saldo")?;
    let (credits, sums_page) = sum(pages, "Summe Gutschriften")?;
    let (debits, _) = sum(pages, "Summe Lastschriften")?;
    let (new_balance, _) = sum(pages, "Neuer Saldo")?;
    Ok(ParsedStatement {
        layout: LAYOUT,
        iban,
        number,
        year,
        from,
        to,
        old_balance,
        credits,
        debits,
        new_balance,
        sums_page,
        lines: lines(pages, from, to)?,
    })
}

/// The booking lines: a line with a value date and an amount starts a booking, indented lines below add to it.
fn lines(
    pages: &[Vec<String>],
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<ParsedLine>, ParseError> {
    let mut out: Vec<ParsedLine> = Vec::new();
    for (p, page) in pages.iter().enumerate() {
        let Some((first_row, body)) = body(page) else {
            continue;
        };
        for (i, line) in body.iter().enumerate() {
            if line.trim_start().starts_with("Alter Saldo") {
                continue;
            }
            if line.starts_with("    ") {
                let last = out.last_mut().ok_or_else(|| {
                    ParseError::Unreadable(format!("page {}: {}", p + 1, line.trim()))
                })?;
                last.extra.push(line.trim().to_string());
                continue;
            }
            out.push(booking(line, p + 1, first_row + i, from, to)?);
        }
    }
    for line in &mut out {
        details(line, from, to);
    }
    Ok(out)
}

/// The page's printed lines between its first two rules, and the 1-based row of the first of them. Blank lines
/// don't count as rows: a text layer may leave them out.
fn body(page: &[String]) -> Option<(usize, Vec<&String>)> {
    let printed: Vec<&String> = page.iter().filter(|l| !l.trim().is_empty()).collect();
    let start = printed.iter().position(|l| is_rule(l))?;
    let end = printed[start + 1..]
        .iter()
        .position(|l| is_rule(l))
        .map_or(printed.len(), |i| start + 1 + i);
    Some((start + 2, printed[start + 1..end].to_vec()))
}

fn is_rule(line: &str) -> bool {
    let line = line.trim();
    line.len() > 20 && line.chars().all(|c| c == '-')
}

fn booking(
    line: &str,
    page: usize,
    row: usize,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<ParsedLine, ParseError> {
    let unreadable = || ParseError::Unreadable(format!("page {page}: {}", line.trim()));
    let (rest, last) = split_last(line).ok_or_else(unreadable)?;
    let cents = amount(last)
        .filter(|_| last.starts_with(['+', '-']))
        .ok_or_else(unreadable)?;
    let (text, day) = split_last(rest).ok_or_else(unreadable)?;
    let value_date = day_in_period(day, from, to).ok_or_else(unreadable)?;
    Ok(ParsedLine {
        page,
        row,
        text: text.trim().to_string(),
        extra: Vec::new(),
        value_date,
        cents,
        counterparty: None,
        counterparty_iban: None,
        card: None,
        fx: None,
    })
}

/// Fills in what the text and the detail lines say: card, FX, counterparty and its IBAN.
fn details(line: &mut ParsedLine, from: NaiveDate, to: NaiveDate) {
    line.card = card(&line.text, from, to);
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

/// `E-COMM 24,00 USD K1 08.11. LUMEN AI SAN FRANCISCO 840` or `POS 58,30 AT K1 04.11. 17:40 TANKSTELLE ...`.
fn card(text: &str, from: NaiveDate, to: NaiveDate) -> Option<Card> {
    let mut words = text.split_whitespace();
    let kind = match words.next()? {
        "E-COMM" => CardKind::Online,
        "POS" => CardKind::Terminal,
        _ => return None,
    };
    let original_amount = amount(words.next()?)?;
    let currency = words.next()?;
    words.next().filter(|w| w.starts_with('K'))?;
    let date = day_in_period(words.next()?, from, to)?;
    let mut rest: Vec<&str> = words.collect();
    let time = rest
        .first()
        .filter(|w| is_clock_time(w))
        .map(|w| w.to_string());
    if time.is_some() {
        rest.remove(0);
    }
    if rest.last().is_some_and(|w| is_country_code(w)) {
        rest.pop();
    }
    let foreign = currency.len() == 3 && currency != "EUR";
    Some(Card {
        kind,
        date,
        time,
        merchant: rest.join(" "),
        original: foreign.then(|| (original_amount, currency.to_string())),
    })
}

fn is_clock_time(word: &str) -> bool {
    word.len() == 5 && word.as_bytes()[2] == b':'
}

/// The numeric country code card lines end with (040, 840).
fn is_country_code(word: &str) -> bool {
    word.len() == 3 && word.chars().all(|c| c.is_ascii_digit())
}

fn token_after<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.split_once(label)?.1.split_whitespace().next()
}

/// `SPESEN: 0,49 KURS: 1,0912 VOM: 06.11.2026`.
fn fx(text: &str) -> Option<Fx> {
    let fee = token_after(text, "SPESEN:")?;
    let rate = token_after(text, "KURS:")?;
    let day = token_after(text, "VOM:")?;
    Some(Fx {
        fee: amount(fee)?,
        rate: rate.to_string(),
        rate_date: date(day)?,
    })
}
