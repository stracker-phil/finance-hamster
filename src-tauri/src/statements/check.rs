//! The checks a statement must pass before import (plan: block on failure) and the chain to the previous one.

use super::parse::model::ParsedStatement;
use crate::money::{Money, format_signed};
use crate::settings::NumberFormat;

fn euros(cents: i64) -> String {
    format_signed(&Money::eur(cents), NumberFormat::default())
}

/// Why a statement can't be imported, naming the page and the difference.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    #[error(
        "Page {page}: the old balance plus the sums isn't the new balance; it is off by {difference}"
    )]
    Balance { page: usize, difference: String },
    #[error("Page {page}: the credits add up to {lines}, the statement prints {printed}")]
    Credits {
        page: usize,
        lines: String,
        printed: String,
    },
    #[error("Page {page}: the debits add up to {lines}, the statement prints {printed}")]
    Debits {
        page: usize,
        lines: String,
        printed: String,
    },
}

/// Old balance + credits + debits = new balance, and the lines sum to the printed credits and debits.
pub fn check(s: &ParsedStatement) -> Result<(), CheckError> {
    let page = s.sums_page;
    let difference = s.old_balance + s.credits + s.debits - s.new_balance;
    if difference != 0 {
        return Err(CheckError::Balance {
            page,
            difference: euros(difference),
        });
    }
    let credits: i64 = s.lines.iter().map(|l| l.cents).filter(|c| *c > 0).sum();
    if credits != s.credits {
        return Err(CheckError::Credits {
            page,
            lines: euros(credits),
            printed: euros(s.credits),
        });
    }
    let debits: i64 = s.lines.iter().map(|l| l.cents).filter(|c| *c < 0).sum();
    if debits != s.debits {
        return Err(CheckError::Debits {
            page,
            lines: euros(debits),
            printed: euros(s.debits),
        });
    }
    Ok(())
}

/// The previous statement of the same account, as far as the chain needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Previous {
    pub number: u32,
    pub year: i32,
    pub new_balance: i64,
}

/// How a statement continues the previous one; a gap still imports, and the account shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chain {
    /// The account's first statement.
    First,
    Continues,
    /// Numbers are missing in between (the next year's numbering restarts at 1).
    MissingNumbers {
        expected: u32,
    },
    /// The old balance isn't the previous new balance.
    BalanceGap {
        previous: i64,
    },
}

pub fn chain(previous: Option<&Previous>, s: &ParsedStatement) -> Chain {
    let Some(p) = previous else {
        return Chain::First;
    };
    let next_number = s.year == p.year && s.number == p.number + 1;
    let new_year = s.year == p.year + 1 && s.number == 1;
    if !(next_number || new_year) {
        return Chain::MissingNumbers {
            expected: if s.year == p.year { p.number + 1 } else { 1 },
        };
    }
    if s.old_balance != p.new_balance {
        return Chain::BalanceGap {
            previous: p.new_balance,
        };
    }
    Chain::Continues
}
