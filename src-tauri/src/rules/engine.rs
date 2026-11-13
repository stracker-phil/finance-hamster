//! Which rule decides a line: rules run top to bottom and the first whose conditions all hold wins.

use super::store::{DecisionRow, RuleRow};
use crate::statements::store::TransactionRow;

/// A line as the conditions see it.
pub struct Line<'a> {
    /// Booking text, detail lines, merchant and counterparty, upper case.
    text: String,
    abs_cents: i64,
    account_id: &'a str,
    counterparty_own: bool,
}

/// An own account's id and IBAN, to tell transfers between own accounts.
pub type OwnAccount = (String, String);

/// The text conditions search: booking text, detail lines, merchant and counterparty, upper case.
pub fn line_text(t: &TransactionRow) -> String {
    let extra: Vec<String> = serde_json::from_str(&t.extra).unwrap_or_default();
    let mut parts = vec![t.text.as_str()];
    parts.extend(extra.iter().map(String::as_str));
    parts.extend(t.merchant.as_deref());
    parts.extend(t.counterparty.as_deref());
    parts.join("\n").to_uppercase()
}

impl<'a> Line<'a> {
    pub fn of(t: &'a TransactionRow, own: &[OwnAccount]) -> Line<'a> {
        Line {
            text: line_text(t),
            abs_cents: t.cents.abs(),
            account_id: &t.account_id,
            counterparty_own: t.counterparty_iban.as_ref().is_some_and(|iban| {
                own.iter()
                    .any(|(id, own_iban)| own_iban == iban && id != &t.account_id)
            }),
        }
    }
}

/// True when every condition the rule has holds; a rule without conditions matches nothing.
pub fn matches(rule: &RuleRow, line: &Line) -> bool {
    let tokens = rule.tokens();
    if tokens.is_empty()
        && rule.amount_cents.is_none()
        && rule.account_id.is_none()
        && !rule.own_counterparty
    {
        return false;
    }
    let text_holds = tokens.is_empty()
        || tokens
            .iter()
            .any(|t| line.text.contains(&t.trim().to_uppercase()));
    let amount_holds = rule
        .amount_cents
        .is_none_or(|amount| (line.abs_cents - amount.abs()).abs() <= rule.tolerance_cents.abs());
    let account_holds = rule
        .account_id
        .as_deref()
        .is_none_or(|a| a == line.account_id);
    let counterparty_holds = !rule.own_counterparty || line.counterparty_own;
    text_holds && amount_holds && account_holds && counterparty_holds
}

/// The decision the first matching rule makes, if any rule matches.
pub fn decide(
    rules: &[RuleRow],
    t: &TransactionRow,
    own: &[OwnAccount],
    now: i64,
) -> Option<DecisionRow> {
    let line = Line::of(t, own);
    rules
        .iter()
        .find(|r| matches(r, &line))
        .map(|r| DecisionRow {
            id: t.id.clone(),
            kind: r.action,
            item_key: r.item_key.clone(),
            subscription_id: r.subscription_id.clone(),
            document_id: r.document_id.clone(),
            rule_id: Some(r.id.clone()),
            decided_at: now,
        })
}
