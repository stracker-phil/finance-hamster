//! Rules sort imported lines: a line a rule matches is ignored, assigned an item or linked to a subscription
//! without asking. They run on import and whenever a rule changes; a hand decision always wins.

pub mod apply;
pub mod commands;
pub mod engine;
pub mod preview;
pub mod store;
pub mod view;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod view_tests;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::taxrules::model::Ruleset;
use crate::taxyears::{self, YearState};
use store::{Action, Builtin, RuleRow};

/// A rule as the Rule sheet edits it; amounts as typed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RuleForm {
    /// Any of these in the bank text matches, ignoring case.
    pub contains: Vec<String>,
    pub amount: Option<String>,
    pub tolerance: Option<String>,
    pub account_id: Option<String>,
    pub own_counterparty: bool,
    pub action: Action,
    pub item_key: Option<String>,
}

/// An item the ANV action can assign.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ItemOption {
    pub key: String,
    /// `KZ 169 · Digitale Arbeitsmittel`.
    pub label: String,
    pub kz: String,
}

pub const TRANSFERS_ID: &str = "builtin-transfers";
const BANK_FEES_ID: &str = "builtin-bank-fees";
const BANK_FEE_TOKENS: [&str; 2] = ["Kontoführung", "Debitkarte"];

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidField {
        field: format!("rule.{field}"),
        message: message.into(),
    }
}

/// The rules the newest open tax year applies, or the newest year's when every year is closed.
pub fn ruleset(db: &Db) -> AppResult<Option<Ruleset>> {
    let years = db.read(taxyears::rows)?;
    let Some(year) = years
        .iter()
        .find(|y| y.state == YearState::Open)
        .or(years.first())
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&year.rules)?))
}

/// The rules tax year `year` applies; the newest open year's while `year` has none yet.
pub fn ruleset_for_year(db: &Db, year: i32) -> AppResult<Option<Ruleset>> {
    match db.read(|c| taxyears::row(c, year))? {
        Some(row) => Ok(Some(serde_json::from_str(&row.rules)?)),
        None => ruleset(db),
    }
}

/// The visible items of a ruleset, as pickers show them.
pub fn item_options(ruleset: &Ruleset) -> Vec<ItemOption> {
    ruleset
        .items
        .iter()
        .filter(|i| i.visible)
        .map(|i| ItemOption {
            label: format!("KZ {} · {}", i.kz, i.name),
            key: i.key.clone(),
            kz: i.kz.clone(),
        })
        .collect()
}

/// The visible items of the newest open tax year's rules.
pub fn items(db: &Db) -> AppResult<Vec<ItemOption>> {
    Ok(ruleset(db)?.map(|r| item_options(&r)).unwrap_or_default())
}

/// The amount typed into `field`, without sign; None when nothing was typed.
fn parse_amount(field: &str, text: &Option<String>) -> AppResult<Option<i64>> {
    text.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            parse_cents(t)
                .map(i64::abs)
                .ok_or_else(|| invalid(field, "This isn't an amount"))
        })
        .transpose()
}

pub(crate) fn encode_tokens(tokens: &[impl AsRef<str>]) -> String {
    let tokens: Vec<&str> = tokens.iter().map(AsRef::as_ref).collect();
    serde_json::Value::from(tokens).to_string()
}

/// A rule with no conditions that ignores; callers set what it matches and does.
pub(crate) fn blank(id: String, position: i64, now: i64) -> RuleRow {
    RuleRow {
        id,
        position,
        builtin: None,
        contains: encode_tokens(&[] as &[&str]),
        amount_cents: None,
        tolerance_cents: 0,
        account_id: None,
        own_counterparty: false,
        action: Action::Ignore,
        item_key: None,
        subscription_id: None,
        document_id: None,
        created_at: now,
    }
}

struct Conditions {
    tokens: Vec<String>,
    amount_cents: Option<i64>,
    tolerance_cents: i64,
}

fn conditions(db: &Db, form: &RuleForm) -> AppResult<Conditions> {
    let tokens: Vec<String> = form
        .contains
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let amount_cents = parse_amount("amount", &form.amount)?;
    let tolerance_cents = parse_amount("tolerance", &form.tolerance)?.unwrap_or(0);
    if tolerance_cents > 0 && amount_cents.is_none() {
        return Err(invalid("tolerance", "A tolerance needs an amount"));
    }
    if let Some(account) = &form.account_id
        && !db
            .read(crate::accounts::rows)?
            .iter()
            .any(|a| &a.id == account)
    {
        return Err(invalid("account", "This account doesn't exist"));
    }
    if tokens.is_empty()
        && amount_cents.is_none()
        && form.account_id.is_none()
        && !form.own_counterparty
    {
        return Err(invalid("conditions", "Add at least one condition"));
    }
    match form.action {
        Action::Anv => {
            let key = form.item_key.as_deref().unwrap_or_default();
            if !items(db)?.iter().any(|i| i.key == key) {
                return Err(invalid("item", "Choose an item"));
            }
        }
        Action::Subscription => return Err(invalid("action", "Choose a subscription")),
        Action::Ignore => {}
    }
    Ok(Conditions {
        tokens,
        amount_cents,
        tolerance_cents,
    })
}

