//! Monthly payslips (ADR 0055): what the salary line on the bank statement doesn't show, typed from the PDF.
//! One per year and month; the year's home office days come from the latest one, since payslips print them as
//! a year-to-date sum.

pub mod commands;
pub mod edit;
pub mod guess;
pub mod view;

#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

pub const PAYSLIPS: &str = "payslips";

/// The year's question the payslips answer.
pub const TELEWORK_QUESTION: &str = "telearbeitstage";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PayslipRow {
    pub id: String,
    pub year: i32,
    pub month: u32,
    /// Under the data folder; None for a payslip typed without its PDF.
    pub file: Option<String>,
    pub file_hash: Option<String>,
    pub gross: Option<i64>,
    pub social: Option<i64>,
    pub tax: Option<i64>,
    pub net: Option<i64>,
    pub payout: Option<i64>,
    /// The employer's tax-free Homeoffice-Pauschale.
    pub home_office_pay: Option<i64>,
    /// Year to date, as the payslip's annual sums print it.
    pub telework_days: Option<i64>,
    pub note: String,
    pub created_at: i64,
}

const COLUMNS: &str = "id, year, month, file, file_hash, gross, social, tax, net, payout, home_office_pay, \
                       telework_days, note, created_at";

fn read_row(r: &rusqlite::Row) -> rusqlite::Result<PayslipRow> {
    Ok(PayslipRow {
        id: r.get(0)?,
        year: r.get(1)?,
        month: r.get(2)?,
        file: r.get(3)?,
        file_hash: r.get(4)?,
        gross: r.get(5)?,
        social: r.get(6)?,
        tax: r.get(7)?,
        net: r.get(8)?,
        payout: r.get(9)?,
        home_office_pay: r.get(10)?,
        telework_days: r.get(11)?,
        note: r.get(12)?,
        created_at: r.get(13)?,
    })
}

/// The year's payslips by month.
pub fn payslips(conn: &Connection, year: i32) -> AppResult<Vec<PayslipRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM {PAYSLIPS} WHERE year = ? ORDER BY month"
    ))?;
    let rows = query.query_map([year], read_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn payslip(conn: &Connection, id: &str) -> AppResult<Option<PayslipRow>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM {PAYSLIPS} WHERE id = ?"),
            [id],
            read_row,
        )
        .optional()?)
}

/// The home office days of `year` from its latest payslip that prints them, with that payslip's month.
pub fn telework_days(conn: &Connection, year: i32) -> AppResult<Option<(u32, i64)>> {
    Ok(payslips(conn, year)?
        .into_iter()
        .rev()
        .find_map(|p| Some((p.month, p.telework_days?))))
}
