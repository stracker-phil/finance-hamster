//! The parts of the tax year page computed from the gathered facts: questions, progress, Kennzahlen, months
//! and the Werbungskostenpauschale note.

use std::collections::{BTreeMap, HashMap};

use chrono::Datelike;

use super::facts::{Facts, Placed, eur, is_countable};
use super::notes;
use super::page::{KennzahlView, MonthView, QuestionView};
use crate::date;
use crate::money::{copy_value, format_plain};
use crate::settings::NumberFormat;
use crate::taxrules::model::{Item, QuestionType, Ruleset};

pub fn question_views(
    ruleset: &Ruleset,
    given: &HashMap<String, i64>,
    format: NumberFormat,
) -> Vec<QuestionView> {
    ruleset
        .questions
        .iter()
        .map(|q| {
            let value = given.get(&q.id).map(|v| match q.kind {
                QuestionType::Integer => v.to_string(),
                QuestionType::Amount => format_plain(*v, format),
            });
            QuestionView {
                id: q.id.clone(),
                label: q.label.clone(),
                help: q.help.clone(),
                unit: q.unit.clone(),
                answered: value.is_some(),
                value: value.unwrap_or_default(),
            }
        })
        .collect()
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `2 drafts to finish · 1 invoice and 2 answers missing`, with the share of done facts in percent.
pub fn progress_text(facts: &Facts, unanswered: usize, questions: usize) -> (String, u32) {
    let mut missing = Vec::new();
    if facts.missing_invoices > 0 {
        missing.push(plural(facts.missing_invoices, "invoice", "invoices"));
    }
    if unanswered > 0 {
        missing.push(plural(unanswered, "answer", "answers"));
    }
    let mut parts = Vec::new();
    if facts.drafts > 0 {
        parts.push(format!(
            "{} to finish",
            plural(facts.drafts, "draft", "drafts")
        ));
    }
    if !missing.is_empty() {
        parts.push(format!("{} missing", missing.join(" and ")));
    }
    let open = facts.drafts + facts.missing_invoices + unanswered;
    let done = facts.finished + (questions - unanswered);
    let percent = (done * 100)
        .checked_div(open + done)
        .map_or(100, |p| p as u32);
    let text = if parts.is_empty() {
        "Ready to close".to_string()
    } else {
        parts.join(" · ")
    };
    (text, percent)
}

/// One view per countable item in rules-file order; `shown` decides which of its rows the filter keeps.
pub fn kennzahlen(
    ruleset: &Ruleset,
    facts: &Facts,
    given: &HashMap<String, i64>,
    filtering: bool,
    shown: &dyn Fn(&Placed, &Item) -> bool,
    format: NumberFormat,
) -> Vec<KennzahlView> {
    ruleset
        .items
        .iter()
        .filter(|i| is_countable(i))
        .map(|item| {
            let own: Vec<&Placed> = facts.placed.iter().filter(|p| p.item == item.key).collect();
            KennzahlView {
                key: item.key.clone(),
                kz: item.kz.clone(),
                name: item.name.clone(),
                copy: copy_value(own.iter().map(|p| p.cents).sum()),
                note: (!own.is_empty())
                    .then(|| notes::note(item, &ruleset.questions, given, format))
                    .flatten(),
                rows: own
                    .into_iter()
                    .filter(|p| shown(p, item))
                    .map(|p| p.row.clone())
                    .collect(),
            }
        })
        .filter(|k| !filtering || !k.rows.is_empty())
        .collect()
}

/// "Whole year" first (rows without a month, such as slices of earlier purchases), then each month.
pub fn months(placed: &[&Placed], format: NumberFormat) -> Vec<MonthView> {
    // Month 0 holds the whole-year rows.
    let mut by_month: BTreeMap<u32, Vec<&Placed>> = BTreeMap::new();
    for p in placed {
        let month = p
            .date
            .as_deref()
            .and_then(date::parse)
            .map_or(0, |day| day.month());
        by_month.entry(month).or_default().push(p);
    }
    by_month
        .into_iter()
        .map(|(month, rows)| MonthView {
            label: if month == 0 {
                "Whole year".to_string()
            } else {
                date::month_name(month).to_string()
            },
            total: eur(rows.iter().map(|p| p.cents).sum(), format),
            rows: rows.into_iter().map(|p| p.row.clone()).collect(),
        })
        .collect()
}

pub fn pauschale_note(ruleset: &Ruleset, total: i64, format: NumberFormat) -> Option<String> {
    ruleset
        .settings
        .werbungskostenpauschale
        .as_ref()
        .filter(|p| total < p.0)
        .map(|p| {
            format!(
                "Below the {} Werbungskostenpauschale, which the tax office applies anyway.",
                eur(p.0, format)
            )
        })
}
