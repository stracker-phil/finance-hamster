//! Brings the rule decisions of every line in line with a set of rules, then with the subscriptions' own
//! matching for lines no rule decides; hand decisions and lines in closed tax years are never touched.

use rusqlite::Connection;

use super::engine::{self, OwnAccount};
use super::store::{self, DecisionRow, RuleRow};
use crate::accounts;
use crate::db::Change;
use crate::error::AppResult;
use crate::statements::store::{self as statements, TransactionRow};
use crate::subscriptions::matcher::{self, Matchers};
use crate::taxyears;

pub fn own_accounts(conn: &Connection) -> AppResult<Vec<OwnAccount>> {
    Ok(own_of(accounts::rows(conn)?))
}

/// The own accounts among `rows`.
pub fn own_of(rows: Vec<accounts::AccountRow>) -> Vec<OwnAccount> {
    rows.into_iter()
        .filter(|a| a.own)
        .map(|a| OwnAccount {
            id: a.id,
            iban: a.iban,
            statement_name: a.statement_name,
        })
        .collect()
}

/// One change per line whose decision `rules` change; hand decisions stay.
pub fn changes(conn: &Connection, rules: &[RuleRow], now: i64) -> AppResult<Vec<Change>> {
    changes_except(conn, rules, now, None)
}

/// Like `changes`, leaving out the line `skipped`, which is being decided by hand in the same op.
pub fn changes_except(
    conn: &Connection,
    rules: &[RuleRow],
    now: i64,
    skipped: Option<&str>,
) -> AppResult<Vec<Change>> {
    changes_with(conn, rules, &Matchers::load(conn)?, now, skipped)
}

/// Like `changes_except`, matching `subscriptions` as they will be after the op that carries the changes.
pub fn changes_with(
    conn: &Connection,
    rules: &[RuleRow],
    subscriptions: &Matchers,
    now: i64,
    skipped: Option<&str>,
) -> AppResult<Vec<Change>> {
    changes_for(
        conn,
        rules,
        subscriptions,
        &own_accounts(conn)?,
        now,
        skipped,
    )
}

/// The rules' changes with the own accounts as they will be after the op that carries them.
pub fn changes_with_own(conn: &Connection, own: &[OwnAccount], now: i64) -> AppResult<Vec<Change>> {
    let rules = store::rules(conn)?;
    changes_for(conn, &rules, &Matchers::load(conn)?, own, now, None)
}

fn changes_for(
    conn: &Connection,
    rules: &[RuleRow],
    subscriptions: &Matchers,
    own: &[OwnAccount],
    now: i64,
    skipped: Option<&str>,
) -> AppResult<Vec<Change>> {
    let existing = store::decisions(conn)?;
    let lines = statements::all_transactions(conn)?;
    let closed = taxyears::closed_years(conn)?;
    let recomputed = |line: &TransactionRow| {
        !existing.get(&line.id).is_some_and(DecisionRow::by_hand)
            && skipped != Some(line.id.as_str())
            && !taxyears::in_closed(&closed, &line.value_date)
    };
    let charges = subscriptions.assign(&lines, &matcher::taken(&existing, true), |line| {
        recomputed(line) && engine::decide(rules, line, own, now).is_none()
    });
    let mut changes = Vec::new();
    for line in &lines {
        if !recomputed(line) {
            continue;
        }
        let current = existing.get(&line.id);
        let wanted = engine::decide(rules, line, own, now).or_else(|| charges.decide(line, now));
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
