//! Which subscription a bank line is a charge of. Its bank text contains the subscription's text, and a
//! subscription takes at most one charge per period: a line too close to a charge it already has is not its.
//! A charge at the plan's amount is booked; at another amount it waits in Review ("Is this …?") unless the
//! subscription takes only its exact amount.

use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::Connection;

use super::store::{self, Interval, PlanRow, SubscriptionRow};
use crate::date;
use crate::error::AppResult;
use crate::rules::engine::line_text;
use crate::rules::store::{Action, DecisionRow};
use crate::statements::store::TransactionRow;

pub struct Matchers {
    subscriptions: Vec<SubscriptionRow>,
    plans: Vec<PlanRow>,
}

/// The plan a charge on `day` belongs to: the one covering it, else the last one started before it, else the
/// first.
pub fn plan_on<'a>(plans: &[&'a PlanRow], day: &str) -> Option<&'a PlanRow> {
    plans
        .iter()
        .find(|p| p.covers(day))
        .or_else(|| plans.iter().rev().find(|p| p.start.as_str() <= day))
        .or_else(|| plans.first())
        .copied()
}

/// The charge's amount in the plan's currency, without sign; None for a credit (a refund is no charge) or a
/// line paid in another currency.
pub fn charged_in(t: &TransactionRow, currency: &str) -> Option<i64> {
    if t.cents >= 0 {
        return None;
    }
    match t.original_currency.as_deref() {
        None if currency == "EUR" => Some(t.cents.abs()),
        Some(code) if code == currency => t.original_cents.map(i64::abs),
        _ => None,
    }
}

impl Matchers {
    pub fn new(subscriptions: Vec<SubscriptionRow>, plans: Vec<PlanRow>) -> Matchers {
        Matchers {
            subscriptions,
            plans,
        }
    }

    pub fn load(conn: &Connection) -> AppResult<Matchers> {
        Ok(Matchers::new(
            store::subscriptions(conn)?,
            store::plans(conn)?,
        ))
    }

    fn plans_of(&self, id: &str) -> Vec<&PlanRow> {
        self.plans
            .iter()
            .filter(|p| p.subscription_id == id)
            .collect()
    }

    /// Only charges (outgoing lines) on or before a subscription's last day.
    fn by_text(&self, t: &TransactionRow) -> Vec<&SubscriptionRow> {
        let text = if t.cents < 0 {
            line_text(t)
        } else {
            String::new()
        };
        self.subscriptions
            .iter()
            .filter(|s| {
                let token = s.match_text.trim().to_uppercase();
                !text.is_empty()
                    && !token.is_empty()
                    && text.contains(&token)
                    && s.ended_on
                        .as_deref()
                        .is_none_or(|end| t.value_date.as_str() <= end)
            })
            .collect()
    }

    /// Walks `lines` oldest first and settles which subscription each `candidate` line belongs to. `taken` are
    /// the charges already linked, as (subscription id, line id): they fill their subscription's period.
    pub fn assign<'a>(
        &'a self,
        lines: &[TransactionRow],
        taken: &[(String, String)],
        candidate: impl Fn(&TransactionRow) -> bool,
    ) -> Assignment<'a> {
        let day_of: HashMap<&str, NaiveDate> = lines
            .iter()
            .filter_map(|t| date::parse(&t.value_date).map(|d| (t.id.as_str(), d)))
            .collect();
        let mut charged: HashMap<String, Vec<NaiveDate>> = HashMap::new();
        for (sub, line) in taken {
            if let Some(day) = day_of.get(line.as_str()) {
                charged.entry(sub.clone()).or_default().push(*day);
            }
        }
        let mut lines_of = HashMap::new();
        // Exact amounts claim their periods first, so an odd amount next to the real charge can't take its slot.
        for exact_pass in [true, false] {
            let open: Vec<&TransactionRow> = lines
                .iter()
                .filter(|t| candidate(t) && !lines_of.contains_key(&t.id))
                .collect();
            for t in open {
                let Some(day) = date::parse(&t.value_date) else {
                    continue;
                };
                let found = self.by_text(t).into_iter().find_map(|sub| {
                    let plan = plan_on(&self.plans_of(&sub.id), &t.value_date)?;
                    let gap = i64::from(min_days_between(plan.interval));
                    let free = charged
                        .get(&sub.id)
                        .is_none_or(|days| days.iter().all(|d| (*d - day).num_days().abs() >= gap));
                    let exact = charged_in(t, &plan.currency) == Some(plan.cents);
                    match (free, exact, exact_pass) {
                        (true, true, true) => Some(Outcome::Charge(sub)),
                        (true, false, false) if !sub.exact_only => Some(Outcome::Ask(sub, plan)),
                        _ => None,
                    }
                });
                if let Some(outcome) = found {
                    charged
                        .entry(outcome.subscription().id.clone())
                        .or_default()
                        .push(day);
                    lines_of.insert(t.id.clone(), outcome);
                }
            }
        }
        Assignment { lines: lines_of }
    }
}

/// How close two charges of one subscription may be; any closer and the second isn't its. Leaves room for a
/// charge moved by a weekend or a short month.
fn min_days_between(interval: Interval) -> u32 {
    match interval {
        Interval::Monthly => 20,
        Interval::Quarterly => 60,
        Interval::Yearly => 240,
    }
}

/// The subscription charges already linked in `decisions`, as `assign` takes them; `hand_only` keeps the
/// owner's links and leaves out the matches about to be recomputed.
pub fn taken(decisions: &HashMap<String, DecisionRow>, hand_only: bool) -> Vec<(String, String)> {
    decisions
        .values()
        .filter(|d| d.kind == Action::Subscription && (!hand_only || d.by_hand()))
        .filter_map(|d| d.subscription_id.clone().map(|sub| (sub, d.id.clone())))
        .collect()
}

pub enum Outcome<'a> {
    /// At the plan's amount: booked.
    Charge(&'a SubscriptionRow),
    /// At another amount: Review asks whether it is this subscription.
    Ask(&'a SubscriptionRow, &'a PlanRow),
}

impl Outcome<'_> {
    fn subscription(&self) -> &SubscriptionRow {
        match self {
            Outcome::Charge(sub) | Outcome::Ask(sub, _) => sub,
        }
    }
}

pub struct Assignment<'a> {
    lines: HashMap<String, Outcome<'a>>,
}

impl<'a> Assignment<'a> {
    /// The subscription the line is a charge of.
    pub fn charge_of(&self, line_id: &str) -> Option<&'a SubscriptionRow> {
        match self.lines.get(line_id)? {
            Outcome::Charge(sub) => Some(*sub),
            Outcome::Ask(..) => None,
        }
    }

    /// The subscription Review asks about for the line, with the plan its amount differs from.
    pub fn asks_about(&self, line_id: &str) -> Option<(&'a SubscriptionRow, &'a PlanRow)> {
        match self.lines.get(line_id)? {
            Outcome::Ask(sub, plan) => Some((*sub, *plan)),
            Outcome::Charge(_) => None,
        }
    }

    pub fn decide(&self, t: &TransactionRow, now: i64) -> Option<DecisionRow> {
        self.charge_of(&t.id).map(|s| DecisionRow {
            id: t.id.clone(),
            kind: Action::Subscription,
            item_key: None,
            subscription_id: Some(s.id.clone()),
            document_id: None,
            rule_id: Some(s.id.clone()),
            decided_at: now,
        })
    }
}
