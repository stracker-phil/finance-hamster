//! A subscription's detail panel: amount, next charge, the year's plan timeline, its plans, ANV, reminder,
//! the rule that finds it and the charges found in the statements.

use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use super::Book;
use super::edit::SubscriptionForm;
use super::matcher::plan_on;
use super::store::{Interval, PlanRow, SubscriptionRow};
use super::view::{Format, interval_text, letter};
use crate::accounts::AccountRow;
use crate::date;
use crate::error::{AppError, AppResult};
use crate::i18n::Text;
use crate::money::format_plain;
use crate::rules::ItemOption;
use crate::tr;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct TimelineMonth {
    /// `J`.
    pub label: String,
    /// The plan's place in `plans`; None before the first or after the end.
    pub plan: Option<u32>,
    /// `March: Pro`.
    pub tip: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PlanView {
    pub name: String,
    /// `USD 24.00 monthly`.
    pub price: String,
    /// `since 1 Apr 2026`, `1 Jan – 31 Mar 2026`.
    pub when: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ChargeView {
    pub id: String,
    /// `Mon, 9 Nov 2026`.
    pub date: String,
    pub account: String,
    pub amount: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionDetail {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub tone: i64,
    pub interval: String,
    /// `€22.48 / month`.
    pub amount: String,
    /// `Mon, 9 Dec · in 26 days`; None once ended.
    pub next: Option<String>,
    /// `Ended 31 Aug 2026`.
    pub ended: Option<String>,
    pub year: i32,
    pub timeline: Vec<TimelineMonth>,
    pub plans: Vec<PlanView>,
    /// `Yes · Computer, phone & software` or `No`.
    pub anv: String,
    pub reminder: String,
    /// `LUMEN AI · another amount asks`.
    pub rule: String,
    pub charges: Vec<ChargeView>,
    /// What Edit starts from.
    pub form: SubscriptionForm,
    /// `Pro · monthly · USD 24.00 since 1 Apr 2026`, for the Change Plan sheet.
    pub current_plan: String,
}

fn relative(days: i64) -> String {
    match days {
        0 => tr!("today", "heute"),
        1 => tr!("tomorrow", "morgen"),
        -1 => tr!("yesterday", "gestern"),
        n if n < 0 => {
            let ago = -n;
            tr!("{ago} days ago", "vor {ago} Tagen")
        }
        n => tr!("in {n} days", "in {n} Tagen"),
    }
}

fn price(plan: &PlanRow, format: &Format) -> String {
    format!(
        "{} {}",
        format.money(plan.cents, &plan.currency),
        plan.interval.label().to_lowercase()
    )
}

fn when(plan: &PlanRow, format: &Format) -> String {
    let start = date::parse(&plan.start);
    match (start, plan.end.as_deref().and_then(date::parse)) {
        (Some(s), None) => {
            let day = date::day_month_year(s, format.order);
            tr!("since {day}", "seit {day}")
        }
        (Some(s), Some(e)) => format!(
            "{} – {}",
            date::day_month(s, format.order),
            date::day_month_year(e, format.order)
        ),
        _ => String::new(),
    }
}

fn timeline(plans: &[&PlanRow], sub: &SubscriptionRow, year: i32) -> Vec<TimelineMonth> {
    (1..=12)
        .map(|month| {
            let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_default();
            let mid = date::iso(first.with_day(15).unwrap_or(first));
            let ended = sub
                .ended_on
                .as_deref()
                .is_some_and(|end| end < date::iso(first).as_str());
            let plan = plans.iter().position(|p| p.covers(&mid)).filter(|_| !ended);
            let name = date::month_name(month);
            TimelineMonth {
                label: name[..1].to_string(),
                plan: plan.map(|i| i as u32),
                tip: match plan {
                    Some(i) => format!("{name}: {}", plans[i].name),
                    None => tr!("{name}: no plan", "{name}: kein Tarif"),
                },
            }
        })
        .collect()
}

fn form_of(sub: &SubscriptionRow, plan: Option<&PlanRow>, format: &Format) -> SubscriptionForm {
    SubscriptionForm {
        name: sub.name.clone(),
        tone: sub.tone,
        category: sub.category.clone(),
        active: sub.ended_on.is_none(),
        ended_on: sub.ended_on.clone(),
        amount: plan.map_or_else(String::new, |p| format_plain(p.cents, format.number)),
        currency: plan.map_or_else(|| "EUR".into(), |p| p.currency.clone()),
        interval: plan.map_or(Interval::Monthly, |p| p.interval),
        start: plan.map_or_else(String::new, |p| p.start.clone()),
        reminder: sub.reminder,
        item_key: sub.item_key.clone(),
        match_text: sub.match_text.clone(),
        exact_only: sub.exact_only,
    }
}

pub fn detail(
    book: &Book,
    id: &str,
    accounts: &[AccountRow],
    items: &[ItemOption],
    today: NaiveDate,
    format: &Format,
) -> AppResult<SubscriptionDetail> {
    let sub = book
        .find(id)
        .ok_or_else(|| AppError::NotFound(tr!("No subscription {id}", "Kein Abo {id}")))?;
    let running = book.running(sub);
    let current = running.current(today);
    let next = running.next_charge(today).map(|n| {
        format!(
            "{} · {}",
            date::weekday_short(n, format.order),
            relative((n - today).num_days())
        )
    });
    let per = current.map_or("", |p| {
        match p.interval {
            Interval::Monthly => Text::new(" / month", " / Monat"),
            Interval::Quarterly => Text::new(" / quarter", " / Quartal"),
            Interval::Yearly => Text::new(" / year", " / Jahr"),
        }
        .get()
    });
    let amount = running
        .charge_cents(today)
        .map_or_else(String::new, |c| format!("{}{per}", format.eur(c)));
    let anv = match &sub.item_key {
        Some(key) => items.iter().find(|i| &i.key == key).map_or_else(
            || tr!("Yes · {key}", "Ja · {key}"),
            |i| tr!("Yes · {}", "Ja · {}", i.label),
        ),
        None => tr!("No", "Nein"),
    };
    let amount_rule = if sub.exact_only {
        tr!("exact amount only", "nur der genaue Betrag")
    } else {
        tr!("another amount asks", "ein anderer Betrag fragt nach")
    };
    let charges = running
        .charges
        .iter()
        .rev()
        .map(|c| ChargeView {
            id: c.id.clone(),
            date: date::parse(&c.value_date)
                .map_or_else(String::new, |d| date::weekday_short(d, format.order)),
            account: accounts
                .iter()
                .find(|a| a.id == c.account_id)
                .map_or_else(String::new, |a| a.name.clone()),
            amount: format.eur(c.cents.abs()),
        })
        .collect();
    let current_plan = current.map_or_else(String::new, |p| {
        format!("{} · {} · {}", p.name, price(p, format), when(p, format))
    });
    let year = today.year();
    Ok(SubscriptionDetail {
        id: sub.id.clone(),
        name: sub.name.clone(),
        letter: letter(&sub.name),
        tone: sub.tone,
        interval: interval_text(current, format),
        amount,
        next,
        ended: sub.ended_on.as_deref().and_then(date::parse).map(|d| {
            let day = date::day_month_year(d, format.order);
            tr!("Ended {day}", "Beendet am {day}")
        }),
        year,
        timeline: timeline(&running.plans, sub, year),
        plans: running
            .plans
            .iter()
            .map(|p| PlanView {
                name: p.name.clone(),
                price: price(p, format),
                when: when(p, format),
            })
            .collect(),
        anv,
        reminder: sub.reminder.label().into(),
        rule: format!("{} · {amount_rule}", sub.match_text.to_uppercase()),
        charges,
        form: form_of(sub, plan_on(&running.plans, &date::iso(today)), format),
        current_plan,
    })
}
