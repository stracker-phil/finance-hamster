//! Adding an invoice PDF: refuse a file already added, read what the text says, suggest a vendor, an item and
//! the bank line that paid it (`prepare`, nothing written yet); then copy it to `docs/inbox/` and store it as
//! a draft (`commit`, one undoable op; `files::relocate` moves it to its year once it has a payment date).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::guess::{self, Guess};
use super::payments::{self, days_apart};
use super::store::{self, DocState, DocumentRow, LinkRow, VendorRow};
use super::{amounts, files, title_of};
use crate::date::{self, DateOrder};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;
use crate::money::{Money, format_money};
use crate::platform::pdf::Pdf;
use crate::rules::{self, ItemOption};
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::tr;

/// Days within which the same vendor and amount look like the same invoice.
const NEAR_DUPLICATE_DAYS: i64 = 3;

/// A read PDF waiting for the owner's Add.
#[derive(Clone, Debug)]
pub struct AddPending {
    pub bytes: Vec<u8>,
    pub hash: String,
    pub original_name: String,
    pub guess: Guess,
}

/// PDFs read but not yet added, by token.
#[derive(Default)]
pub struct AddQueue(Mutex<HashMap<String, AddPending>>);

impl AddQueue {
    pub fn put(&self, pending: AddPending) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.0.lock_unpoisoned().insert(token.clone(), pending);
        token
    }

    pub fn take(&self, token: &str) -> Option<AddPending> {
        self.0.lock_unpoisoned().remove(token)
    }

    pub fn get(&self, token: &str) -> Option<AddPending> {
        self.0.lock_unpoisoned().get(token).cloned()
    }
}

/// What the Add document sheet shows.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AddPreview {
    pub token: String,
    pub file_name: String,
    /// A document with the same vendor and amount a few days apart.
    pub near_duplicate: Option<String>,
    pub vendor: Option<String>,
    /// Read from the PDF, not one of the known vendors.
    pub vendor_from_pdf: bool,
    /// `VEC-2026-1109 · 9 Nov 2026`.
    pub invoice: Option<String>,
    /// `USD 129.00`.
    pub invoice_amount: Option<String>,
    pub items: Vec<ItemOption>,
    pub item_key: Option<String>,
    /// `last used for Pixelforge Ltd.`.
    pub item_why: Option<String>,
    pub lines: Vec<payments::LineChoice>,
}

/// What the owner confirmed in the sheet.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AddInput {
    pub token: String,
    pub vendor: String,
    pub item_key: Option<String>,
    pub line_id: Option<String>,
}

/// The invoice's currency; euro when none is printed.
pub(super) fn currency_of(g: &Guess) -> String {
    g.currency.clone().unwrap_or_else(|| "EUR".into())
}

fn text_of(bytes: &[u8]) -> Vec<String> {
    Pdf::from_bytes(bytes)
        .and_then(|pdf| pdf.text_pages())
        .map(|pages| pages.into_iter().flatten().collect())
        .unwrap_or_default()
}

/// Reads a PDF; refuses one already added.
pub fn prepare(db: &Db, bytes: Vec<u8>, original_name: String) -> AppResult<AddPending> {
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::InvalidInput(tr!(
            "{original_name} isn't a PDF",
            "{original_name} ist kein PDF"
        )));
    }
    let hash = crate::statements::file_hash(&bytes);
    if let Some(existing) = db.read(|c| store::document_by_hash(c, &hash))? {
        let vendors = db.read(store::vendors)?;
        let title = title_of(&existing, &vendors);
        return Err(AppError::InvalidInput(tr!(
            "Already added: {title}",
            "Schon hinzugefügt: {title}"
        )));
    }
    let names: Vec<String> = db
        .read(store::vendors)?
        .into_iter()
        .map(|v| v.name)
        .collect();
    let guess = guess::guess(&text_of(&bytes), &names);
    Ok(AddPending {
        bytes,
        hash,
        original_name,
        guess,
    })
}

fn find_vendor<'a>(vendors: &'a [VendorRow], name: &str) -> Option<&'a VendorRow> {
    vendors
        .iter()
        .find(|v| v.name.trim().eq_ignore_ascii_case(name.trim()))
}

fn near_duplicate(db: &Db, guess: &Guess) -> AppResult<Option<String>> {
    let (Some(name), Some(amount), Some(day)) =
        (&guess.vendor, guess.amount_cents, guess.invoice_date)
    else {
        return Ok(None);
    };
    let vendors = db.read(store::vendors)?;
    let Some(vendor) = find_vendor(&vendors, name) else {
        return Ok(None);
    };
    let similar = db.read(store::documents)?.into_iter().find(|d| {
        d.vendor_id.as_deref() == Some(vendor.id.as_str())
            && (d.invoice_cents == Some(amount) || d.paid_cents == Some(amount))
            && [&d.invoice_date, &d.pay_date].iter().any(|when| {
                when.as_deref()
                    .and_then(|w| days_apart(w, day))
                    .is_some_and(|n| n <= NEAR_DUPLICATE_DAYS)
            })
    });
    Ok(similar.map(|d| {
        let title = title_of(&d, &vendors);
        tr!(
            "Looks like {title}: same vendor and amount a few days apart",
            "Sieht aus wie {title}: gleicher Händler und Betrag, wenige Tage auseinander"
        )
    }))
}

