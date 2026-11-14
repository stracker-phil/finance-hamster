//! The Sparkasse/Erste "Kontoauszug" layout: a fixed-width text layer with the header on every page, one line
//! per booking (text, value date `dd.mm.`, signed amount at the right edge) and indented detail lines below.

use chrono::NaiveDate;

use super::booking::{date, day_in_period, details, split_last};
use super::model::{ParsedLine, ParsedStatement};
use super::{ParseError, amount};
use crate::accounts::normalize_iban;

pub const LAYOUT: &str = "sparkasse-erste-1";

pub fn recognizes(pages: &[Vec<String>]) -> bool {
    pages.first().is_some_and(|page| {
        page.iter().any(|l| l.contains("Kontoauszug Nr."))
            && page.iter().any(|l| l.trim_start().starts_with("IBAN:"))
    })
}

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
