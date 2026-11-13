//! The Document page's view: one document with its linked lines, computed amounts and what is still missing.

use serde::Serialize;
use ts_rs::TS;

use super::amounts::{self, Estimate};
use super::depreciation::{self, Schedule};
use super::guess::Guess;
use super::payments::{self, LineChoice};
use super::store::{self, DocState, DocumentRow, VendorRow};
use super::title_of;
use crate::date::DateOrder;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money, format_plain};
use crate::rules::{self, ItemOption};
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};

/// A vendor the combo box suggests.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct VendorOption {
    pub name: String,
    /// `2 documents`.
    pub detail: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DocumentView {
    pub id: String,
    pub title: String,
    pub state: DocState,
    /// Relative to the data folder.
    pub file: String,
    /// `YYYY-MM-DD`, for the date field.
    pub pay_date: Option<String>,
    pub year: Option<i32>,
    pub lines: Vec<LineChoice>,
    /// `€121.23`.
    pub paid: Option<String>,
    /// The amount comes from the linked lines and can't be typed.
    pub paid_from_lines: bool,
    /// The typed amount as the field shows it.
    pub paid_text: Option<String>,
    pub vendor: Option<String>,
    pub vendors: Vec<VendorOption>,
    /// The payment year's items; empty until a payment date is set.
    pub items: Vec<ItemOption>,
    pub item_key: Option<String>,
    pub work_share: i64,
    /// Present once the amount paid is known.
    pub estimate: Option<Estimate>,
    pub note: String,
    pub invoice_amount: Option<String>,
    pub invoice_currency: String,
    /// The facts still missing before it can be finished, e.g. `payment date`.
    pub missing: Vec<String>,
    /// `3 Apr 2026`, while finished.
    pub finished_on: Option<String>,
    /// `2 Nov 2026`, while excluded.
    pub excluded_on: Option<String>,
    pub exclude_reason: Option<String>,
    /// The schedule of a purchase above the GWG limit.
    pub depreciation: Option<DepreciationView>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SliceView {
    pub year: i32,
    pub amount: String,
    /// `half year`, `frozen`.
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DepreciationView {
    /// What is spread: the counted amount, `€1,500.00`.
    pub cost: String,
    pub years: u32,
    /// The years the rules give, shown beside the choice: `3 years for computer in ruleset 2026 v1`.
    pub rule: String,
    /// Changed from the rule's years for this document.
    pub own: bool,
    /// Why it is spread, in words.
    pub why: String,
    pub slices: Vec<SliceView>,
}

fn year_of(date: &str) -> Option<i32> {
    date.get(..4)?.parse().ok()
}

fn load_document(db: &Db, id: &str) -> AppResult<DocumentRow> {
    db.read(|c| store::document_by_id(c, id))?
        .ok_or_else(|| AppError::NotFound(format!("No document {id}")))
}

fn vendor_name(doc: &DocumentRow, vendors: &[VendorRow]) -> Option<String> {
    doc.vendor_id
        .as_ref()
        .and_then(|v| vendors.iter().find(|x| &x.id == v))
        .map(|v| v.name.clone())
}

fn line_choices(
    db: &Db,
    lines: &[TransactionRow],
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<LineChoice>> {
    let names = payments::account_names(db)?;
    Ok(lines
        .iter()
        .map(|t| payments::line_choice(t, &names, format, order))
        .collect())
}

/// The document's facts as an invoice's, to look for the bank line that paid it.
fn guess_for(doc: &DocumentRow, vendor: Option<String>) -> Guess {
    Guess {
        vendor,
        amount_cents: doc.invoice_cents.or(doc.paid_cents),
        currency: doc.invoice_currency.clone(),
        invoice_number: doc.invoice_number.clone(),
        invoice_date: doc
            .invoice_date
            .as_deref()
            .or(doc.pay_date.as_deref())
            .and_then(crate::date::parse),
    }
}

fn vendor_options(vendors: &[VendorRow], documents: &[DocumentRow]) -> Vec<VendorOption> {
    vendors
        .iter()
        .map(|v| {
            let count = documents
                .iter()
                .filter(|d| d.vendor_id.as_deref() == Some(v.id.as_str()))
                .count();
            VendorOption {
                name: v.name.clone(),
                detail: format!(
                    "{count} {}",
                    if count == 1 { "document" } else { "documents" }
                ),
            }
        })
        .collect()
}

/// What the document paid: the linked lines' sum, or the typed amount without lines.
pub fn paid_cents(doc: &DocumentRow, lines: &[TransactionRow]) -> Option<i64> {
    if lines.is_empty() {
        doc.paid_cents
    } else {
        Some(lines.iter().map(|l| l.cents.abs()).sum())
    }
}

pub fn linked_lines(db: &Db, doc_id: &str) -> AppResult<Vec<TransactionRow>> {
    let mut lines = Vec::new();
    for link in db.read(store::links)? {
        if link.document_id == doc_id
            && let Some(line) =
                db.read(|c| statements::transaction_by_id(c, &link.transaction_id))?
        {
            lines.push(line);
        }
    }
    lines.sort_by(|a, b| a.value_date.cmp(&b.value_date));
    Ok(lines)
}

pub fn document(
    db: &Db,
    id: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<DocumentView> {
    let doc = load_document(db, id)?;
    let vendors = db.read(store::vendors)?;
    let documents = db.read(store::documents)?;
    let lines = linked_lines(db, id)?;
    let year = doc.pay_date.as_deref().and_then(year_of);
    let ruleset = match year {
        Some(y) => rules::ruleset_for_year(db, y)?,
        None => None,
    };
    let paid = paid_cents(&doc, &lines);
    let vendor = vendor_name(&doc, &vendors);
    let missing = super::lifecycle::missing(db, &doc)?
        .into_iter()
        .map(String::from)
        .collect();
    let on = |at: Option<i64>| {
        at.and_then(chrono::DateTime::from_timestamp_millis)
            .map(|t| {
                crate::date::day_month_year(t.with_timezone(&chrono::Local).date_naive(), order)
            })
    };
    let mut estimate = paid.map(|p| {
        amounts::estimate(
            ruleset.as_ref(),
            p,
            doc.item_key.as_deref(),
            doc.work_share,
            year.unwrap_or_default(),
            format,
        )
    });
    let depreciation = depreciation_view(db, &doc, paid, format)?;
    // A depreciated purchase counts only its first slice in the year it was paid.
    if let (Some(estimate), Some(spread)) = (estimate.as_mut(), depreciation.as_ref())
        && let Some(first) = spread.slices.first()
    {
        estimate.formula = format!("year 1 of {} · {} in total", spread.years, spread.cost);
        estimate.anv = first.amount.clone();
    }
    Ok(DocumentView {
        title: title_of(&doc, &vendors),
        id: doc.id.clone(),
        state: doc.state,
        file: doc.file.clone(),
        pay_date: doc.pay_date.clone(),
        year,
        lines: line_choices(db, &lines, format, order)?,
        paid: paid.map(|c| format_money(&Money::eur(c), format)),
        paid_from_lines: !lines.is_empty(),
        paid_text: doc.paid_cents.map(|c| format_plain(c, format)),
        vendor,
        vendors: vendor_options(&vendors, &documents),
        items: ruleset
            .as_ref()
            .map(rules::item_options)
            .unwrap_or_default(),
        item_key: doc.item_key.clone(),
        work_share: doc.work_share,
        estimate,
        note: doc.note.clone(),
        invoice_amount: doc.invoice_cents.map(|c| format_plain(c, format)),
        invoice_currency: doc.invoice_currency.clone().unwrap_or_else(|| "EUR".into()),
        missing,
        finished_on: on(doc.finished_at),
        excluded_on: on(doc.excluded_at),
        exclude_reason: doc.exclude_reason.clone(),
        depreciation,
    })
}

/// Lines the bank-line picker offers: matches for the document's facts, or lines containing `query`.
pub fn line_options(
    db: &Db,
    id: &str,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<LineChoice>> {
    let doc = load_document(db, id)?;
    let lines = if query.trim().is_empty() {
        let vendors = db.read(store::vendors)?;
        payments::candidates(db, &guess_for(&doc, vendor_name(&doc, &vendors)))?
    } else {
        payments::search(db, query)?
    };
    line_choices(db, &lines, format, order)
}

/// Only a draft can be edited; finished and excluded documents say what to do first.
pub fn editable(doc: &DocumentRow) -> AppResult<()> {
    match doc.state {
        DocState::Draft => Ok(()),
        DocState::Finished => Err(AppError::InvalidInput(
            "This document is finished; unlock it to change it".into(),
        )),
        DocState::Excluded => Err(AppError::InvalidInput(
            "This document is excluded; restore it to change it".into(),
        )),
    }
}

/// The Depreciation block of a document whose counted amount is spread over several years.
fn depreciation_view(
    db: &Db,
    doc: &DocumentRow,
    paid: Option<i64>,
    format: NumberFormat,
) -> AppResult<Option<DepreciationView>> {
    let (Some(paid), Some(day), Some(key)) = (
        paid,
        doc.pay_date.as_deref().and_then(crate::date::parse),
        doc.item_key.as_ref(),
    ) else {
        return Ok(None);
    };
    let year = chrono::Datelike::year(&day);
    let Some(ruleset) = rules::ruleset_for_year(db, year)? else {
        return Ok(None);
    };
    let Some(item) = ruleset.items.iter().find(|i| &i.key == key) else {
        return Ok(None);
    };
    let cost = amounts::anv_cents(paid, doc.work_share);
    let schedule = depreciation::schedule(cost, day, item, &ruleset, doc.useful_life);
    let Schedule::Spread {
        asset_class,
        rule_years,
        years,
        half_year,
        slices,
    } = schedule
    else {
        return Ok(None);
    };
    let eur = |c: i64| format_money(&Money::eur(c), format);
    let closed: Vec<i32> = db
        .read(crate::taxyears::rows)?
        .into_iter()
        .filter(|y| y.state == crate::taxyears::YearState::Closed)
        .map(|y| y.year)
        .collect();
    let mut why = format!(
        "{} is above the GWG limit of {}, so it counts over {years} {}",
        eur(cost),
        eur(ruleset.settings.gwg_limit.0),
        if years == 1 { "year" } else { "years" }
    );
    if half_year {
        why.push_str(&format!(
            "; bought on or after {}, the first and the last year count half",
            ruleset.settings.half_year_from
        ));
    }
    why.push('.');
    Ok(Some(DepreciationView {
        cost: eur(cost),
        years,
        rule: format!(
            "{rule_years} years for {asset_class} in ruleset {}",
            ruleset.ruleset
        ),
        own: doc.useful_life.is_some_and(|y| y != rule_years),
        why,
        slices: slices
            .iter()
            .map(|s| SliceView {
                year: s.year,
                amount: eur(s.cents),
                note: if closed.contains(&s.year) {
                    Some("frozen".into())
                } else if s.half {
                    Some("half year".into())
                } else {
                    None
                },
            })
            .collect(),
    }))
}
