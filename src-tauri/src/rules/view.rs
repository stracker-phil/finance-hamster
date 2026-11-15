//! What Settings → Rules shows: the rules grouped by action, each in words with its matches.

use serde::Serialize;
use ts_rs::TS;

use super::store::{self, Action, Builtin, Matches, RuleRow};
use super::{ItemOption, RuleForm};
use crate::accounts::{self, AccountRow};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::i18n::Text;
use crate::money::format_plain;
use crate::settings::NumberFormat;
use crate::{tr, tr_plural};

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
        let any = tokens.join(&tr!(" or ", " oder "));
        parts.push(tr!("bank text contains {any}", "Banktext enthält {any}"));
    }
    if let Some(cents) = rule.amount_cents {
        let value = format_plain(cents, format);
        let mut amount = tr!("amount {value}", "Betrag {value}");
        if rule.tolerance_cents > 0 {
            amount.push_str(&format!(
                " ± {}",
                format_plain(rule.tolerance_cents, format)
            ));
        }
        parts.push(amount);
    }
    if let Some(id) = &rule.account_id {
        let name = accounts.iter().find(|a| &a.id == id).map_or_else(
            || tr!("a deleted account", "ein gelöschtes Konto"),
            |a| a.name.clone(),
        );
        parts.push(tr!("account is {name}", "Konto ist {name}"));
    }
    if rule.own_counterparty {
        parts.push(tr!(
            "counterparty is one of my accounts",
            "Gegenkonto ist eines meiner Konten"
        ));
    }
    capitalized(parts.join(&tr!(" and ", " und ")))
}

fn detail(rule: &RuleRow, items: &[ItemOption], matches: &Matches, order: DateOrder) -> String {
    let mut parts = Vec::new();
    match rule.builtin {
        Some(Builtin::Transfers) => {
            parts.push(tr!("Transfers · built in", "Umbuchungen · integriert"));
        }
        Some(Builtin::BankFees) => {
            parts.push(tr!("Bank fees · built in", "Bankgebühren · integriert"));
        }
        None => {}
    }
    if rule.action == Action::Anv {
        let key = rule.item_key.clone().unwrap_or_default();
        let target = items
            .iter()
            .find(|i| i.key == key)
            .map_or(key, |i| format!("KZ {}", i.kz));
        parts.push(tr!("→ assign item {target}", "→ Posten {target} zuordnen"));
    }
    let count = matches.count;
    parts.push(tr_plural!(
        count,
        "{count} match",
        "{count} matches",
        "{count} Treffer",
        "{count} Treffer"
    ));
    if let Some(last) = matches.last.as_deref().and_then(date::parse) {
        let day = date::day_month(last, order);
        parts.push(tr!("last {day}", "zuletzt {day}"));
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
        (Action::Ignore, Text::new("Ignore", "Ignorieren")),
        (Action::Subscription, Text::new("Subscription", "Abo")),
        (Action::Anv, Text::new("ANV", "ANV")),
    ]
    .into_iter()
    .map(|(action, title)| RuleGroup {
        title: title.get().into(),
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
