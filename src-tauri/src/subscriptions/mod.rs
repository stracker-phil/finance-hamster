//! Subscriptions: services with a history of plans, matched to their charges in the statements and always
//! shown as of today.

pub mod commands;
pub mod detail;
pub mod edit;
pub mod matcher;
pub mod schedule;
pub mod store;
pub mod view;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rusqlite::Connection;

use crate::error::AppResult;
use crate::rules::store::{self as rules_store, Action};
use crate::statements::store::{self as statements, TransactionRow};
use schedule::Running;
use store::{PlanRow, SubscriptionRow};

/// Every subscription with its plans and the lines decided as its charges.
pub struct Book {
    pub subscriptions: Vec<SubscriptionRow>,
    pub plans: Vec<PlanRow>,
    charges: HashMap<String, Vec<TransactionRow>>,
}

impl Book {
    pub fn load(conn: &Connection) -> AppResult<Book> {
        let decisions = rules_store::decisions(conn)?;
        let mut charges: HashMap<String, Vec<TransactionRow>> = HashMap::new();
        for line in statements::all_transactions(conn)? {
            let linked = decisions
                .get(&line.id)
                .filter(|d| d.kind == Action::Subscription)
                .and_then(|d| d.subscription_id.clone());
            if let Some(id) = linked {
                charges.entry(id).or_default().push(line);
            }
        }
        for lines in charges.values_mut() {
            lines.sort_by(|a, b| a.value_date.cmp(&b.value_date));
        }
        Ok(Book {
            subscriptions: store::subscriptions(conn)?,
            plans: store::plans(conn)?,
            charges,
        })
    }

    pub fn running<'a>(&'a self, sub: &'a SubscriptionRow) -> Running<'a> {
        Running {
            sub,
            plans: self
                .plans
                .iter()
                .filter(|p| p.subscription_id == sub.id)
                .collect(),
            charges: self
                .charges
                .get(&sub.id)
                .map(|lines| lines.iter().collect())
                .unwrap_or_default(),
        }
    }

    pub fn find(&self, id: &str) -> Option<&SubscriptionRow> {
        self.subscriptions.iter().find(|s| s.id == id)
    }
}
