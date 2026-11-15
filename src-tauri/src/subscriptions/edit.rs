//! Writing subscriptions: New and Edit share one form; Change Plan ends the current plan the day before the new
//! one starts, so charges stay with the plan whose period they fall in. Every write re-matches the lines
//! against the subscriptions as they will be, in the same op.

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::matcher::Matchers;
use super::store::{self, Interval, PlanRow, Reminder, SubscriptionRow};
use crate::date;
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::rules::{self, apply, store as rules_store};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionForm {
    pub name: String,
    pub tone: i64,
    pub category: String,
    pub active: bool,
    /// The last day it runs, when not active; today when left empty.
    pub ended_on: Option<String>,
    pub amount: String,
    pub currency: String,
    pub interval: Interval,
    /// The first charge of a new subscription; ignored when editing.
    pub start: String,
    pub reminder: Reminder,
    pub item_key: Option<String>,
    pub match_text: String,
    /// Only the plan's exact amount is this subscription; otherwise another amount asks in Review.
    pub exact_only: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PlanForm {
    pub name: String,
    pub amount: String,
    pub currency: String,
    pub interval: Interval,
    pub start: String,
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidField {
        field: format!("subscription.{field}"),
        message: message.into(),
    }
}

fn amount(text: &str) -> AppResult<i64> {
    parse_cents(text)
        .map(i64::abs)
        .filter(|c| *c > 0)
        .ok_or_else(|| invalid("amount", "This isn't an amount"))
}

fn currency(text: &str) -> AppResult<String> {
    let code = text.trim().to_uppercase();
    if code.len() == 3 && code.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok(code)
    } else {
        Err(invalid("currency", "A three-letter code, such as EUR"))
    }
}

fn day(field: &str, text: &str) -> AppResult<NaiveDate> {
    date::parse(text).ok_or_else(|| invalid(field, "This isn't a date"))
}

/// The subscription row a form describes, keeping `id` and `created_at`.
fn row_of(
    db: &Db,
    form: &SubscriptionForm,
    id: String,
    created_at: i64,
    today: NaiveDate,
) -> AppResult<SubscriptionRow> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err(invalid("name", "Give it a name"));
    }
    if form.match_text.trim().is_empty() {
        return Err(invalid("match", "The bank text its charges contain"));
    }
    if let Some(key) = &form.item_key
        && !rules::items(db)?.iter().any(|i| &i.key == key)
    {
        return Err(invalid("item", "Choose one of the year's items"));
    }
    let ended_on = if form.active {
        None
    } else {
        Some(match form.ended_on.as_deref().map(str::trim) {
            Some(text) if !text.is_empty() => date::iso(day("endedOn", text)?),
            _ => date::iso(today),
        })
    };
    Ok(SubscriptionRow {
        id,
        name: name.to_string(),
        tone: form.tone.rem_euclid(6),
        category: form.category.trim().to_string(),
        ended_on,
        reminder: form.reminder,
        item_key: form.item_key.clone(),
        match_text: form.match_text.trim().to_string(),
        exact_only: form.exact_only,
        created_at,
    })
}

/// Writes `changes` with the decisions they bring: the lines matched against `subscriptions` and `plans` as
/// they will be.
fn write_matched(
    db: &Db,
    label: String,
    mut changes: Vec<Change>,
    subscriptions: Vec<SubscriptionRow>,
    plans: Vec<PlanRow>,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let matchers = Matchers::new(subscriptions, plans);
    changes.extend(db.read(|c| {
        let rules = rules_store::rules(c)?;
        apply::changes_with(c, &rules, &matchers, now, None)
    })?);
    db.write(Op::new(label, changes), source)
}

fn current_state(db: &Db) -> AppResult<(Vec<SubscriptionRow>, Vec<PlanRow>)> {
    db.read(|c| Ok((store::subscriptions(c)?, store::plans(c)?)))
}

fn existing(subscriptions: &[SubscriptionRow], id: &str) -> AppResult<SubscriptionRow> {
    subscriptions
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("No subscription {id}")))
}

fn replace(rows: &mut [SubscriptionRow], row: &SubscriptionRow) {
    if let Some(slot) = rows.iter_mut().find(|r| r.id == row.id) {
        *slot = row.clone();
    }
}

/// Adds a subscription with its first plan.
pub fn create(
    db: &Db,
    form: &SubscriptionForm,
    today: NaiveDate,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let sub = row_of(db, form, uuid::Uuid::new_v4().to_string(), now, today)?;
    let plan = PlanRow {
        id: uuid::Uuid::new_v4().to_string(),
        subscription_id: sub.id.clone(),
        name: "Standard".into(),
        cents: amount(&form.amount)?,
        currency: currency(&form.currency)?,
        interval: form.interval,
        start: date::iso(day("start", &form.start)?),
        end: None,
        created_at: now,
    };
    let (mut subscriptions, mut plans) = current_state(db)?;
    let changes = vec![
        Change::insert(store::SUBSCRIPTIONS, &sub)?,
        Change::insert(store::PLANS, &plan)?,
    ];
    let label = format!("New Subscription: {}", sub.name);
    subscriptions.push(sub);
    plans.push(plan);
    write_matched(db, label, changes, subscriptions, plans, now, source)
}

