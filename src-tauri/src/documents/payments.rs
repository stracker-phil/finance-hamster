//! The bank lines that may have paid an invoice, and how a line shows as a choice.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

use super::add::currency_of;
use super::guess::Guess;
use super::store;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::money::{Money, format_money, format_signed};
use crate::review::{key_of, title_of as transaction_title};
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};

/// Days between an invoice and the bank line that paid it, either way.
const LINE_WINDOW_DAYS: i64 = 10;
const SUGGESTED_LINES: usize = 3;

/// A bank line the Add sheet offers as the payment.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct LineChoice {
    pub id: String,
    pub title: String,
    /// `Business · Tue, 10 Nov · USD 129.00`.
    pub detail: String,
    pub amount: String,
}

pub(super) fn days_apart(a: &str, b: NaiveDate) -> Option<i64> {
    date::parse(a).map(|a| (a - b).num_days().abs())
}

/// The bank lines that likely paid the invoice: same amount, or the vendor's token, within the line window.
pub(super) fn candidates(db: &Db, guess: &Guess) -> AppResult<Vec<TransactionRow>> {
    let linked: Vec<String> = db
        .read(store::links)?
        .into_iter()
        .map(|l| l.transaction_id)
        .collect();
    let token = guess.vendor.as_deref().map(key_of);
    let mut scored: Vec<(i64, TransactionRow)> = db
        .read(statements::all_transactions)?
        .into_iter()
        .filter(|t| t.cents < 0 && !linked.contains(&t.id))
        .filter_map(|t| {
            let distance = guess
                .invoice_date
                .and_then(|d| days_apart(&t.value_date, d));
            if distance.is_some_and(|n| n > LINE_WINDOW_DAYS) {
                return None;
            }
            let currency = currency_of(guess);
            let same_amount = guess.amount_cents.is_some_and(|a| {
                if currency == "EUR" {
                    t.cents.abs() == a
                } else {
                    t.original_cents == Some(a)
                        && t.original_currency.as_deref() == Some(currency.as_str())
                }
            });
            let text = format!("{} {}", t.text, t.extra).to_uppercase();
            let names_vendor = token.as_ref().is_some_and(|k| text.contains(k.as_str()));
            if !same_amount && !(names_vendor && distance.is_some()) {
                return None;
            }
            let score = i64::from(same_amount) * 100 + i64::from(names_vendor) * 50
                - distance.unwrap_or(LINE_WINDOW_DAYS);
            Some((score, t))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.value_date.cmp(&a.1.value_date)));
    Ok(scored
        .into_iter()
        .take(SUGGESTED_LINES)
        .map(|(_, t)| t)
        .collect())
}

pub fn line_choice(
    t: &TransactionRow,
    account_names: &HashMap<String, String>,
    format: NumberFormat,
    order: DateOrder,
) -> LineChoice {
    let mut detail = vec![
        account_names
            .get(&t.account_id)
            .cloned()
            .unwrap_or_default(),
    ];
    if let Some(day) = date::parse(&t.value_date) {
        detail.push(date::weekday_short(day, order));
    }
    if let (Some(cents), Some(code)) = (t.original_cents, &t.original_currency) {
        detail.push(format_money(
            &Money {
                cents,
                currency: code.clone(),
            },
            format,
        ));
    }
    LineChoice {
        id: t.id.clone(),
        title: transaction_title(t),
        detail: detail.join(" · "),
        amount: format_signed(&Money::eur(t.cents), format),
    }
}
