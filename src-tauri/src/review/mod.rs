//! Review: the lines neither a rule nor the owner decided, one at a time. A hand decision can also become a
//! rule for the line's merchant token, which then decides the other lines it matches.

pub mod commands;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money, format_plain, format_signed};
use crate::rules::store::{self as rules_store, Action, DecisionRow};
use crate::rules::{self, ItemOption};
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};
use crate::subscriptions::{
    self,
    matcher::{self, Assignment, Matchers, charged_in},
};

/// A line waiting for a decision, as the Review card shows it.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ReviewLine {
    pub id: String,
    pub account_name: String,
    /// The account's place in the account list, for its color.
    pub account_index: u32,
    /// `Tue, 10 Nov`.
    pub date: String,
    pub title: String,
    pub amount: String,
    /// The booking text and its detail lines as printed.
    pub raw: String,
    /// The merchant token an "Always do this" rule matches, e.g. `SEITENWEISE`.
    pub key: String,
    /// What the owner decided for this key before.
    pub suggestion: Option<Action>,
    /// The item to assign if ANV: the one last used for this key.
    pub item_key: Option<String>,
    /// `last used for Buchhandlung Seitenweise`.
    pub item_why: Option<String>,
    /// A subscription's bank text at another amount than its plan's.
    pub plan_changed: Option<PlanChanged>,
}

/// What the "Plan changed?" card offers: the subscription and the line's amount for its new plan.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PlanChanged {
    pub subscription_id: String,
    pub name: String,
    /// `Lumen AI usually charges USD 24.00; this charge is USD 120.00.`
    pub text: String,
    /// The charge in the plan's currency as typed in a sheet, `120.00`.
    pub amount: String,
    pub currency: String,
    /// The line's date, `YYYY-MM-DD`, where a new plan would start.
    pub date: String,
}

/// An active subscription a line can be linked to.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct SubscriptionOption {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub tone: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ReviewQueue {
    /// Newest first.
    pub lines: Vec<ReviewLine>,
    pub items: Vec<ItemOption>,
    pub subscriptions: Vec<SubscriptionOption>,
}

/// The merchant, the counterparty or the booking text.
pub fn title_of(t: &TransactionRow) -> String {
    t.merchant
        .clone()
        .or_else(|| t.counterparty.clone())
        .unwrap_or_else(|| t.text.clone())
}

/// The significant token of a title: its first word, or the first two when the first is short.
pub fn key_of(title: &str) -> String {
    let words: Vec<String> = title
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphabetic))
        .map(str::to_uppercase)
        .collect();
    match words.as_slice() {
        [] => title.trim().to_uppercase(),
        [first, second, ..] if first.chars().count() < 4 => format!("{first} {second}"),
        [first, ..] => first.clone(),
    }
}

