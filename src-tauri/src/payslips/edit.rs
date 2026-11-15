//! Adding, changing and deleting payslips; each is one undoable op. A closed year refuses them through the
//! frozen-history check.

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{PAYSLIPS, PayslipRow, payslip, payslips};
use crate::date::month_name;
use crate::db::{Change, Db, Op, Source, Written};
use crate::documents::files;
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;

/// The amounts a payslip records.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum PayslipAmount {
    Gross,
    Social,
    Tax,
    Net,
    Payout,
    HomeOfficePay,
}

impl PayslipAmount {
    fn slot(self, row: &mut PayslipRow) -> &mut Option<i64> {
        match self {
            Self::Gross => &mut row.gross,
            Self::Social => &mut row.social,
            Self::Tax => &mut row.tax,
            Self::Net => &mut row.net,
            Self::Payout => &mut row.payout,
            Self::HomeOfficePay => &mut row.home_office_pay,
        }
    }

    fn field(self) -> &'static str {
        match self {
            Self::Gross => "payslip.gross",
            Self::Social => "payslip.social",
            Self::Tax => "payslip.tax",
            Self::Net => "payslip.net",
            Self::Payout => "payslip.payout",
            Self::HomeOfficePay => "payslip.homeOfficePay",
        }
    }
}

/// One change to a payslip; blank text clears a value.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "field", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum PayslipEdit {
    Amount { name: PayslipAmount, value: String },
    TeleworkDays { value: String },
    Note { text: String },
    Month { month: u32 },
}

fn month_title(year: i32, month: u32) -> String {
    format!("{} {year}", month_name(month))
}

fn not_found() -> AppError {
    AppError::NotFound(crate::tr!(
        "No such payslip",
        "Diesen Lohnzettel gibt es nicht"
    ))
}

fn taken(year: i32, month: u32) -> AppError {
    AppError::InvalidInput(crate::tr!(
        "There already is a payslip for {}",
        "Für {} gibt es schon einen Lohnzettel",
        month_title(year, month)
    ))
}

fn check_month(month: u32) -> AppResult<()> {
    if (1..=12).contains(&month) {
        Ok(())
    } else {
        Err(AppError::InvalidInput(crate::tr!(
            "{month} isn't a month",
            "{month} ist kein Monat"
        )))
    }
}

fn month_taken(conn: &Connection, year: i32, month: u32) -> AppResult<bool> {
    Ok(payslips(conn, year)?.iter().any(|p| p.month == month))
}

/// Copies a payslip PDF to `docs/<year>/payslips/<yyyy-mm>.pdf`; the path and the file's hash.
fn copy_pdf(config_dir: &Path, year: i32, month: u32, bytes: &[u8]) -> AppResult<(String, String)> {
    let hash = crate::statements::file_hash(bytes);
    let base = format!("{}/{year}/payslips/{year}-{month:02}.pdf", files::FOLDER);
    Ok((files::copy_to(config_dir, &base, bytes, &hash)?, hash))
}

/// Adds the payslip of `year`/`month`, with its PDF when given; the change and the new payslip's id.
pub fn add(
    db: &Db,
    config_dir: &Path,
    year: i32,
    month: u32,
    pdf: Option<&[u8]>,
    now: i64,
) -> AppResult<(Written, String)> {
    check_month(month)?;
    if db.read(|c| month_taken(c, year, month))? {
        return Err(taken(year, month));
    }
    let (file, file_hash) = match pdf {
        Some(bytes) => {
            let (file, hash) = copy_pdf(config_dir, year, month, bytes)?;
            (Some(file), Some(hash))
        }
        None => (None, None),
    };
    let row = PayslipRow {
        id: uuid::Uuid::new_v4().to_string(),
        year,
        month,
        file,
        file_hash,
        gross: None,
        social: None,
        tax: None,
        net: None,
        payout: None,
        home_office_pay: None,
        telework_days: None,
        note: String::new(),
        created_at: now,
    };
    let written = db.write(
        Op::new(
            crate::tr!(
                "Add Payslip {}",
                "Lohnzettel {} hinzufügen",
                month_title(year, month)
            ),
            vec![Change::insert(PAYSLIPS, &row)?],
        ),
        Source::Ui,
    )?;
    Ok((written, row.id))
}

