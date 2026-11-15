//! Changing one fact of a draft document, as one undoable op. The file moves afterwards (`files::relocate`).

use serde::Deserialize;
use ts_rs::TS;

use super::store::{self, DocumentRow, LinkRow, VendorRow};
use super::view::{editable, linked_lines};
use super::{amounts, title_of};
use crate::date;
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::rules;
use crate::statements::store as statements;

/// One field of the Document page.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "field", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum DocumentEdit {
    PayDate {
        value: Option<String>,
    },
    /// Typed while no bank line is linked.
    Paid {
        value: Option<String>,
    },
    /// A new name creates the vendor.
    Vendor {
        name: String,
    },
    Item {
        key: Option<String>,
    },
    /// Percent.
    /// As typed: `60`, `60 %`.
    WorkShare {
        value: String,
    },
    Note {
        text: String,
    },
    Invoice {
        amount: Option<String>,
        currency: String,
    },
    /// Years to depreciate over; none returns to the item's asset class.
    UsefulLife {
        years: Option<u32>,
    },
    Link {
        line: String,
    },
    Unlink {
        line: String,
    },
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidField {
        field: format!("document.{field}"),
        message: message.into(),
    }
}

fn typed(text: &Option<String>) -> Option<&str> {
    text.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// The amount typed into `field`, without sign; None when nothing was typed.
fn amount_cents(value: &Option<String>, field: &str) -> AppResult<Option<i64>> {
    typed(value)
        .map(|t| {
            parse_cents(t)
                .map(i64::abs)
                .ok_or_else(|| invalid(field, "This isn't an amount"))
        })
        .transpose()
}

fn year_of(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}

/// Changes one field; None when the value is the one already stored.
pub fn edit(
    db: &Db,
    id: &str,
    change: &DocumentEdit,
    now: i64,
    source: Source,
) -> AppResult<Option<Written>> {
    let doc = db
        .read(|c| store::document_by_id(c, id))?
        .ok_or_else(|| AppError::NotFound(format!("No document {id}")))?;
    editable(&doc)?;
    let vendors = db.read(store::vendors)?;
    let title = title_of(&doc, &vendors);
    let mut after = doc.clone();
    let mut changes = Vec::new();
    match change {
        DocumentEdit::PayDate { value } => {
            after.pay_date = typed(value)
                .map(|text| {
                    date::parse(text)
                        .map(date::iso)
                        .ok_or_else(|| invalid("payDate", "This isn't a date"))
                })
                .transpose()?;
        }
        DocumentEdit::Paid { value } => {
            if !linked_lines(db, id)?.is_empty() {
                return Err(invalid("paid", "The linked bank line is the amount paid"));
            }
            after.paid_cents = amount_cents(value, "paid")?;
        }
        DocumentEdit::Vendor { name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err(invalid("vendor", "Name the vendor"));
            }
            let vendor_id = match vendors.iter().find(|v| v.name.eq_ignore_ascii_case(name)) {
                Some(v) => v.id.clone(),
                None => {
                    let vendor = VendorRow {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        default_item_key: None,
                        created_at: now,
                    };
                    changes.push(Change::insert(store::VENDORS, &vendor)?);
                    vendor.id
                }
            };
            after.vendor_id = Some(vendor_id);
        }
        DocumentEdit::Item { key } => match key.as_deref().filter(|k| !k.is_empty()) {
            None => after.item_key = None,
            Some(key) => {
                let year = doc
                    .pay_date
                    .as_deref()
                    .and_then(year_of)
                    .ok_or_else(|| invalid("item", "Set the payment date first"))?;
                let ruleset = rules::ruleset_for_year(db, year)?;
                if !ruleset
                    .as_ref()
                    .is_some_and(|r| r.items.iter().any(|i| i.key == key && i.visible))
                {
                    return Err(invalid("item", "Choose one of the year's items"));
                }
                after.item_key = Some(key.to_string());
                after.work_share = amounts::default_work_share(ruleset.as_ref(), Some(key));
            }
        },
        DocumentEdit::WorkShare { value } => {
            after.work_share = value
                .trim()
                .trim_end_matches('%')
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|share| (0..=100).contains(share))
                .ok_or_else(|| invalid("workShare", "A share between 0 and 100 %"))?;
        }
        DocumentEdit::Note { text } => after.note = text.clone(),
        DocumentEdit::Invoice { amount, currency } => {
            let code = currency.trim().to_uppercase();
            if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
                return Err(invalid("invoice", "A three-letter currency code"));
            }
            after.invoice_cents = amount_cents(amount, "invoice")?;
            after.invoice_currency = after.invoice_cents.map(|_| code);
        }
        DocumentEdit::UsefulLife { years } => {
            if years.is_some_and(|y| !(1..=50).contains(&y)) {
                return Err(invalid("usefulLife", "Between 1 and 50 years"));
            }
            after.useful_life = *years;
        }
        DocumentEdit::Link { line } => {
            let row = db
                .read(|c| statements::transaction_by_id(c, line))?
                .ok_or_else(|| AppError::NotFound(format!("No line {line}")))?;
            let links = db.read(store::links)?;
            if links
                .iter()
                .any(|l| l.document_id == doc.id && &l.transaction_id == line)
            {
                return Err(invalid("lines", "This line is linked already"));
            }
            super::refuse_paid_elsewhere(db, line, &doc.id)?;
            changes.push(Change::insert(
                store::LINKS,
                &LinkRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    document_id: doc.id.clone(),
                    transaction_id: row.id.clone(),
                    created_at: now,
                },
            )?);
            if after.pay_date.is_none() {
                after.pay_date = Some(row.value_date.clone());
            }
            after.paid_cents = None;
        }
        DocumentEdit::Unlink { line } => {
            let link = db
                .read(store::links)?
                .into_iter()
                .find(|l| l.document_id == doc.id && &l.transaction_id == line)
                .ok_or_else(|| AppError::NotFound(format!("Line {line} isn't linked")))?;
            changes.push(Change::delete(store::LINKS, &link)?);
        }
    }
    write(db, &doc, after, changes, &title, source)
}

fn write(
    db: &Db,
    before: &DocumentRow,
    after: DocumentRow,
    mut changes: Vec<Change>,
    title: &str,
    source: Source,
) -> AppResult<Option<Written>> {
    if &after != before {
        crate::taxyears::refuse_closed(
            db,
            &[before.pay_date.as_deref(), after.pay_date.as_deref()],
        )?;
        changes.push(Change::update(store::DOCUMENTS, before, &after)?);
    }
    if changes.is_empty() {
        return Ok(None);
    }
    db.write(Op::new(format!("Edit {title}"), changes), source)
        .map(Some)
}
