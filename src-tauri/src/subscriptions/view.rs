//! The Subscriptions page: active ones grouped by next charge, the ended ones apart, and what the New sheet
//! starts from.

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

use super::Book;
use super::edit::SubscriptionForm;
use super::matcher::Matchers;
use super::schedule::Running;
use super::store::{Interval, PlanRow, Reminder, SubscriptionRow};
use crate::date::{self, DateOrder};
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money, format_plain};
use crate::review::{key_of, title_of};
use crate::rules::ItemOption;
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionRowView {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub tone: i64,
    pub anv: bool,
    /// `Monthly · USD 24.00`.
    pub interval: String,
    /// The latest price change while it is recent: `+€2.00 since Oct`.
    pub change: Option<String>,
    /// `Tomorrow`, `9 Dec`.
    pub next: String,
    pub amount: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionGroup {
    pub label: String,
    pub total: String,
    pub rows: Vec<SubscriptionRowView>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct EndedRowView {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub tone: i64,
    pub interval: String,
    /// `Ended 31 Aug 2026 · last charge 3 Aug 2026`.
    pub ended: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionsView {
    /// `8 active · €123.45 per month`.
    pub summary: String,
    pub groups: Vec<SubscriptionGroup>,
    pub ended: Vec<EndedRowView>,
    /// No subscription at all, before filtering.
    pub empty: bool,
    /// Every category in use, for the sheet's menu.
    pub categories: Vec<String>,
    /// The items an ANV subscription can count on.
    pub items: Vec<ItemOption>,
}

pub struct Format {
    pub number: NumberFormat,
    pub order: DateOrder,
}

impl Format {
    pub fn eur(&self, cents: i64) -> String {
        format_money(&Money::eur(cents), self.number)
    }

    pub fn money(&self, cents: i64, currency: &str) -> String {
        format_money(
            &Money {
                cents,
                currency: currency.into(),
            },
            self.number,
        )
    }

    /// `Tomorrow`, `Today`, `9 Dec`, or `9 Dec 2027` beyond a month.
    pub fn next(&self, next: NaiveDate, today: NaiveDate) -> String {
        match (next - today).num_days() {
            0 => "Today".into(),
            1 => "Tomorrow".into(),
            n if n > 31 => date::day_month_year(next, self.order),
            _ => date::day_month(next, self.order),
        }
    }
}

pub fn letter(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
}

/// `Monthly`, or `Monthly · USD 24.00` for a plan in another currency.
pub fn interval_text(plan: Option<&PlanRow>, format: &Format) -> String {
    match plan {
        Some(p) if p.currency != "EUR" => {
            format!(
                "{} · {}",
                p.interval.label(),
                format.money(p.cents, &p.currency)
            )
        }
        Some(p) => p.interval.label().into(),
        None => String::new(),
    }
}

/// The latest plan's price change within three months of today: `+€2.00 since Oct`.
fn recent_change(running: &Running, today: NaiveDate, format: &Format) -> Option<String> {
    let [.., before, last] = running.plans.as_slice() else {
        return None;
    };
    let start = date::parse(&last.start)?;
    if before.currency != last.currency || (today - start).num_days() > 92 || start > today {
        return None;
    }
    let difference = last.cents - before.cents;
    if difference == 0 {
        return None;
    }
    let sign = if difference > 0 { "+" } else { "\u{2212}" };
    Some(format!(
        "{sign}{} since {}",
        format.money(difference.abs(), &last.currency),
        date::short_month(start)
    ))
}

fn row_view(
    running: &Running,
    next: NaiveDate,
    charge: Option<i64>,
    today: NaiveDate,
    format: &Format,
) -> SubscriptionRowView {
    let sub = running.sub;
    SubscriptionRowView {
        id: sub.id.clone(),
        name: sub.name.clone(),
        letter: letter(&sub.name),
        tone: sub.tone,
        anv: sub.item_key.is_some(),
        interval: interval_text(running.current(today), format),
        change: recent_change(running, today, format),
        next: format.next(next, today),
        amount: charge.map_or_else(String::new, |c| format.eur(c)),
    }
}

fn ended_view(running: &Running, today: NaiveDate, format: &Format) -> EndedRowView {
    let sub = running.sub;
    let mut ended = sub
        .ended_on
        .as_deref()
        .and_then(date::parse)
        .map(|d| format!("Ended {}", date::day_month_year(d, format.order)))
        .unwrap_or_default();
    if let Some(last) = running
        .charges
        .last()
        .and_then(|c| date::parse(&c.value_date))
    {
        ended.push_str(&format!(
            " · last charge {}",
            date::day_month_year(last, format.order)
        ));
    }
    EndedRowView {
        id: sub.id.clone(),
        name: sub.name.clone(),
        letter: letter(&sub.name),
        tone: sub.tone,
        interval: interval_text(running.current(today), format),
        ended,
    }
}

const GROUPS: [(&str, i64, i64); 3] = [
    ("Next 7 days", i64::MIN, 7),
    ("Next 30 days", 8, 31),
    ("Later", 32, i64::MAX),
];

/// `query` keeps the subscriptions whose name or category contains it.
pub fn subscriptions(
    book: &Book,
    items: Vec<ItemOption>,
    query: &str,
    today: NaiveDate,
    format: &Format,
) -> SubscriptionsView {
    let needle = query.trim().to_lowercase();
    let shown = |s: &SubscriptionRow| {
        needle.is_empty()
            || s.name.to_lowercase().contains(&needle)
            || s.category.to_lowercase().contains(&needle)
    };
    let mut active: Vec<(NaiveDate, SubscriptionRowView, i64)> = Vec::new();
    let mut ended = Vec::new();
    let mut monthly = 0;
    let mut count = 0;
    for sub in &book.subscriptions {
        let running = book.running(sub);
        if sub.ended_on.is_some() {
            if shown(sub) {
                ended.push(ended_view(&running, today, format));
            }
            continue;
        }
        count += 1;
        monthly += running.monthly_cents(today).unwrap_or(0);
        if let (true, Some(next)) = (shown(sub), running.next_charge(today)) {
            let charge = running.charge_cents(today);
            active.push((
                next,
                row_view(&running, next, charge, today, format),
                charge.unwrap_or(0),
            ));
        }
    }
    active.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(&b.1.name)));
    let groups = GROUPS
        .iter()
        .filter_map(|(label, from, to)| {
            let rows: Vec<_> = active
                .iter()
                .filter(|(next, ..)| (from..=to).contains(&&(*next - today).num_days()))
                .collect();
            (!rows.is_empty()).then(|| SubscriptionGroup {
                label: (*label).into(),
                total: format.eur(rows.iter().map(|(.., c)| c).sum()),
                rows: rows.into_iter().map(|(_, row, _)| row.clone()).collect(),
            })
        })
        .collect();
    let mut categories: Vec<String> = book
        .subscriptions
        .iter()
        .map(|s| s.category.clone())
        .filter(|c| !c.is_empty())
        .collect();
    categories.sort();
    categories.dedup();
    SubscriptionsView {
        summary: format!("{count} active · {} per month", format.eur(monthly)),
        groups,
        ended,
        empty: book.subscriptions.is_empty(),
        categories,
        items,
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct MatchCount {
    /// `Matches 7 charges, the last on 9 Nov`.
    pub text: String,
    pub count: u32,
}

/// How many lines a form would match, the way the sheet shows it while typing.
pub fn match_count(
    lines: &[TransactionRow],
    form: &SubscriptionForm,
    format: &Format,
) -> MatchCount {
    let sub = SubscriptionRow {
        id: "draft".into(),
        name: form.name.clone(),
        tone: 0,
        category: String::new(),
        ended_on: None,
        reminder: Reminder::Off,
        item_key: None,
        match_text: form.match_text.clone(),
        exact_only: form.exact_only,
        created_at: 0,
    };
    let plan = PlanRow {
        id: "draft".into(),
        subscription_id: "draft".into(),
        name: String::new(),
        cents: crate::money::parse_cents(&form.amount).map_or(0, i64::abs),
        currency: form.currency.trim().to_uppercase(),
        interval: form.interval,
        start: "0000-01-01".into(),
        end: None,
        created_at: 0,
    };
    let matchers = Matchers::new(vec![sub], vec![plan]);
    let assignment = matchers.assign(lines, &[], |_| true);
    let matched: Vec<&TransactionRow> = lines
        .iter()
        .filter(|l| assignment.charge_of(&l.id).is_some())
        .collect();
    let count = matched.len() as u32;
    let last = matched
        .iter()
        .filter_map(|l| date::parse(&l.value_date))
        .max();
    let text = match (count, last) {
        (0, _) | (_, None) => "No charge in the statements matches yet".into(),
        (1, Some(last)) => format!(
            "Matches 1 charge, on {}",
            date::day_month_year(last, format.order)
        ),
        (n, Some(last)) => format!(
            "Matches {n} charges, the last on {}",
            date::day_month_year(last, format.order)
        ),
    };
    MatchCount { text, count }
}

/// The New sheet's starting values for a bank line.
/// A New Subscription form from a bank line; `term` (text of the line) becomes its match text.
pub fn prefill(
    conn: &rusqlite::Connection,
    line_id: &str,
    term: Option<&str>,
    format: &Format,
) -> AppResult<SubscriptionForm> {
    let line = statements::transaction_by_id(conn, line_id)?
        .ok_or_else(|| AppError::NotFound(format!("No line {line_id}")))?;
    let match_text = crate::review::rule_term(&line, term)?;
    // The first charge is the oldest line of the account with that text, not the one being reviewed.
    let start = statements::all_transactions(conn)?
        .into_iter()
        .filter(|t| {
            t.account_id == line.account_id
                && crate::rules::engine::line_text(t).contains(&match_text)
        })
        .map(|t| t.value_date)
        .min()
        .unwrap_or_else(|| line.value_date.clone());
    Ok(SubscriptionForm {
        match_text,
        start,
        ..prefill_from(&line, format)
    })
}

pub fn prefill_from(line: &TransactionRow, format: &Format) -> SubscriptionForm {
    let title = title_of(line);
    let (cents, currency) = match (&line.original_currency, line.original_cents) {
        (Some(code), Some(cents)) => (cents.abs(), code.clone()),
        _ => (line.cents.abs(), "EUR".to_string()),
    };
    SubscriptionForm {
        name: title.clone(),
        tone: (title.len() % 6) as i64,
        category: String::new(),
        active: true,
        ended_on: None,
        amount: format_plain(cents, format.number),
        currency,
        interval: Interval::Monthly,
        start: line.value_date.clone(),
        reminder: Reminder::Off,
        item_key: None,
        match_text: key_of(&title),
        exact_only: false,
    }
}
