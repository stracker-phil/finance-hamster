//! What the Account page shows: an account's lines grouped by statement, each with how it was decided.

use serde::Serialize;
use ts_rs::TS;

use super::store::{self, StatementRow, TransactionRow};
use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money, format_signed};
use crate::rules::store::{self as rules_store, Action, DecisionRow};
use crate::rules::{self, ItemOption};
use crate::settings::NumberFormat;
use crate::{tr, tr_plural};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct LineView {
    pub id: String,
    /// `10 Nov`.
    pub date: String,
    /// The merchant, the counterparty or the booking text.
    pub title: String,
    /// The foreign amount of a card payment, `USD 24.00`.
    pub original: Option<String>,
    /// `−€22.48`, `+€2,850.00`.
    pub amount: String,
    pub credit: bool,
    pub transfer: bool,
    pub status: LineStatus,
    /// `KZ 720`, `Ignored by rule`, `Waiting for review`.
    pub note: Option<String>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum LineStatus {
    Review,
    Anv,
    Subscription,
    Ignored,
    Transfer,
}

fn status(
    t: &TransactionRow,
    decision: Option<&DecisionRow>,
    items: &[ItemOption],
) -> (LineStatus, Option<String>) {
    let Some(d) = decision else {
        return (
            LineStatus::Review,
            Some(tr!("Waiting for review", "Wartet auf Prüfung")),
        );
    };
    match d.kind {
        Action::Anv => {
            let label = d
                .item_key
                .as_ref()
                .and_then(|key| items.iter().find(|i| &i.key == key))
                .map(|i| i.label.clone());
            (LineStatus::Anv, label)
        }
        Action::Subscription => (LineStatus::Subscription, None),
        Action::Ignore
            if t.transfer || d.rule_id.as_deref() == Some(crate::rules::TRANSFERS_ID) =>
        {
            (LineStatus::Transfer, None)
        }
        Action::Ignore => {
            let text = if d.by_hand() {
                tr!("Ignored", "Ignoriert")
            } else {
                tr!("Ignored by rule", "Per Regel ignoriert")
            };
            (LineStatus::Ignored, Some(text))
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementGroup {
    pub id: String,
    /// `Nr. 003/2026`.
    pub label: String,
    /// `7 Sep – 10 Nov`.
    pub range: String,
    /// `62 lines`.
    pub count: String,
    pub lines: Vec<LineView>,
    /// The statements missing between this one and the next older one, in words.
    pub missing_before: Option<String>,
}

/// `Nr. 002/2026 was never imported` when `older` isn't the statement just before `newer`.
fn missing_between(older: &StatementRow, newer: &StatementRow) -> Option<String> {
    if super::view::follows(older, newer) {
        return None;
    }
    Some(
        if newer.year == older.year && newer.number == older.number + 2 {
            tr!(
                "Nr. {:03}/{} was never imported",
                "Nr. {:03}/{} wurde nie importiert",
                older.number + 1,
                older.year
            )
        } else {
            tr!(
                "Statements between Nr. {:03}/{} and Nr. {:03}/{} were never imported",
                "Kontoauszüge zwischen Nr. {:03}/{} und Nr. {:03}/{} wurden nie importiert",
                older.number,
                older.year,
                newer.number,
                newer.year
            )
        },
    )
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AccountLines {
    pub id: String,
    pub name: String,
    pub iban_masked: String,
    pub groups: Vec<StatementGroup>,
}

/// `10 Nov` from a stored `YYYY-MM-DD`; empty when unreadable.
fn day_month_or_empty(iso: &str, order: DateOrder) -> String {
    date::parse(iso)
        .map(|d| date::day_month(d, order))
        .unwrap_or_default()
}

fn line_view(
    t: TransactionRow,
    decision: Option<&DecisionRow>,
    items: &[ItemOption],
    format: NumberFormat,
    order: DateOrder,
) -> LineView {
    let (line_status, note) = status(&t, decision, items);
    let title = t.merchant.or(t.counterparty).unwrap_or(t.text);
    let original = match (t.original_cents, t.original_currency) {
        (Some(cents), Some(currency)) => Some(format_money(&Money { cents, currency }, format)),
        _ => None,
    };
    LineView {
        date: day_month_or_empty(&t.value_date, order),
        title,
        original,
        amount: format_signed(&Money::eur(t.cents), format),
        credit: t.cents > 0,
        transfer: t.transfer,
        status: line_status,
        note,
        id: t.id,
    }
}

/// The account's lines grouped by statement, newest first.
/// `query` keeps only the lines whose title or booking text contains it.
pub fn account_lines(
    db: &Db,
    account_id: &str,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<AccountLines> {
    let account = db
        .read(accounts::rows)?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| {
            AppError::NotFound(tr!("No account {account_id}", "Kein Konto {account_id}"))
        })?;
    let decisions = db.read(rules_store::decisions)?;
    let items = rules::items(db)?;
    let mut groups = Vec::new();
    let needle = query.trim().to_lowercase();
    // Newest first.
    let statements = db.read(|c| store::statements_of(c, account_id))?;
    let olders: Vec<Option<StatementRow>> = statements
        .iter()
        .skip(1)
        .cloned()
        .map(Some)
        .chain([None])
        .collect();
    for (s, older) in statements.into_iter().zip(olders) {
        let missing_before = older.as_ref().and_then(|o| missing_between(o, &s));
        let lines: Vec<TransactionRow> = db
            .read(|c| store::transactions_of(c, &s.id))?
            .into_iter()
            .filter(|t| {
                needle.is_empty()
                    || [
                        t.merchant.as_deref(),
                        t.counterparty.as_deref(),
                        Some(t.text.as_str()),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|text| text.to_lowercase().contains(&needle))
            })
            .collect();
        if lines.is_empty() && !needle.is_empty() {
            continue;
        }
        let range = format!(
            "{} – {}",
            day_month_or_empty(&s.from_date, order),
            day_month_or_empty(&s.to_date, order)
        );
        groups.push(StatementGroup {
            label: format!("Nr. {:03}/{}", s.number, s.year),
            range,
            count: {
                let n = lines.len();
                tr_plural!(n, "{n} line", "{n} lines", "{n} Buchung", "{n} Buchungen")
            },
            lines: lines
                .into_iter()
                .map(|t| {
                    let decision = decisions.get(&t.id);
                    line_view(t, decision, &items, format, order)
                })
                .collect(),
            missing_before,
            id: s.id,
        });
    }
    Ok(AccountLines {
        iban_masked: accounts::mask_iban(&account.iban),
        id: account.id,
        name: account.name,
        groups,
    })
}
