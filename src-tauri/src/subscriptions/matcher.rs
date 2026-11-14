//! Which subscription a bank line is a charge of: its bank text contains the subscription's text and its
//! amount is within the tolerance of the plan in effect on its date. Lines that match the text but not the
//! amount wait in Review as "Plan changed?".

use rusqlite::Connection;

use super::store::{self, PlanRow, SubscriptionRow};
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

/// The charge's amount in the plan's currency, without sign; None when the line was paid in another one.
pub fn charged_in(t: &TransactionRow, currency: &str) -> Option<i64> {
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
    fn by_text<'a>(&'a self, t: &'a TransactionRow) -> impl Iterator<Item = &'a SubscriptionRow> {
        let text = if t.cents < 0 {
            line_text(t)
        } else {
            String::new()
        };
        self.subscriptions.iter().filter(move |s| {
            let token = s.match_text.trim().to_uppercase();
            !text.is_empty()
                && !token.is_empty()
                && text.contains(&token)
                && s.ended_on
                    .as_deref()
                    .is_none_or(|end| t.value_date.as_str() <= end)
        })
    }

    fn amount_fits(&self, sub: &SubscriptionRow, t: &TransactionRow) -> bool {
        let Some(percent) = sub.tolerance_percent else {
            return true;
        };
        plan_on(&self.plans_of(&sub.id), &t.value_date)
            .and_then(|plan| charged_in(t, &plan.currency).map(|c| (plan, (c - plan.cents).abs())))
            .is_some_and(|(plan, difference)| difference * 100 <= percent * plan.cents)
    }

    /// The first subscription the line is a charge of.
    pub fn matched<'a>(&'a self, t: &'a TransactionRow) -> Option<&'a SubscriptionRow> {
        self.by_text(t).find(|s| self.amount_fits(s, t))
    }

    pub fn decide(&self, t: &TransactionRow, now: i64) -> Option<DecisionRow> {
        self.matched(t).map(|s| DecisionRow {
            id: t.id.clone(),
            kind: Action::Subscription,
            item_key: None,
            subscription_id: Some(s.id.clone()),
            document_id: None,
            rule_id: Some(s.id.clone()),
            decided_at: now,
        })
    }

    /// The subscription whose text the line matches at an amount outside the tolerance, with its plan.
    pub fn plan_changed<'a>(
        &'a self,
        t: &'a TransactionRow,
    ) -> Option<(&'a SubscriptionRow, &'a PlanRow)> {
        if self.matched(t).is_some() {
            return None;
        }
        self.by_text(t)
            .find_map(|s| plan_on(&self.plans_of(&s.id), &t.value_date).map(|plan| (s, plan)))
    }
}