/// Writes the rule change together with what it changes in the lines' decisions.
fn write_with_rerun(
    db: &Db,
    label: String,
    rule_change: Change,
    rules: &[RuleRow],
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let mut changes = vec![rule_change];
    changes.extend(db.read(|c| apply::changes(c, rules, now))?);
    db.write(Op::new(label, changes), source)
}

/// A rule matching `token` in the bank text, after every other rule.
pub fn appended(
    rules: &[RuleRow],
    token: &str,
    action: Action,
    item_key: Option<String>,
    now: i64,
) -> RuleRow {
    let position = rules.iter().map(|r| r.position).max().unwrap_or(-1) + 1;
    RuleRow {
        contains: encode_tokens(&[token]),
        action,
        item_key,
        ..blank(uuid::Uuid::new_v4().to_string(), position, now)
    }
}

/// Adds a rule at the end, or changes the rule `id`, and applies the rules again.
pub fn save(
    db: &Db,
    id: Option<&str>,
    form: &RuleForm,
    now: i64,
    source: Source,
) -> AppResult<Written> {
    let c = conditions(db, form)?;
    let mut rules = db.read(store::rules)?;
    let fields = |row: RuleRow| RuleRow {
        contains: encode_tokens(&c.tokens),
        amount_cents: c.amount_cents,
        tolerance_cents: c.tolerance_cents,
        account_id: form.account_id.clone(),
        own_counterparty: form.own_counterparty,
        action: form.action,
        item_key: (form.action == Action::Anv)
            .then(|| form.item_key.clone())
            .flatten(),
        ..row
    };
    match id {
        Some(id) => {
            let index = rules
                .iter()
                .position(|r| r.id == id)
                .ok_or_else(|| AppError::NotFound(format!("No rule {id}")))?;
            let after = fields(rules[index].clone());
            let change = Change::update(store::RULES, &rules[index], &after)?;
            rules[index] = after;
            write_with_rerun(db, "Edit Rule".into(), change, &rules, now, source)
        }
        None => {
            let position = rules.iter().map(|r| r.position).max().unwrap_or(-1) + 1;
            let row = fields(blank(uuid::Uuid::new_v4().to_string(), position, now));
            let change = Change::insert(store::RULES, &row)?;
            rules.push(row);
            write_with_rerun(db, "Add Rule".into(), change, &rules, now, source)
        }
    }
}

/// Deletes a rule; the lines it decided get the next matching rule's decision, or go back to Review.
pub fn delete(db: &Db, id: &str, now: i64, source: Source) -> AppResult<Written> {
    let mut rules = db.read(store::rules)?;
    let index = rules
        .iter()
        .position(|r| r.id == id)
        .ok_or_else(|| AppError::NotFound(format!("No rule {id}")))?;
    if rules[index].builtin.is_some() {
        return Err(AppError::InvalidInput(
            "A built-in rule can't be deleted".into(),
        ));
    }
    let removed = rules.remove(index);
    let change = Change::delete(store::RULES, &removed)?;
    write_with_rerun(db, "Delete Rule".into(), change, &rules, now, source)
}

/// Applies the rules to every line again, with the number of lines whose decision changed; None when
/// nothing changes.
pub fn rerun(db: &Db, now: i64, source: Source) -> AppResult<Option<(Written, usize)>> {
    let rules = db.read(store::rules)?;
    let changes = db.read(|c| apply::changes(c, &rules, now))?;
    if changes.is_empty() {
        return Ok(None);
    }
    let lines = changes.len();
    let written = db.write(Op::new("Re-run Rules", changes), source)?;
    Ok(Some((written, lines)))
}

fn builtin(id: &str, builtin: Builtin, position: i64, now: i64) -> RuleRow {
    let tokens: &[&str] = match builtin {
        Builtin::Transfers => &[],
        Builtin::BankFees => &BANK_FEE_TOKENS,
    };
    RuleRow {
        builtin: Some(builtin),
        contains: encode_tokens(tokens),
        own_counterparty: builtin == Builtin::Transfers,
        ..blank(id.into(), position, now)
    }
}

/// The rules that come with the app, ahead of the owner's: transfers between own accounts and bank fees are
/// ignored.
pub fn builtin_rules(now: i64) -> Vec<RuleRow> {
    vec![
        builtin(TRANSFERS_ID, Builtin::Transfers, -2, now),
        builtin(BANK_FEES_ID, Builtin::BankFees, -1, now),
    ]
}

/// Adds the built-in rules that are missing and applies them to the lines already imported.
pub fn ensure_builtins(db: &Db, now: i64, source: Source) -> AppResult<Option<Written>> {
    let mut rules = db.read(store::rules)?;
    let mut changes = Vec::new();
    for rule in builtin_rules(now) {
        if !rules.iter().any(|r| r.builtin == rule.builtin) {
            changes.push(Change::insert(store::RULES, &rule)?);
            rules.push(rule);
        }
    }
    if changes.is_empty() {
        return Ok(None);
    }
    rules.sort_by_key(|r| (r.position, r.created_at));
    changes.extend(db.read(|c| apply::changes(c, &rules, now))?);
    db.write(Op::new("Add Built-in Rules", changes), source)
        .map(Some)
}
