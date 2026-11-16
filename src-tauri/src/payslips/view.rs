//! The payslips as the window shows them: the year's twelve months with their sums, and one payslip's form.

use serde::Serialize;
use ts_rs::TS;

use super::edit::PayslipAmount;
use super::{PAYSLIPS, PayslipRow, payslip, payslips, telework_days};
use crate::date::month_name;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::money::format_plain;
use crate::settings::NumberFormat;
use crate::taxyears::{self, YearState};
use crate::year::facts::eur;

/// One month of the year's payslip panel.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipMonth {
    pub month: u32,
    pub name: String,
    /// None while the month has no payslip.
    pub id: Option<String>,
    pub payout: Option<String>,
    pub has_pdf: bool,
}

/// A labelled sum of the year.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipTotal {
    pub label: String,
    pub value: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipsYear {
    pub year: i32,
    pub open: bool,
    /// January to December, always twelve.
    pub months: Vec<PayslipMonth>,
    pub totals: Vec<PayslipTotal>,
    /// `122 days · as of July`; None while no payslip prints them.
    pub telework: Option<String>,
    /// `10 of 12 payslips`.
    pub count: String,
}

/// One amount field of the payslip form.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipField {
    pub name: PayslipAmount,
    pub label: String,
    /// Where on the payslip the value is printed.
    pub hint: String,
    /// As the field shows it; empty while not typed.
    pub value: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PayslipView {
    pub id: String,
    pub year: i32,
    pub month: u32,
    /// `Payslip July 2026`.
    pub title: String,
    pub open: bool,
    pub has_pdf: bool,
    pub amounts: Vec<PayslipField>,
    pub telework_days: String,
    pub note: String,
    /// January to December, for the month picker.
    pub month_names: Vec<String>,
}

const AMOUNTS: [PayslipAmount; 6] = [
    PayslipAmount::Gross,
    PayslipAmount::Social,
    PayslipAmount::Tax,
    PayslipAmount::Net,
    PayslipAmount::Payout,
    PayslipAmount::HomeOfficePay,
];

fn value(row: &PayslipRow, name: PayslipAmount) -> Option<i64> {
    match name {
        PayslipAmount::Gross => row.gross,
        PayslipAmount::Social => row.social,
        PayslipAmount::Tax => row.tax,
        PayslipAmount::Net => row.net,
        PayslipAmount::Payout => row.payout,
        PayslipAmount::HomeOfficePay => row.home_office_pay,
    }
}

pub(super) fn label(name: PayslipAmount) -> String {
    match name {
        PayslipAmount::Gross => crate::tr!("Gross", "Brutto"),
        PayslipAmount::Social => crate::tr!("Social insurance", "Sozialversicherung"),
        PayslipAmount::Tax => crate::tr!("Income tax", "Lohnsteuer"),
        PayslipAmount::Net => crate::tr!("Net", "Netto"),
        PayslipAmount::Payout => crate::tr!("Payout", "Auszahlung"),
        PayslipAmount::HomeOfficePay => crate::tr!(
            "Home office allowance from the employer",
            "Homeoffice-Pauschale vom Arbeitgeber"
        ),
    }
}

/// Where the value is printed; empty where the German label already names it.
fn hint(name: PayslipAmount) -> String {
    match name {
        PayslipAmount::Gross => crate::tr!("“Brutto”", ""),
        PayslipAmount::Social => crate::tr!("“SV Beitrag lfd”", "„SV Beitrag lfd“"),
        PayslipAmount::Tax => crate::tr!("“LSt Beitrag lfd”", "„LSt Beitrag lfd“"),
        PayslipAmount::Net => crate::tr!("“Netto”", ""),
        PayslipAmount::Payout => crate::tr!(
            "“Auszahlung”, the amount on your bank statement",
            "Der Betrag auf dem Kontoauszug"
        ),
        PayslipAmount::HomeOfficePay => crate::tr!(
            "Tax-free, only when the employer pays it",
            "Steuerfrei, nur wenn der Arbeitgeber sie zahlt"
        ),
    }
}

fn is_open(db: &Db, year: i32) -> AppResult<bool> {
    Ok(db
        .read(|c| taxyears::row(c, year))?
        .is_some_and(|r| r.state == YearState::Open))
}

pub fn year_view(db: &Db, year: i32, format: NumberFormat) -> AppResult<PayslipsYear> {
    let rows = db.read(|c| payslips(c, year))?;
    let months = (1..=12)
        .map(|month| {
            let row = rows.iter().find(|p| p.month == month);
            PayslipMonth {
                month,
                name: month_name(month).to_string(),
                id: row.map(|p| p.id.clone()),
                payout: row.and_then(|p| p.payout).map(|c| eur(c, format)),
                has_pdf: row.is_some_and(|p| p.file.is_some()),
            }
        })
        .collect();
    let totals = AMOUNTS
        .iter()
        .filter(|name| **name != PayslipAmount::Payout)
        .map(|name| PayslipTotal {
            label: label(*name),
            value: eur(rows.iter().filter_map(|p| value(p, *name)).sum(), format),
        })
        .collect();
    let telework = db.read(|c| telework_days(c, year))?.map(|(month, days)| {
        crate::tr!(
            "{days} days · as of {}",
            "{days} Tage · Stand {}",
            month_name(month)
        )
    });
    let filled = rows.len();
    Ok(PayslipsYear {
        year,
        open: is_open(db, year)?,
        months,
        totals,
        telework,
        count: crate::tr!("{filled} of 12 payslips", "{filled} von 12 Lohnzetteln"),
    })
}

/// Every year with a payslip or an open tax year, newest first, for Dokumente → Lohnzettel.
pub fn list(db: &Db, format: NumberFormat) -> AppResult<Vec<PayslipsYear>> {
    let mut years: Vec<i32> = db.read(|c| {
        let mut query = c.prepare(&format!("SELECT DISTINCT year FROM {PAYSLIPS}"))?;
        let rows = query.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<Vec<i32>, _>>()?)
    })?;
    years.extend(
        db.read(taxyears::rows)?
            .into_iter()
            .filter(|r| r.state == YearState::Open)
            .map(|r| r.year),
    );
    years.sort_unstable_by(|a, b| b.cmp(a));
    years.dedup();
    years
        .into_iter()
        .map(|y| year_view(db, y, format))
        .collect()
}

pub fn payslip_view(db: &Db, id: &str, format: NumberFormat) -> AppResult<PayslipView> {
    let row = db.read(|c| payslip(c, id))?.ok_or_else(|| {
        AppError::NotFound(crate::tr!(
            "No such payslip",
            "Diesen Lohnzettel gibt es nicht"
        ))
    })?;
    let amounts = AMOUNTS
        .iter()
        .map(|name| PayslipField {
            name: *name,
            label: label(*name),
            hint: hint(*name),
            value: value(&row, *name)
                .map(|c| format_plain(c, format))
                .unwrap_or_default(),
        })
        .collect();
    Ok(PayslipView {
        id: row.id.clone(),
        year: row.year,
        month: row.month,
        title: crate::tr!(
            "Payslip {} {}",
            "Lohnzettel {} {}",
            month_name(row.month),
            row.year
        ),
        open: is_open(db, row.year)?,
        has_pdf: row.file.is_some(),
        amounts,
        telework_days: row.telework_days.map(|d| d.to_string()).unwrap_or_default(),
        note: row.note,
        month_names: (1..=12).map(|m| month_name(m).to_string()).collect(),
    })
}