/// The vendor's default item, else the item of its newest document, with the reason.
fn item_for(db: &Db, vendor: Option<&VendorRow>) -> AppResult<(Option<String>, Option<String>)> {
    let Some(vendor) = vendor else {
        return Ok((None, None));
    };
    if let Some(key) = &vendor.default_item_key {
        return Ok((
            Some(key.clone()),
            Some(tr!("default for {}", "Standard für {}", vendor.name)),
        ));
    }
    let last = db
        .read(store::documents)?
        .into_iter()
        .filter(|d| d.vendor_id.as_deref() == Some(vendor.id.as_str()))
        .filter_map(|d| d.item_key)
        .next_back();
    let why = last
        .as_ref()
        .map(|_| tr!("last used for {}", "zuletzt verwendet für {}", vendor.name));
    Ok((last, why))
}

pub fn preview(
    db: &Db,
    token: String,
    pending: &AddPending,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<AddPreview> {
    let g = &pending.guess;
    let vendors = db.read(store::vendors)?;
    let known = g.vendor.as_deref().and_then(|n| find_vendor(&vendors, n));
    let (item_key, item_why) = item_for(db, known)?;
    let account_names = payments::account_names(db)?;
    let invoice = [
        g.invoice_number.clone(),
        g.invoice_date.map(|d| date::day_month_year(d, order)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    Ok(AddPreview {
        token,
        file_name: pending.original_name.clone(),
        near_duplicate: near_duplicate(db, g)?,
        vendor: known.map(|v| v.name.clone()).or_else(|| g.vendor.clone()),
        vendor_from_pdf: known.is_none() && g.vendor.is_some(),
        invoice: (!invoice.is_empty()).then_some(invoice),
        invoice_amount: g.amount_cents.map(|cents| {
            format_money(
                &Money {
                    cents,
                    currency: currency_of(g),
                },
                format,
            )
        }),
        items: rules::items(db)?,
        item_key,
        item_why,
        lines: payments::candidates(db, g)?
            .iter()
            .map(|t| payments::line_choice(t, &account_names, format, order))
            .collect(),
    })
}

/// Copies the PDF to the inbox and stores the draft, a new vendor and the link as one op.
pub fn commit(
    db: &Db,
    config_dir: &Path,
    pending: AddPending,
    input: &AddInput,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let name = input.vendor.trim();
    if name.is_empty() {
        return Err(AppError::InvalidField {
            field: "document.vendor".into(),
            message: tr!("Name the vendor", "Gib den Händler an"),
        });
    }
    let line = match &input.line_id {
        Some(id) => Some(
            db.read(|c| statements::transaction_by_id(c, id))?
                .ok_or_else(|| AppError::NotFound(tr!("No line {id}", "Keine Buchung {id}")))?,
        ),
        None => None,
    };
    crate::taxyears::refuse_closed(db, &[line.as_ref().map(|l| l.value_date.as_str())])?;
    let items = rules::items(db)?;
    if let Some(key) = &input.item_key
        && !items.iter().any(|i| &i.key == key)
    {
        return Err(AppError::InvalidField {
            field: "document.item".into(),
            message: tr!(
                "Choose one of the year's items",
                "Wähle einen der Posten des Jahres"
            ),
        });
    }
    let mut changes = Vec::new();
    let vendors = db.read(store::vendors)?;
    let vendor_id = match find_vendor(&vendors, name) {
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
    let ruleset = rules::ruleset(db)?;
    let g = &pending.guess;
    let file = files::copy_in(
        config_dir,
        &pending.original_name,
        &pending.bytes,
        &pending.hash,
    )?;
    let doc = DocumentRow {
        id: uuid::Uuid::new_v4().to_string(),
        state: DocState::Draft,
        file,
        original_name: pending.original_name.clone(),
        file_hash: pending.hash.clone(),
        vendor_id: Some(vendor_id),
        pay_date: line.as_ref().map(|l| l.value_date.clone()),
        paid_cents: None,
        item_key: input.item_key.clone(),
        work_share: amounts::default_work_share(ruleset.as_ref(), input.item_key.as_deref()),
        note: String::new(),
        invoice_cents: g.amount_cents,
        invoice_currency: g.amount_cents.map(|_| currency_of(g)),
        invoice_number: g.invoice_number.clone(),
        invoice_date: g.invoice_date.map(date::iso),
        created_at: now,
        finished_at: None,
        excluded_at: None,
        exclude_reason: None,
        useful_life: None,
    };
    changes.push(Change::insert(store::DOCUMENTS, &doc)?);
    if let Some(line) = &line {
        super::refuse_paid_elsewhere(db, &line.id, &doc.id)?;
        let link = LinkRow {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: doc.id.clone(),
            transaction_id: line.id.clone(),
            created_at: now,
        };
        changes.push(Change::insert(store::LINKS, &link)?);
    }
    db.write(
        Op::new(
            tr!("Add Document {name}", "Beleg hinzufügen: {name}"),
            changes,
        ),
        source,
    )
}