pub fn undecided_by_account(conn: &Connection) -> AppResult<HashMap<String, u32>> {
    let mut query = conn.prepare(&format!(
        "SELECT t.account_id, COUNT(*) FROM {} t LEFT JOIN {} d ON d.id = t.id \
         WHERE d.id IS NULL GROUP BY t.account_id",
        statements::TRANSACTIONS,
        rules_store::DECISIONS
    ))?;
    let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn raw_text(t: &TransactionRow) -> String {
    let extra: Vec<String> = serde_json::from_str(&t.extra).unwrap_or_default();
    std::iter::once(t.text.clone())
        .chain(extra)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The newest hand decision for one key.
struct Earlier {
    action: Action,
    item_key: Option<String>,
    /// The title of the line it was made for.
    title: String,
}

pub fn queue(db: &Db, format: NumberFormat, order: DateOrder) -> AppResult<ReviewQueue> {
    let lines = db.read(statements::all_transactions)?;
    let decisions = db.read(rules_store::decisions)?;
    let accounts = db.read(accounts::rows)?;
    let matchers = db.read(Matchers::load)?;
    let asked = matchers.assign(&lines, &matcher::taken(&decisions, false), |t| {
        !decisions.contains_key(&t.id)
    });
    let mut earlier: HashMap<String, Earlier> = HashMap::new();
    for line in &lines {
        if let Some(d) = decisions.get(&line.id).filter(|d| d.by_hand()) {
            let title = title_of(line);
            earlier.insert(
                key_of(&title),
                Earlier {
                    action: d.kind,
                    item_key: d.item_key.clone(),
                    title,
                },
            );
        }
    }
    let mut waiting: Vec<ReviewLine> = lines
        .iter()
        .filter(|t| !decisions.contains_key(&t.id))
        .map(|t| {
            let title = title_of(t);
            let key = key_of(&title);
            let before = earlier.get(&key);
            let (account_index, account_name) = accounts
                .iter()
                .enumerate()
                .find(|(_, a)| a.id == t.account_id)
                .map(|(i, a)| (i as u32, a.name.clone()))
                .unwrap_or_default();
            ReviewLine {
                id: t.id.clone(),
                account_name,
                account_index,
                date: date::parse(&t.value_date)
                    .map(|d| date::weekday_short(d, order))
                    .unwrap_or_default(),
                amount: format_signed(&Money::eur(t.cents), format),
                raw: raw_text(t),
                suggestion: before.map(|b| b.action),
                item_key: before.and_then(|b| b.item_key.clone()),
                item_why: before
                    .filter(|b| b.item_key.is_some())
                    .map(|b| format!("last used for {}", b.title)),
                plan_changed: plan_changed(&asked, t, format),
                title,
                key,
            }
        })
        .collect();
    waiting.reverse();
    Ok(ReviewQueue {
        lines: waiting,
        items: rules::items(db)?,
        subscriptions: subscription_options(db)?,
    })
}

fn plan_changed(
    asked: &Assignment,
    t: &TransactionRow,
    format: NumberFormat,
) -> Option<PlanChanged> {
    let (sub, plan) = asked.asks_about(&t.id)?;
    let money = |cents| {
        format_money(
            &Money {
                cents,
                currency: plan.currency.clone(),
            },
            format,
        )
    };
    let charged = charged_in(t, &plan.currency);
    let text = match charged {
        Some(cents) => format!(
            "{} usually charges {}; this charge is {}.",
            sub.name,
            money(plan.cents),
            money(cents)
        ),
        None => format!("{} usually charges {}.", sub.name, money(plan.cents)),
    };
    Some(PlanChanged {
        subscription_id: sub.id.clone(),
        name: sub.name.clone(),
        text,
        amount: format_plain(charged.unwrap_or(plan.cents), format),
        currency: plan.currency.clone(),
        date: t.value_date.clone(),
    })
}

fn subscription_options(db: &Db) -> AppResult<Vec<SubscriptionOption>> {
    Ok(db
        .read(subscriptions::store::subscriptions)?
        .into_iter()
        .filter(|s| s.ended_on.is_none())
        .map(|s| SubscriptionOption {
            letter: subscriptions::view::letter(&s.name),
            id: s.id,
            name: s.name,
            tone: s.tone,
        })
        .collect())
}

fn verb(action: Action) -> &'static str {
    match action {
        Action::Anv => "ANV",
        Action::Subscription => "Subscription",
        Action::Ignore => "Ignore",
    }
}

/// The subscription a Subscription decision links, which must exist; other actions link none.
fn checked_subscription(
    db: &Db,
    action: Action,
    subscription_id: Option<String>,
) -> AppResult<Option<String>> {
    if action != Action::Subscription {
        return Ok(None);
    }
    let id = subscription_id.unwrap_or_default();
    if !db
        .read(subscriptions::store::subscriptions)?
        .iter()
        .any(|s| s.id == id)
    {
        return Err(AppError::InvalidField {
            field: "review.subscription".into(),
            message: "Choose the subscription it belongs to".into(),
        });
    }
    Ok(Some(id))
}

/// The item an ANV decision assigns, which must be one of the year's; other actions assign none.
fn checked_item_key(
    db: &Db,
    action: Action,
    item_key: Option<String>,
) -> AppResult<Option<String>> {
    if action != Action::Anv {
        return Ok(None);
    }
    let key = item_key.unwrap_or_default();
    if !rules::items(db)?.iter().any(|i| i.key == key) {
        return Err(AppError::InvalidField {
            field: "review.item".into(),
            message: "Choose the item it counts as".into(),
        });
    }
    Ok(Some(key))
}

/// Decides a line by hand; with `always`, also adds a rule for its key and applies it to the other lines.
#[allow(clippy::too_many_arguments)]
pub fn decide(
    db: &Db,
    line_id: &str,
    action: Action,
    item_key: Option<String>,
    subscription_id: Option<String>,
    always: bool,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    decide_as(
        db,
        line_id,
        action,
        item_key,
        subscription_id,
        always,
        None,
        now,
        source,
    )
}

/// The shortest text a rule may match on; shorter terms match too much by accident.
const MIN_TERM: usize = 3;

/// The text an "Always" rule matches: the owner's `term`, which must appear in the line, or the title's key.
pub fn rule_term(line: &TransactionRow, term: Option<&str>) -> AppResult<String> {
    let Some(term) = term.map(|t| t.split_whitespace().collect::<Vec<_>>().join(" ")) else {
        return Ok(key_of(&title_of(line)));
    };
    let term = term.to_uppercase();
    if term.chars().count() < MIN_TERM {
        return Err(AppError::InvalidField {
            field: "review.term".into(),
            message: format!("Select at least {MIN_TERM} characters"),
        });
    }
    if !crate::rules::engine::line_text(line).contains(&term) {
        return Err(AppError::InvalidField {
            field: "review.term".into(),
            message: format!("“{term}” isn't in this line"),
        });
    }
    Ok(term)
}

/// `decide` with the text the rule matches on: `term` (from the line's text), else the title's key. A
/// subscription finds its charges by its own bank text, so a Subscription decision makes no rule.
#[allow(clippy::too_many_arguments)]
pub fn decide_as(
    db: &Db,
    line_id: &str,
    action: Action,
    item_key: Option<String>,
    subscription_id: Option<String>,
    always: bool,
    term: Option<&str>,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let line = db
        .read(|c| statements::transaction_by_id(c, line_id))?
        .ok_or_else(|| AppError::NotFound(format!("No line {line_id}")))?;
    let item_key = checked_item_key(db, action, item_key)?;
    let subscription_id = checked_subscription(db, action, subscription_id)?;
    let always = always && action != Action::Subscription;
    let term = if always {
        Some(rule_term(&line, term)?)
    } else {
        None
    };
    let decision = DecisionRow {
        id: line.id.clone(),
        kind: action,
        item_key: item_key.clone(),
        subscription_id,
        document_id: None,
        rule_id: None,
        decided_at: now,
    };
    let title = title_of(&line);
    let mut changes = vec![match db.read(rules_store::decisions)?.get(&line.id) {
        Some(before) => Change::update(rules_store::DECISIONS, before, &decision)?,
        None => Change::insert(rules_store::DECISIONS, &decision)?,
    }];
    let mut label = format!("{}: {title}", verb(action));
    if let Some(term) = term {
        let mut all = db.read(rules_store::rules)?;
        let rule = rules::appended(&all, &term, action, item_key, now);
        changes.push(Change::insert(rules_store::RULES, &rule)?);
        all.push(rule);
        changes.extend(db.read(|c| rules::apply::changes_except(c, &all, now, Some(&line.id)))?);
        label.push_str(" · rule created");
    }
    db.write(Op::new(label, changes), source)
}
