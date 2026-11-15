//! The demo payslips: January to June of the demo year, typed without PDFs, with home office days counted up
//! the way payslips print them (year to date).

use crate::db::{Change, Db, Op, Source};
use crate::error::AppResult;
use crate::payslips::{PAYSLIPS, PayslipRow};

/// Home office days so far at the end of each month.
const DAYS_SO_FAR: [i64; 6] = [8, 16, 25, 33, 41, 50];

/// Adds the demo payslips to `year`; returns how many ops it wrote.
pub fn seed(db: &Db, year: i32, now: i64) -> AppResult<usize> {
    let changes = DAYS_SO_FAR
        .iter()
        .zip(1..)
        .map(|(days, month)| {
            Change::insert(
                PAYSLIPS,
                &PayslipRow {
                    id: format!("demo-payslip-{month}"),
                    year,
                    month,
                    file: None,
                    file_hash: None,
                    gross: Some(345_000),
                    social: Some(62_010),
                    tax: Some(45_530),
                    net: Some(237_460),
                    payout: Some(237_460),
                    home_office_pay: None,
                    telework_days: Some(*days),
                    note: String::new(),
                    created_at: now,
                },
            )
        })
        .collect::<AppResult<Vec<_>>>()?;
    db.write(Op::new("Add the demo payslips", changes), Source::Seed)?;
    Ok(1)
}
