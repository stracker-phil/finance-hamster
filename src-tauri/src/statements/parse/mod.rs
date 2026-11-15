//! Statement text to `ParsedStatement`, one parser per bank layout (ADR 0018: the text is untrusted and only
//! ever read as text). A new layout adds an entry to `PARSERS`.

mod booking;
pub mod george;
pub mod model;

#[cfg(test)]
mod george_tests;
#[cfg(test)]
mod tests;

use model::ParsedStatement;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("No known statement layout")]
    UnknownLayout,
    #[error("The statement has no {0}")]
    Missing(String),
    #[error("Can't read {0}")]
    Unreadable(String),
}

/// A bank layout: how to recognize and read it.
pub struct Parser {
    pub recognizes: fn(&[Vec<String>]) -> bool,
    pub parse: fn(&[Vec<String>]) -> Result<ParsedStatement, ParseError>,
}

pub const PARSERS: &[Parser] = &[Parser {
    recognizes: george::recognizes,
    parse: george::parse,
}];

/// Reads the pages with the first parser that recognizes them.
pub fn parse(pages: &[Vec<String>]) -> Result<ParsedStatement, ParseError> {
    let parser = PARSERS
        .iter()
        .find(|p| (p.recognizes)(pages))
        .ok_or(ParseError::UnknownLayout)?;
    (parser.parse)(pages)
}

/// A printed amount in cents: `1.234,56`, `+2.850,00`, `-980,00`, `980,00-`.
pub fn amount(text: &str) -> Option<i64> {
    let (negative, digits) = match (text.strip_prefix('-'), text.strip_suffix('-')) {
        (Some(rest), None) | (None, Some(rest)) => (true, rest),
        _ => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once(',')?;
    let groups: Vec<&str> = whole.split('.').collect();
    // Dots only group thousands; a misread like `12.34,56` is no amount.
    let grouped = (1..=3).contains(&groups[0].len()) && groups[1..].iter().all(|g| g.len() == 3);
    let whole = groups.concat();
    let valid = (groups.len() == 1 || grouped)
        && !whole.is_empty()
        && whole.chars().all(|c| c.is_ascii_digit())
        && fraction.len() == 2
        && fraction.chars().all(|c| c.is_ascii_digit());
    if !valid {
        return None;
    }
    let cents = whole.parse::<i64>().ok()?.checked_mul(100)? + fraction.parse::<i64>().ok()?;
    Some(if negative { -cents } else { cents })
}
