//! Brings the rule decisions of every line in line with a set of rules; hand decisions are never touched.

use rusqlite::Connection;

use super::engine::{self, OwnAccount};
use super::store::{self, DecisionRow, RuleRow};
use crate::accounts;
use crate::db::Change;
use crate::error::AppResult;
use crate::statements::store as statements;

pub fn own_accounts(conn: &Connection) -> AppResult<Vec<OwnAccount>> {
    Ok(accounts::rows(conn)?
        .into_iter()
        .filter(|a| a.own)
        .map(|a| (a.id, a.iban))
        .collect())
}

/// One change per line whose decision `rules` change; hand decisions stay.
pub fn changes(conn: &Connection, rules: &[RuleRow], now: i64) -> AppResult<Vec<Change>> {
    let own = own_accounts(conn)?;
    let existing = store::decisions(conn)?;
    let mut changes = Vec::new();
    for line in statements::all_transactions(conn)? {
        let current = existing.get(&line.id);
        if current.is_some_and(DecisionRow::by_hand) {
            continue;
        }
        let wanted = engine::decide(rules, &line, &own, now);
        if let Some(change) = change_for(current, wanted)? {
            changes.push(change);
        }
    }
    Ok(changes)
}

fn change_for(
    current: Option<&DecisionRow>,
    wanted: Option<DecisionRow>,
) -> AppResult<Option<Change>> {
    Ok(match (current, wanted) {
        (None, None) => None,
        (None, Some(new)) => Some(Change::insert(store::DECISIONS, &new)?),
        (Some(old), None) => Some(Change::delete(store::DECISIONS, old)?),
        (Some(old), Some(new)) if old.same_as(&new) => None,
        (Some(old), Some(new)) => Some(Change::update(store::DECISIONS, old, &new)?),
    })
}
