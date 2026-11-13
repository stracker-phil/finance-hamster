//! What a statement says, as parsed from its text: the header, the printed sums and every line.

use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedStatement {
    /// The layout id of the parser that read it, stored with the statement so its lines are never re-read.
    pub layout: &'static str,
    /// Upper case, without spaces.
    pub iban: String,
    pub number: u32,
    /// The year the number counts in: the year of the statement's end.
    pub year: i32,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub old_balance: i64,
    /// Printed sum of the credits (positive).
    pub credits: i64,
    /// Printed sum of the debits (negative).
    pub debits: i64,
    pub new_balance: i64,
    /// The 1-based page the printed sums are on.
    pub sums_page: usize,
    pub lines: Vec<ParsedLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedLine {
    /// 1-based page and the line's position on it, for "line 68" references.
    pub page: usize,
    pub row: usize,
    /// The booking text as printed on the amount's line.
    pub text: String,
    /// The indented lines below it, as printed.
    pub extra: Vec<String>,
    /// The payment date.
    pub value_date: NaiveDate,
    /// Signed: credits positive, debits negative.
    pub cents: i64,
    pub counterparty: Option<String>,
    pub counterparty_iban: Option<String>,
    pub card: Option<Card>,
    pub fx: Option<Fx>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    /// An online payment (`E-COMM`).
    Online,
    /// At a terminal (`POS`).
    Terminal,
}

impl CardKind {
    /// How it is stored.
    pub fn as_str(self) -> &'static str {
        match self {
            CardKind::Online => "online",
            CardKind::Terminal => "terminal",
        }
    }
}

/// A card payment's details from its booking text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub kind: CardKind,
    /// The day the card was used, kept apart from the value date.
    pub date: NaiveDate,
    /// `17:40` at a terminal.
    pub time: Option<String>,
    pub merchant: String,
    /// The amount in a foreign currency, with its code; None for euro payments.
    pub original: Option<(i64, String)>,
}

/// A foreign-currency payment's fee and rate (`SPESEN: 0,49 KURS: 1,0912 VOM: 06.11.2026`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fx {
    pub fee: i64,
    /// As printed, e.g. `1,0912`.
    pub rate: String,
    pub rate_date: NaiveDate,
}
