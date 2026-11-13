//! Gathering what counts in a tax year: its documents (placed by their payments in the year) and the ANV bank
//! lines without a document, amounts and trips (`rows`), plus what still needs attention.

use chrono::Datelike;

use super::page::{AttentionKind, AttentionView, RowKind, YearRow};
use super::rows;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::documents::amounts::anv_cents;
use crate::documents::depreciation::{self, Schedule};
use crate::documents::store::{self as documents, DocState, DocumentRow};
use crate::documents::title_of;
use crate::error::AppResult;
use crate::money::{Money, format_money};
use crate::rules;
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};
use crate::taxrules::model::{Item, ItemKind, Ruleset};

pub fn eur(cents: i64, format: NumberFormat) -> String {
    format_money(&Money::eur(cents), format)
}

pub fn in_year(text: &str, year: i32) -> bool {
    date::parse(text).is_some_and(|d| d.year() == year)
}

pub(super) fn day_text(text: &str, order: DateOrder) -> String {
    date::parse(text)
        .map(|d| date::day_month(d, order))
        .unwrap_or_default()
}

/// Items that show and count: visible and not reported by others.
pub fn is_countable(item: &Item) -> bool {
    item.visible && item.kind != ItemKind::ReportedByOthers
}

pub(super) fn countable<'a>(items: &'a [Item], key: Option<&String>) -> Option<&'a Item> {
    key.and_then(|k| items.iter().find(|i| &i.key == k && is_countable(i)))
}

/// A row with what places it: its item and its date.
#[derive(Clone)]
pub struct Placed {
    pub item: String,
    /// None for a row of the whole year, such as a slice of an earlier purchase.
    pub date: Option<String>,
    pub cents: i64,
    pub row: YearRow,
}

#[derive(Default)]
pub struct Facts {
    pub placed: Vec<Placed>,
    pub attention: Vec<AttentionView>,
    pub drafts: usize,
    pub finished: usize,
    pub missing_invoices: usize,
    /// Excluded documents paid in the year.
    pub archived: usize,
}

impl Facts {
    /// A payment that counts only once its invoice or evidence is there.
    pub(super) fn flag_missing_invoice(
        &mut self,
        title: String,
        detail: String,
        entry_id: Option<String>,
    ) {
        self.missing_invoices += 1;
        self.attention.push(AttentionView {
            kind: AttentionKind::MissingInvoice,
            title,
            detail,
            document_id: None,
            entry_id,
        });
    }
}