/// Fixes a subscription and its current plan in place, without plan history.
pub fn update(
    db: &Db,
    id: &str,
    form: &SubscriptionForm,
    today: NaiveDate,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let (mut subscriptions, mut plans) = current_state(db)?;
    let before = existing(&subscriptions, id)?;
    let after = row_of(db, form, before.id.clone(), before.created_at, today)?;
    let mut changes = vec![Change::update(store::SUBSCRIPTIONS, &before, &after)?];
    if let Some(plan) = plans.iter_mut().rev().find(|p| p.subscription_id == id) {
        let fixed = PlanRow {
            cents: amount(&form.amount)?,
            currency: currency(&form.currency)?,
            interval: form.interval,
            ..plan.clone()
        };
        if fixed != *plan {
            changes.push(Change::update(store::PLANS, plan, &fixed)?);
            *plan = fixed;
        }
    }
    replace(&mut subscriptions, &after);
    let label = format!("Edit Subscription: {}", after.name);
    write_matched(db, label, changes, subscriptions, plans, now, source)
}

/// Ends the current plan the day before `form.start` and starts the new one.
pub fn change_plan(
    db: &Db,
    id: &str,
    form: &PlanForm,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let (subscriptions, mut plans) = current_state(db)?;
    let sub = existing(&subscriptions, id)?;
    let start = day("start", &form.start)?;
    let name = form.name.trim();
    if name.is_empty() {
        return Err(invalid("planName", "Name the plan"));
    }
    let mut changes = Vec::new();
    if let Some(current) = plans.iter_mut().rev().find(|p| p.subscription_id == id) {
        if date::iso(start) <= current.start {
            return Err(invalid("start", "After the current plan's start"));
        }
        let ended = PlanRow {
            end: start.checked_sub_days(Days::new(1)).map(date::iso),
            ..current.clone()
        };
        changes.push(Change::update(store::PLANS, current, &ended)?);
        *current = ended;
    }
    let plan = PlanRow {
        id: uuid::Uuid::new_v4().to_string(),
        subscription_id: id.to_string(),
        name: name.to_string(),
        cents: amount(&form.amount)?,
        currency: currency(&form.currency)?,
        interval: form.interval,
        start: date::iso(start),
        end: None,
        created_at: now,
    };
    changes.push(Change::insert(store::PLANS, &plan)?);
    plans.push(plan);
    let label = format!("Change Plan: {}", sub.name);
    write_matched(db, label, changes, subscriptions, plans, now, source)
}

/// Ends a subscription on `ended_on` (None: resumes it); nothing is deleted.
pub fn set_ended(
    db: &Db,
    id: &str,
    ended_on: Option<&str>,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let (mut subscriptions, plans) = current_state(db)?;
    let before = existing(&subscriptions, id)?;
    let after = SubscriptionRow {
        ended_on: ended_on
            .map(|text| day("endedOn", text).map(date::iso))
            .transpose()?,
        ..before.clone()
    };
    let label = match ended_on {
        Some(_) => format!("End Subscription: {}", before.name),
        None => format!("Resume Subscription: {}", before.name),
    };
    let changes = vec![Change::update(store::SUBSCRIPTIONS, &before, &after)?];
    replace(&mut subscriptions, &after);
    write_matched(db, label, changes, subscriptions, plans, now, source)
}

/// Deletes a subscription and its plans; its charges go back to Review.
pub fn delete(db: &Db, id: &str, now: i64, source: Source) -> AppResult<Written> {
    let (mut subscriptions, mut plans) = current_state(db)?;
    let sub = existing(&subscriptions, id)?;
    // Its decisions go first, while the subscription still says whether they counted (db/frozen.rs).
    let mut changes = Vec::new();
    for decision in db.read(rules_store::decisions)?.values() {
        if decision.by_hand() && decision.subscription_id.as_deref() == Some(id) {
            changes.push(Change::delete(rules_store::DECISIONS, decision)?);
        }
    }
    for plan in plans.iter().filter(|p| p.subscription_id == id) {
        changes.push(Change::delete(store::PLANS, plan)?);
    }
    changes.push(Change::delete(store::SUBSCRIPTIONS, &sub)?);
    subscriptions.retain(|s| s.id != id);
    plans.retain(|p| p.subscription_id != id);
    let label = format!("Delete Subscription: {}", sub.name);
    write_matched(db, label, changes, subscriptions, plans, now, source)
}
