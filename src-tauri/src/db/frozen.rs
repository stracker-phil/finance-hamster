//! Frozen history: no write changes what a closed tax year counts. Checked for every change inside the write's
//! transaction, so each change sees the rows as the ones before it in the same op left them; closing and
//! reopening a year write only the tables this check doesn't look at.

use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::Value;

use super::{Change, Row};
use crate::error::{AppError, AppResult};

/// Refuses `change` when a row it removes or writes counts in a closed tax year.
pub(super) fn check(tx: &Transaction, change: &Change) -> AppResult<()> {
    let rows: Vec<&Row> = match change {
        Change::Insert { row, .. } | Change::Delete { row, .. } => vec![row],
        Change::Update { before, after, .. } => vec![before, after],
    };
    for row in rows {
        for year in years_of(tx, change.table(), row)? {
            if is_closed(tx, year)? {
                return Err(AppError::InvalidInput(format!(
                    "Tax year {year} is closed; reopen it to change what it counts"
                )));
            }
        }
    }
    Ok(())
}

/// The tax years a row counts in; none for rows that count nowhere, such as an excluded document.
fn years_of(tx: &Transaction, table: &str, row: &Row) -> AppResult<Vec<i32>> {
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    Ok(match table {
        "documents" if text("state") != Some("excluded") => {
            year_of(text("pay_date")).into_iter().collect()
        }
        "document_links" => {
            let mut years: Vec<i32> = line_year(tx, text("transaction_id"))?.into_iter().collect();
            years.extend(document_year(tx, text("document_id"))?);
            years
        }
        "decisions" if counts_as_anv(tx, text("kind"), text("subscription_id"))? => {
            line_year(tx, text("id"))?.into_iter().collect()
        }
        "entries" | "answers" => row
            .get("year")
            .and_then(Value::as_i64)
            .and_then(|y| i32::try_from(y).ok())
            .into_iter()
            .collect(),
        _ => Vec::new(),
    })
}

/// An ANV decision, or a subscription decision whose subscription has a Werbungskosten item; a subscription that
/// is gone can't say, so its decisions count.
fn counts_as_anv(
    tx: &Transaction,
    kind: Option<&str>,
    subscription: Option<&str>,
) -> AppResult<bool> {
    Ok(match (kind, subscription) {
        (Some("anv"), _) => true,
        (Some("subscription"), Some(id)) => tx
            .query_row(
                "SELECT item_key IS NOT NULL FROM subscriptions WHERE id = ?",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(true),
        _ => false,
    })
}

fn year_of(date: Option<&str>) -> Option<i32> {
    date?.get(..4)?.parse().ok()
}

fn line_year(tx: &Transaction, id: Option<&str>) -> AppResult<Option<i32>> {
    let Some(id) = id else { return Ok(None) };
    let date: Option<String> = tx
        .query_row(
            "SELECT value_date FROM transactions WHERE id = ?",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(year_of(date.as_deref()))
}

/// A linked document counts in its payment year unless it is excluded.
fn document_year(tx: &Transaction, id: Option<&str>) -> AppResult<Option<i32>> {
    let Some(id) = id else { return Ok(None) };
    let found: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT state, pay_date FROM documents WHERE id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(found
        .filter(|(state, _)| state != "excluded")
        .and_then(|(_, pay_date)| year_of(pay_date.as_deref())))
}

fn is_closed(tx: &Transaction, year: i32) -> AppResult<bool> {
    let state: Option<String> = tx
        .query_row(
            "SELECT state FROM tax_years WHERE year = ?",
            params![year],
            |r| r.get(0),
        )
        .optional()?;
    Ok(state.as_deref() == Some("closed"))
}
