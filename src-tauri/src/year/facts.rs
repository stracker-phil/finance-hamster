//! Gathering what counts in a tax year: its documents (placed by their payments in the year) and the ANV bank
//! lines without a document, plus what still needs attention.

use chrono::Datelike;

use super::page::{AttentionKind, AttentionView, RowKind, YearRow};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::documents::amounts::{anv_cents, default_work_share};
use crate::documents::store::{self as documents, DocState, DocumentRow};
use crate::documents::title_of;
use crate::error::AppResult;
use crate::money::{Money, format_money};
use crate::review::title_of as line_title;
use crate::rules::store::{self as decisions, Action};
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};
use crate::taxrules::model::{Evidence, Item, ItemKind, Ruleset};

pub fn eur(cents: i64, format: NumberFormat) -> String {
    format_money(&Money::eur(cents), format)
}

pub fn in_year(text: &str, year: i32) -> bool {
    date::parse(text).is_some_and(|d| d.year() == year)
}

fn day_text(text: &str, order: DateOrder) -> String {
    date::parse(text)
        .map(|d| date::day_month(d, order))
        .unwrap_or_default()
}

/// Items that show and count: visible and not reported by others.
pub fn is_countable(item: &Item) -> bool {
    item.visible && item.kind != ItemKind::ReportedByOthers
}

fn countable<'a>(items: &'a [Item], key: Option<&String>) -> Option<&'a Item> {
    key.and_then(|k| items.iter().find(|i| &i.key == k && is_countable(i)))
}

/// A row with what places it: its item and its date.
#[derive(Clone)]
pub struct Placed {
    pub item: String,
    pub date: String,
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
    decided_lines(db, year, ruleset, format, order, &mut facts)?;
    facts.placed.sort_by(|a, b| b.date.cmp(&a.date));
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
        if doc.state == DocState::Draft {
            facts.drafts += 1;
            facts.attention.push(AttentionView {
                kind: AttentionKind::Draft,
                detail: "Finish it to lock its values for the tax year".into(),
                title: title.clone(),
                document_id: Some(doc.id.clone()),
            });
        } else {
            facts.finished += 1;
        }
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
    Placed {
        item: item.key.clone(),
        date: day,
        cents: counted,
        row: YearRow {
            id: doc.id.clone(),
            kind: RowKind::Document,
            title,
            detail,
            amount: eur(counted, format),
            draft: doc.state == DocState::Draft,
            carried: false,
            kz: item.kz.clone(),
        },
    }
}

fn decided_lines(
    db: &Db,
    year: i32,
    ruleset: &Ruleset,
    format: NumberFormat,
    order: DateOrder,
    facts: &mut Facts,
) -> AppResult<()> {
    let linked: Vec<String> = db
        .read(documents::links)?
        .into_iter()
        .map(|l| l.transaction_id)
        .collect();
    let decided = db.read(decisions::decisions)?;
    for line in db.read(statements::all_transactions)? {
        if !in_year(&line.value_date, year) || linked.contains(&line.id) {
            continue;
        }
        let Some(decision) = decided.get(&line.id).filter(|d| d.kind == Action::Anv) else {
            continue;
        };
        let Some(item) = countable(&ruleset.items, decision.item_key.as_ref()) else {
            continue;
        };
        let title = line_title(&line);
        let day = day_text(&line.value_date, order);
        if item.evidence() == Evidence::Required {
            facts.missing_invoices += 1;
            facts.attention.push(AttentionView {
                kind: AttentionKind::MissingInvoice,
                title: format!("{title} · {}", eur(line.cents.abs(), format)),
                detail: format!("{day} · KZ {} needs the invoice before it counts", item.kz),
                document_id: None,
            });
            continue;
        }
        let share = default_work_share(Some(ruleset), Some(&item.key));
        let counted = anv_cents(line.cents, share);
        facts.placed.push(Placed {
            item: item.key.clone(),
            date: line.value_date.clone(),
            cents: counted,
            row: YearRow {
                id: line.id.clone(),
                kind: RowKind::Line,
                title,
                detail: format!("{day} · bank line"),
                amount: eur(counted, format),
                draft: false,
                carried: false,
                kz: item.kz.clone(),
            },
        });
    }
    Ok(())
}