fn stored(db: &Db, id: &str) -> AppResult<PayslipRow> {
    db.read(|c| payslip(c, id))?.ok_or_else(not_found)
}

/// Attaches a PDF to a payslip, replacing any earlier one; the old file stays.
pub fn attach(db: &Db, config_dir: &Path, id: &str, bytes: &[u8]) -> AppResult<Written> {
    let before = stored(db, id)?;
    let (file, hash) = copy_pdf(config_dir, before.year, before.month, bytes)?;
    let after = PayslipRow {
        file: Some(file),
        file_hash: Some(hash),
        ..before.clone()
    };
    db.write(
        Op::new(
            crate::tr!("Attach Payslip PDF", "Lohnzettel-PDF anhängen"),
            vec![Change::update(PAYSLIPS, &before, &after)?],
        ),
        Source::Ui,
    )
}

fn amount(name: PayslipAmount, text: &str) -> AppResult<Option<i64>> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    parse_cents(text)
        .map(i64::abs)
        .map(Some)
        .ok_or_else(|| AppError::InvalidField {
            field: name.field().into(),
            message: crate::tr!("This isn't an amount", "Das ist kein Betrag"),
        })
}

fn days(text: &str) -> AppResult<Option<i64>> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    text.trim()
        .parse::<i64>()
        .ok()
        .filter(|n| (0..=366).contains(n))
        .map(Some)
        .ok_or_else(|| AppError::InvalidField {
            field: "payslip.teleworkDays".into(),
            message: crate::tr!("A whole number of days", "Eine ganze Zahl von Tagen"),
        })
}

/// Applies one change; None when the payslip already says that.
pub fn edit(db: &Db, id: &str, change: &PayslipEdit) -> AppResult<Option<Written>> {
    let before = stored(db, id)?;
    let mut after = before.clone();
    let title = month_title(before.year, before.month);
    let label = match change {
        PayslipEdit::Amount { name, value } => {
            *name.slot(&mut after) = amount(*name, value)?;
            crate::tr!(
                "Change {} of Payslip {title}",
                "{} im Lohnzettel {title} ändern",
                super::view::label(*name)
            )
        }
        PayslipEdit::TeleworkDays { value } => {
            after.telework_days = days(value)?;
            crate::tr!(
                "Change Home Office Days of Payslip {title}",
                "Homeoffice-Tage im Lohnzettel {title} ändern"
            )
        }
        PayslipEdit::Note { text } => {
            after.note = text.trim().to_string();
            crate::tr!(
                "Change Note of Payslip {title}",
                "Notiz im Lohnzettel {title} ändern"
            )
        }
        PayslipEdit::Month { month } => {
            check_month(*month)?;
            if *month != before.month && db.read(|c| month_taken(c, before.year, *month))? {
                return Err(taken(before.year, *month));
            }
            after.month = *month;
            crate::tr!(
                "Move Payslip {title} to {}",
                "Lohnzettel {title} auf {} verschieben",
                month_name(*month)
            )
        }
    };
    if after == before {
        return Ok(None);
    }
    db.write(
        Op::new(label, vec![Change::update(PAYSLIPS, &before, &after)?]),
        Source::Ui,
    )
    .map(Some)
}

/// Deletes a payslip; its PDF stays in the data folder.
pub fn delete(db: &Db, id: &str) -> AppResult<Written> {
    let row = stored(db, id)?;
    db.write(
        Op::new(
            crate::tr!(
                "Delete Payslip {}",
                "Lohnzettel {} löschen",
                month_title(row.year, row.month)
            ),
            vec![Change::delete(PAYSLIPS, &row)?],
        ),
        Source::Ui,
    )
}
