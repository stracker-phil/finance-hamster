//! The Documents page: every document not excluded, grouped by tax year (newest first) plus the Inbox of
//! undated ones, filtered by what the owner types into "Is it already here?".

use chrono::Datelike;
use serde::Serialize;
use ts_rs::TS;

use super::amounts::anv_cents;
use super::store::{self, DocState, DocumentRow, VendorRow};
use super::title_of;
use super::view::{linked_lines, paid_cents};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::money::{Money, format_money, format_plain};
use crate::rules;
use crate::settings::NumberFormat;
use crate::{tr, tr_plural};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DocumentRowView {
    pub id: String,
    pub title: String,
    /// `10 Nov · KZ 720 Fachliteratur`.
    pub sub: String,
    pub state: DocState,
    /// The ANV amount, empty while unknown.
    pub anv: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct DocumentGroup {
    /// `Tax year 2026` or `Inbox`.
    pub title: String,
    /// None for the Inbox.
    pub year: Option<i32>,
    /// `3 documents`.
    pub count: String,
    /// `€1,540.00 paid`: what the year's documents cost, not what counts (the tax year page has that); None
    /// for the Inbox.
    pub total: Option<String>,
    pub documents: Vec<DocumentRowView>,
}

fn matches(
    doc: &DocumentRow,
    title: &str,
    paid: Option<i64>,
    query: &str,
    format: NumberFormat,
) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    let amounts = [paid, doc.invoice_cents]
        .into_iter()
        .flatten()
        .map(|c| format_plain(c, format));
    [title, &doc.original_name, &doc.note]
        .into_iter()
        .map(str::to_string)
        .chain(doc.invoice_number.clone())
        .chain(amounts)
        .any(|text| text.to_lowercase().contains(&needle))
}

fn sub_line(doc: &DocumentRow, item: Option<String>, order: DateOrder) -> String {
    let when = doc
        .pay_date
        .as_deref()
        .and_then(date::parse)
        .map(|d| date::day_month(d, order))
        .unwrap_or_else(|| tr!("Not dated", "Ohne Datum"));
    std::iter::once(when)
        .chain(item)
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A listed document with what sorts and groups it.
struct Listed {
    year: Option<i32>,
    pay_date: String,
    row: DocumentRowView,
    anv_cents: i64,
}

fn group_of(listed: &[Listed], format: NumberFormat) -> DocumentGroup {
    let year = listed.first().and_then(|l| l.year);
    let n = listed.len();
    DocumentGroup {
        title: year.map_or_else(
            || tr!("Inbox", "Eingang"),
            |y| tr!("Tax year {y}", "Steuerjahr {y}"),
        ),
        year,
        count: tr_plural!(
            n,
            "{n} document",
            "{n} documents",
            "{n} Beleg",
            "{n} Belege"
        ),
        total: year.map(|_| {
            let paid = format_money(
                &Money::eur(listed.iter().map(|l| l.anv_cents).sum()),
                format,
            );
            tr!("{paid} paid", "{paid} bezahlt")
        }),
        documents: listed.iter().map(|l| l.row.clone()).collect(),
    }
}

pub fn documents(
    db: &Db,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<DocumentGroup>> {
    let vendors: Vec<VendorRow> = db.read(store::vendors)?;
    let mut listed: Vec<Listed> = Vec::new();
    for doc in db.read(store::documents)? {
        if doc.state == DocState::Excluded {
            continue;
        }
        let lines = linked_lines(db, &doc.id)?;
        let title = title_of(&doc, &vendors);
        let paid = paid_cents(&doc, &lines);
        if !matches(&doc, &title, paid, query, format) {
            continue;
        }
        let year = doc
            .pay_date
            .as_deref()
            .and_then(date::parse)
            .map(|d| d.year());
        let item = match (year, &doc.item_key) {
            (Some(y), Some(key)) => rules::ruleset_for_year(db, y)?
                .and_then(|r| r.items.into_iter().find(|i| &i.key == key))
                .map(|i| format!("KZ {} {}", i.kz, i.display_name())),
            _ => None,
        };
        let anv = paid.map(|p| anv_cents(p, doc.work_share));
        listed.push(Listed {
            year,
            pay_date: doc.pay_date.clone().unwrap_or_default(),
            row: DocumentRowView {
                id: doc.id.clone(),
                sub: sub_line(&doc, item, order),
                state: doc.state,
                anv: anv
                    .map(|c| format_money(&Money::eur(c), format))
                    .unwrap_or_default(),
                title,
            },
            anv_cents: anv.unwrap_or(0),
        });
    }
    // The Inbox first, then the years newest first; inside a group the newest payment first.
    listed.sort_by(|a, b| {
        b.year
            .unwrap_or(i32::MAX)
            .cmp(&a.year.unwrap_or(i32::MAX))
            .then(b.pay_date.cmp(&a.pay_date))
    });
    Ok(listed
        .chunk_by(|a, b| a.year == b.year)
        .map(|chunk| group_of(chunk, format))
        .collect())
}
