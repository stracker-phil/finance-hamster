//! The Sparkasse statement as Digitales Banking (George) prints it: German and English labels, the balances
//! and sums in the first page's header, one line per booking (text, value date `dd.mm.yyyy`, amount with a
//! trailing minus for debits) and detail lines below. It prints no period: it runs from the first value date
//! to the last.

use super::booking::{date, details, split_last};
use super::model::{ParsedLine, ParsedStatement};
use super::{ParseError, amount};
use crate::accounts::normalize_iban;

pub const LAYOUT: &str = "sparkasse-george-1";

/// The column titles above the bookings on every page.
const COLUMNS: &str = "Buchungstext/Booking";

pub fn recognizes(pages: &[Vec<String>]) -> bool {
    pages.first().is_some_and(|page| {
        page.iter().any(|l| l.contains(COLUMNS))
            && page.iter().any(|l| l.trim_start().starts_with("IBAN:"))
    })
}

/// The first page's lines above the column titles.
fn header(pages: &[Vec<String>]) -> &[String] {
    let page = pages.first().map_or(&[][..], Vec::as_slice);
    let end = page
        .iter()
        .position(|l| l.contains(COLUMNS))
        .unwrap_or(page.len());
    &page[..end]
}

/// The amount at the end of the header line that has `label`, e.g. `Alter Kontostand   2.798,40`.
fn sum(header: &[String], label: &str) -> Result<i64, ParseError> {
    header
        .iter()
        .filter(|l| l.contains(label))
        .find_map(|l| amount(split_last(l)?.1))
        .ok_or_else(|| ParseError::Missing(label.into()))
}

/// `Nr.002/2026` or `Nr. 002/2026`.
fn number(header: &[String]) -> Result<(u32, i32), ParseError> {
    header
        .iter()
        .find_map(|l| {
            let (n, y) = l
                .split_once("Nr.")?
                .1
                .split_whitespace()
                .next()?
                .split_once('/')?;
            Some((n.parse().ok()?, y.parse().ok()?))
        })
        .ok_or_else(|| ParseError::Missing("Kontoauszug Nr.".into()))
}

fn iban(header: &[String]) -> Result<String, ParseError> {
    let text = header
        .iter()
        .find_map(|l| l.trim_start().strip_prefix("IBAN:"))
        .ok_or_else(|| ParseError::Missing("IBAN".into()))?;
    let text = text.split_once("BIC:").map_or(text, |(iban, _)| iban);
    normalize_iban(text).ok_or(ParseError::Unreadable(format!("IBAN {}", text.trim())))
}

pub fn parse(pages: &[Vec<String>]) -> Result<ParsedStatement, ParseError> {
    let header = header(pages);
    let (number, year) = number(header)?;
    let iban = iban(header)?;
    let old_balance = sum(header, "Alter Kontostand")?;
    let credits = sum(header, "Gutschriften")?.abs();
    let debits = -sum(header, "Belastungen")?.abs();
    let new_balance = sum(header, "Neuer Kontostand")?;
    let mut bookings = lines(pages, &iban);
    let from = bookings.iter().map(|l| l.value_date).min();
    let to = bookings.iter().map(|l| l.value_date).max();
    let (Some(from), Some(to)) = (from, to) else {
        return Err(ParseError::Missing("booking".into()));
    };
    for line in &mut bookings {
        details(line, from, to);
    }
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
        sums_page: 1,
        lines: bookings,
    })
}

/// The bookings: a line ending in a value date and an amount starts one, the lines below add to it, across
/// pages. A `*** Abschlussbuchung ... ***` heading ends the booking before it; its notes belong to none.
fn lines(pages: &[Vec<String>], iban: &str) -> Vec<ParsedLine> {
    let mut out: Vec<ParsedLine> = Vec::new();
    let mut open = false;
    for (p, page) in pages.iter().enumerate() {
        let printed: Vec<&String> = page.iter().filter(|l| !l.trim().is_empty()).collect();
        let Some(start) = printed.iter().position(|l| l.contains(COLUMNS)) else {
            continue;
        };
        for (i, line) in printed.iter().enumerate().skip(start + 1) {
            let text = single_spaced(line);
            if text.starts_with(iban) || text.starts_with("Neuer Kontostand") {
                break;
            }
            if text.starts_with("***") {
                open = false;
            } else if let Some(booking) = booking(&text, p + 1, i + 1) {
                out.push(booking);
                open = true;
            } else if open && let Some(last) = out.last_mut() {
                last.extra.push(text);
            }
        }
    }
    out
}

/// The words of a line one space apart: how wide a gap reads back depends on the fonts on the page, so the
/// same booking reads the same on every page.
fn single_spaced(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `Polizze Nr. 2044922090 04.05.2026 7,44-`.
fn booking(text: &str, page: usize, row: usize) -> Option<ParsedLine> {
    let (rest, last) = split_last(text)?;
    let cents = amount(last).filter(|_| !last.starts_with(['+', '-']))?;
    let (text, day) = split_last(rest)?;
    Some(ParsedLine {
        page,
        row,
        text: text.to_string(),
        extra: Vec::new(),
        value_date: date(day)?,
        cents,
        counterparty: None,
        counterparty_iban: None,
        card: None,
        fx: None,
    })
}
