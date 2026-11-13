//! The rows of a tax year that aren't documents: amounts and trips entered on the page, and ANV bank lines
//! without a document.

use std::collections::HashSet;

use super::entries::{self, EntryKind};
use super::facts::{Facts, Placed, countable, day_text, eur, in_year};
use super::page::{RowKind, YearRow};
use crate::date::DateOrder;
use crate::db::Db;
use crate::documents::amounts::{anv_cents, default_work_share};
use crate::documents::store as documents;
use crate::error::AppResult;
use crate::review::title_of as line_title;
use crate::rules::store::{self as decisions, Action};
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::taxrules::model::{Evidence, Ruleset};

/// Amounts count for the whole year; trips sit in their month at the rate valid on their date.
pub(super) fn entry_rows(
    db: &Db,
    year: i32,
    ruleset: &Ruleset,
    format: NumberFormat,
    order: DateOrder,
    facts: &mut Facts,
) -> AppResult<()> {
    for entry in entries::entries(db, year)? {
        let Some(item) = countable(&ruleset.items, Some(&entry.item_key)) else {
            continue;
        };
        let (kind, cents, date, detail) = match entry.kind {
            EntryKind::Amount => {
                let cents = entry.cents.unwrap_or(0);
                if item.evidence() == Evidence::Required && entry.evidence.is_none() {
                    facts.flag_missing_invoice(
                        format!("{} · {}", entry.text, eur(cents, format)),
                        format!("KZ {} needs evidence before it counts", item.kz),
                        Some(entry.id.clone()),
                    );
                    (
                        RowKind::Amount,
                        0,
                        None,
                        format!("{} · not counted without evidence", eur(cents, format)),
                    )
                } else {
                    (
                        RowKind::Amount,
                        cents,
                        None,
                        "Amount for the year".to_string(),
                    )
                }
            }
            EntryKind::Trip => {
                let km = entry.km.unwrap_or(0);
                let rate = entries::rate_on(ruleset, &item.key, &entry.date).unwrap_or(0);
                let detail = format!(
                    "{} · {km} km × {}",
                    day_text(&entry.date, order),
                    eur(rate, format)
                );
                (RowKind::Trip, km * rate, Some(entry.date.clone()), detail)
            }
        };
        let attach = entry.kind == EntryKind::Amount && entry.evidence.is_none();
        let title = if entry.text.is_empty() {
            "Amount".to_string()
        } else {
            entry.text.clone()
        };
        facts.placed.push(Placed {
            item: item.key.clone(),
            date,
            cents,
            row: YearRow {
                id: entry.id.clone(),
                kind,
                title,
                detail,
                amount: eur(cents, format),
                draft: false,
                carried_from: None,
                kz: item.kz.clone(),
                attach,
            },
        });
    }
    Ok(())
}

pub(super) fn decided_lines(
    db: &Db,
    year: i32,
    ruleset: &Ruleset,
    format: NumberFormat,
    order: DateOrder,
    facts: &mut Facts,
) -> AppResult<()> {
    let linked: HashSet<String> = db
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
            facts.flag_missing_invoice(
                format!("{title} · {}", eur(line.cents.abs(), format)),
                format!("{day} · KZ {} needs the invoice before it counts", item.kz),
                None,
            );
            continue;
        }
        let share = default_work_share(Some(ruleset), Some(&item.key));
        let counted = anv_cents(line.cents, share);
        facts.placed.push(Placed {
            item: item.key.clone(),
            date: Some(line.value_date.clone()),
            cents: counted,
            row: YearRow {
                id: line.id.clone(),
                kind: RowKind::Line,
                title,
                detail: format!("{day} · bank line"),
                amount: eur(counted, format),
                draft: false,
                carried_from: None,
                kz: item.kz.clone(),
                attach: false,
            },
        });
    }
    Ok(())
}