/// Everything that counts in `year`, newest first, and what needs attention.
pub fn gather(
    db: &Db,
    year: i32,
    ruleset: &Ruleset,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Facts> {
    let mut facts = Facts::default();
    document_rows(db, year, &ruleset.items, format, order, &mut facts)?;
    rows::decided_lines(db, year, ruleset, format, order, &mut facts)?;
    rows::entry_rows(db, year, ruleset, format, order, &mut facts)?;
    // Whole-year rows first, as in the months view; then the newest payment first.
    facts.placed.sort_by(|a, b| {
        a.date
            .is_some()
            .cmp(&b.date.is_some())
            .then(b.date.cmp(&a.date))
    });
    Ok(facts)
}

fn document_rows(
    db: &Db,
    year: i32,
    items: &[Item],
    format: NumberFormat,
    order: DateOrder,
    facts: &mut Facts,
) -> AppResult<()> {
    let vendors = db.read(documents::vendors)?;
    let links = db.read(documents::links)?;
    for doc in db.read(documents::documents)? {
        if doc.state == DocState::Excluded {
            if doc.pay_date.as_deref().is_some_and(|d| in_year(d, year)) {
                facts.archived += 1;
            }
            continue;
        }
        let title = title_of(&doc, &vendors);
        let Some(pay_date) = doc.pay_date.clone() else {
            facts.attention.push(AttentionView {
                kind: AttentionKind::Undated,
                detail: "Set its payment date to place it in a tax year".into(),
                title,
                document_id: Some(doc.id.clone()),
                entry_id: None,
            });
            continue;
        };
        let mut lines: Vec<TransactionRow> = Vec::new();
        for link in links.iter().filter(|l| l.document_id == doc.id) {
            if let Some(line) =
                db.read(|c| statements::transaction_by_id(c, &link.transaction_id))?
            {
                lines.push(line);
            }
        }
        match spread_in(db, &doc, &lines, year)? {
            Spreading::Whole => {}
            Spreading::OtherYear => continue,
            Spreading::Here(spread) => {
                place_slice(&doc, title, &pay_date, &spread, year, format, order, facts);
                continue;
            }
        }
        let this_year: Vec<&TransactionRow> = lines
            .iter()
            .filter(|l| in_year(&l.value_date, year))
            .collect();
        // A document belongs to the years its payments fall in; without linked lines, to its payment date's.
        let (belongs, paid) = if lines.is_empty() {
            let belongs = in_year(&pay_date, year);
            (belongs, doc.paid_cents.filter(|_| belongs))
        } else {
            let sum: i64 = this_year.iter().map(|l| l.cents.abs()).sum();
            (
                !this_year.is_empty(),
                (!this_year.is_empty()).then_some(sum),
            )
        };
        if !belongs {
            continue;
        }
        count_state(&doc, &title, facts);
        let Some(item) = countable(items, doc.item_key.as_ref()) else {
            continue;
        };
        // Listed under its first payment in this year, else its payment date.
        let day = this_year
            .iter()
            .map(|l| l.value_date.clone())
            .min()
            .unwrap_or(pay_date);
        facts
            .placed
            .push(placed_document(&doc, item, title, day, paid, format, order));
    }
    Ok(())
}

/// Counts a document as finished, or as a draft that needs attention.
fn count_state(doc: &DocumentRow, title: &str, facts: &mut Facts) {
    if doc.state == DocState::Draft {
        facts.drafts += 1;
        facts.attention.push(AttentionView {
            kind: AttentionKind::Draft,
            detail: "Finish it to lock its values for the tax year".into(),
            title: title.to_string(),
            document_id: Some(doc.id.clone()),
            entry_id: None,
        });
    } else {
        facts.finished += 1;
    }
}

/// A depreciated document's slice in one year.
struct Spread {
    item: Item,
    purchase_year: i32,
    cost: i64,
    years: u32,
    slice: i64,
}

enum Spreading {
    /// Counts in full where it was paid: the usual way.
    Whole,
    /// Depreciated, with no slice in this year.
    OtherYear,
    Here(Box<Spread>),
}

/// Whether the document is depreciated, and its slice in `year`. The purchase year's rules decide.
fn spread_in(
    db: &Db,
    doc: &DocumentRow,
    lines: &[TransactionRow],
    year: i32,
) -> AppResult<Spreading> {
    let (Some(day), Some(key)) = (
        doc.pay_date.as_deref().and_then(date::parse),
        doc.item_key.as_ref(),
    ) else {
        return Ok(Spreading::Whole);
    };
    let paid = if lines.is_empty() {
        doc.paid_cents
    } else {
        Some(lines.iter().map(|l| l.cents.abs()).sum())
    };
    let Some(paid) = paid else {
        return Ok(Spreading::Whole);
    };
    let Some(ruleset) = rules::ruleset_for_year(db, day.year())? else {
        return Ok(Spreading::Whole);
    };
    let Some(item) = countable(&ruleset.items, Some(key)) else {
        return Ok(Spreading::Whole);
    };
    let cost = anv_cents(paid, doc.work_share);
    match depreciation::schedule(cost, day, item, &ruleset, doc.useful_life) {
        Schedule::Full => Ok(Spreading::Whole),
        Schedule::Spread { years, slices, .. } => {
            Ok(match slices.iter().find(|s| s.year == year) {
                None => Spreading::OtherYear,
                Some(slice) => Spreading::Here(Box::new(Spread {
                    item: item.clone(),
                    purchase_year: day.year(),
                    cost,
                    years,
                    slice: slice.cents,
                })),
            })
        }
    }
}

/// A depreciated document's slice in `year`; it counts as a draft or finished only in its purchase year.
#[allow(clippy::too_many_arguments)]
fn place_slice(
    doc: &DocumentRow,
    title: String,
    pay_date: &str,
    spread: &Spread,
    year: i32,
    format: NumberFormat,
    order: DateOrder,
    facts: &mut Facts,
) {
    let carried = year > spread.purchase_year;
    if !carried {
        count_state(doc, &title, facts);
    }
    let when = if carried {
        format!("From {}", spread.purchase_year)
    } else {
        day_text(pay_date, order)
    };
    let year_no = year - spread.purchase_year + 1;
    let detail = format!(
        "{when} · year {year_no} of depreciating {} over {} {}",
        eur(spread.cost, format),
        spread.years,
        if spread.years == 1 { "year" } else { "years" }
    );
    // Slices of earlier purchases belong to the whole year, not to a month.
    let date = (!carried).then(|| pay_date.to_string());
    facts.placed.push(document_placed(
        doc,
        &spread.item,
        title,
        date,
        detail,
        spread.slice,
        carried.then_some(spread.purchase_year),
        format,
    ));
}

#[allow(clippy::too_many_arguments)]
fn document_placed(
    doc: &DocumentRow,
    item: &Item,
    title: String,
    date: Option<String>,
    detail: String,
    cents: i64,
    carried_from: Option<i32>,
    format: NumberFormat,
) -> Placed {
    Placed {
        item: item.key.clone(),
        date,
        cents,
        row: YearRow {
            id: doc.id.clone(),
            kind: RowKind::Document,
            title,
            detail,
            amount: eur(cents, format),
            draft: doc.state == DocState::Draft,
            carried_from,
            kz: item.kz.clone(),
            attach: false,
        },
    }
}

fn placed_document(
    doc: &DocumentRow,
    item: &Item,
    title: String,
    day: String,
    paid: Option<i64>,
    format: NumberFormat,
    order: DateOrder,
) -> Placed {
    let counted = paid.map(|p| anv_cents(p, doc.work_share)).unwrap_or(0);
    let shown = day_text(&day, order);
    let detail = match paid {
        Some(p) if doc.work_share != 100 => {
            format!("{shown} · {} % of {}", doc.work_share, eur(p, format))
        }
        Some(_) => shown,
        None => format!("{shown} · amount paid missing"),
    };
    document_placed(doc, item, title, Some(day), detail, counted, None, format)
}
