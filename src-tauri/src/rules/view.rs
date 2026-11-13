//! What Settings → Rules shows: the rules grouped by action, each in words with its matches.

use serde::Serialize;
use ts_rs::TS;

use super::store::{self, Action, Builtin, Matches, RuleRow};
use super::{ItemOption, RuleForm};
use crate::accounts::{self, AccountRow};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::money::format_plain;
use crate::settings::NumberFormat;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RuleView {
    pub id: String,
    /// `Bank text contains CLOUDBOX and amount 25.95`.
    pub label: String,
    /// `→ assign item KZ 724 · 10 matches · last 7 Nov`.
    pub detail: String,
    pub builtin: bool,
    pub form: RuleForm,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RuleGroup {
    pub title: String,
    pub rules: Vec<RuleView>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulesView {
    pub groups: Vec<RuleGroup>,
    pub items: Vec<ItemOption>,
}

fn capitalized(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

fn tolerance_text(rule: &RuleRow, format: NumberFormat) -> Option<String> {
    (rule.tolerance_cents > 0).then(|| format_plain(rule.tolerance_cents, format))
}

fn label(rule: &RuleRow, accounts: &[AccountRow], format: NumberFormat) -> String {
    let mut parts = Vec::new();
    let tokens = rule.tokens();
    if !tokens.is_empty() {
        parts.push(format!("bank text contains {}", tokens.join(" or ")));
    }
    if let Some(cents) = rule.amount_cents {
        let mut amount = format!("amount {}", format_plain(cents, format));
        if rule.tolerance_cents > 0 {
            amount.push_str(&format!(
                " ± {}",
                format_plain(rule.tolerance_cents, format)
            ));
        }
        parts.push(amount);
    }
    if let Some(id) = &rule.account_id {
        let name = accounts
            .iter()
            .find(|a| &a.id == id)
            .map_or("a deleted account", |a| a.name.as_str());
        parts.push(format!("account is {name}"));
    }
    if rule.own_counterparty {
        parts.push("counterparty is one of my accounts".into());
    }
    capitalized(parts.join(" and "))
}

fn detail(rule: &RuleRow, items: &[ItemOption], matches: &Matches, order: DateOrder) -> String {
    let mut parts = Vec::new();
    match rule.builtin {
        Some(Builtin::Transfers) => parts.push("Transfers · built in".to_string()),
        Some(Builtin::BankFees) => parts.push("Bank fees · built in".to_string()),
        None => {}
    }
    if rule.action == Action::Anv {
        let key = rule.item_key.clone().unwrap_or_default();
        let target = items
            .iter()
            .find(|i| i.key == key)
            .map_or(key, |i| format!("KZ {}", i.kz));
        parts.push(format!("→ assign item {target}"));
    }
    parts.push(match matches.count {
        1 => "1 match".to_string(),
        n => format!("{n} matches"),
    });
    if let Some(last) = matches.last.as_deref().and_then(date::parse) {
        parts.push(format!("last {}", date::day_month(last, order)));
    }
    parts.join(" · ")
}

fn form(rule: &RuleRow, format: NumberFormat) -> RuleForm {
    RuleForm {
        contains: rule.tokens(),
        amount: rule.amount_cents.map(|c| format_plain(c, format)),
        tolerance: tolerance_text(rule, format),
        account_id: rule.account_id.clone(),
        own_counterparty: rule.own_counterparty,
        action: rule.action,
        item_key: rule.item_key.clone(),
    }
}

pub fn rules(db: &Db, format: NumberFormat, order: DateOrder) -> AppResult<RulesView> {
    let rules = db.read(store::rules)?;
    let matches = db.read(store::matches_by_rule)?;
    let accounts = db.read(accounts::rows)?;
    let items = super::items(db)?;
    let groups = [
        (Action::Ignore, "Ignore"),
        (Action::Subscription, "Subscription"),
        (Action::Anv, "ANV"),
    ]
    .into_iter()
    .map(|(action, title)| RuleGroup {
        title: title.into(),
        rules: rules
            .iter()
            .filter(|r| r.action == action)
            .map(|r| RuleView {
                id: r.id.clone(),
                label: label(r, &accounts, format),
                detail: detail(
                    r,
                    &items,
                    matches.get(&r.id).unwrap_or(&Matches::default()),
                    order,
                ),
                builtin: r.builtin.is_some(),
                form: form(r, format),
            })
            .collect(),
    })
    .filter(|g| !g.rules.is_empty())
    .collect();
    Ok(RulesView { groups, items })
}
