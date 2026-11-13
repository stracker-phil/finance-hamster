//! The demo documents: invented invoices added through the real pipeline, one linked to its bank line and
//! dated, one dated by its link with a work share, one still undated in the inbox.

use std::path::Path;

use chrono::NaiveDate;

use crate::db::{Db, Source};
use crate::documents::{add, files};
use crate::error::AppResult;
use crate::platform::pdf::write::{self, Font, Run};
use crate::statements::store as statements;

struct DemoInvoice {
    file: &'static str,
    lines: &'static [&'static str],
    vendor: &'static str,
    item: Option<&'static str>,
    /// The bank line it is linked to: merchant token and value date.
    paid_by: Option<(&'static str, &'static str)>,
}

const INVOICES: [DemoInvoice; 3] = [
    DemoInvoice {
        file: "seitenweise-rechnung-2026-1014.pdf",
        lines: &[
            "Buchhandlung Seitenweise",
            "Rechnungsnummer: SW-2026-0815",
            "Datum: 14.10.2026",
            "1 x Fachbuch Datenbanken 46,90",
            "Gesamtbetrag EUR 46,90",
        ],
        vendor: "Buchhandlung Seitenweise",
        item: Some("literature"),
        paid_by: Some(("SEITENWEISE", "2026-10-14")),
    },
    DemoInvoice {
        file: "lumen-invoice-LUM-2026-1110.pdf",
        lines: &[
            "Lumen AI",
            "Invoice No: LUM-2026-1110",
            "Date: 2026-11-08",
            "Lumen Pro, monthly 24.00",
            "Total USD 24.00",
        ],
        vendor: "Lumen AI",
        item: Some("digital-equipment"),
        paid_by: Some(("LUMEN", "2026-11-10")),
    },
    DemoInvoice {
        file: "nordbook-air-14.pdf",
        lines: &[
            "Nordbook",
            "Invoice number 7781-2026",
            "Nordbook Air 14 1.249,00",
            "Total EUR 1.249,00",
        ],
        vendor: "Nordbook",
        item: None,
        paid_by: None,
    },
];

fn pdf(lines: &[&str]) -> Vec<u8> {
    let page: Vec<Run> = lines
        .iter()
        .enumerate()
        .map(|(i, text)| Run {
            x: 56.0,
            y: 770.0 - i as f32 * 16.0,
            size: if i == 0 { 16.0 } else { 10.0 },
            font: if i == 0 {
                Font::Helvetica
            } else {
                Font::Courier
            },
            text: (*text).to_string(),
        })
        .collect();
    write::write(&[page])
}

/// Adds the demo invoices; returns how many ops it wrote.
pub fn seed(db: &Db, config_dir: &Path, today: NaiveDate) -> AppResult<usize> {
    let now = today
        .and_hms_opt(12, 0, 0)
        .map(|t| t.and_utc().timestamp_millis())
        .unwrap_or_default();
    let mut count = 0;
    crate::taxrules::files::install(config_dir)?;
    if crate::taxyears::start_from_rules(db, config_dir, chrono::Datelike::year(&today), now)?
        .is_some()
    {
        count += 1;
    }
    let lines = db.read(statements::all_transactions)?;
    for invoice in &INVOICES {
        let pending = add::prepare(db, pdf(invoice.lines), invoice.file.to_string())?;
        let line_id = invoice.paid_by.and_then(|(token, day)| {
            lines
                .iter()
                .find(|t| t.value_date == day && crate::review::title_of(t).contains(token))
                .map(|t| t.id.clone())
        });
        let input = add::AddInput {
            token: String::new(),
            vendor: invoice.vendor.to_string(),
            item_key: invoice.item.map(Into::into),
            line_id,
        };
        add::commit(db, config_dir, pending, &input, now, Source::Seed)?;
        count += 1;
    }
    if files::relocate(db, config_dir)?.is_some() {
        count += 1;
    }
    Ok(count)
}
