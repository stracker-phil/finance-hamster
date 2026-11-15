//! The statements as documents (ADR 0056): every imported statement by account for Dokumente → Kontoauszüge,
//! and one statement's facts beside its PDF.

use std::collections::HashMap;

use chrono::TimeZone;
use serde::Serialize;
use ts_rs::TS;

use super::store::{StatementRow, TRANSACTIONS, statement_by_id, statements_of};
use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::money::{Money, format_money};
use crate::settings::NumberFormat;
use crate::{tr, tr_plural};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementListRow {
    pub id: String,
    /// `Nr. 003/2026`.
    pub title: String,
    /// `7 Sep 2026 – 10 Nov 2026`.
    pub period: String,
    pub closing: String,
    /// `62 lines`.
    pub lines: String,
}

/// One account's statements, newest first.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AccountStatements {
    pub account_id: String,
    pub name: String,
    pub statements: Vec<StatementListRow>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementFact {
    pub label: String,
    pub value: String,
}

/// A statement's facts as printed and as imported; nothing about it can be edited.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatementView {
    pub id: String,
    pub account_id: String,
    /// `Private · Nr. 003/2026`.
    pub title: String,
    pub facts: Vec<StatementFact>,
}

fn eur(cents: i64, format: NumberFormat) -> String {
    format_money(&Money::eur(cents), format)
}

fn title(s: &StatementRow) -> String {
    format!("Nr. {:03}/{}", s.number, s.year)
}

fn day(iso: &str, order: DateOrder) -> String {
    date::parse(iso)
        .map(|d| date::day_month_year(d, order))
        .unwrap_or_else(|| iso.to_string())
}

fn period(s: &StatementRow, order: DateOrder) -> String {
    format!("{} – {}", day(&s.from_date, order), day(&s.to_date, order))
}

fn lines_text(n: usize) -> String {
    tr_plural!(n, "{n} line", "{n} lines", "{n} Buchung", "{n} Buchungen")
}

fn line_counts(db: &Db) -> AppResult<HashMap<String, usize>> {
    db.read(|conn| {
        let mut query = conn.prepare(&format!(
            "SELECT statement_id, COUNT(*) FROM {TRANSACTIONS} GROUP BY statement_id"
        ))?;
        let rows = query.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as usize)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

/// Every account with statements, in the accounts' order; `query` keeps the statements whose number, period or
/// account name contains it.
pub fn list(
    db: &Db,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<AccountStatements>> {
    let counts = line_counts(db)?;
    let needle = query.trim().to_lowercase();
    let mut groups = Vec::new();
    for account in db.read(accounts::rows)? {
        let account_matches = account.name.to_lowercase().contains(&needle);
        let statements: Vec<StatementListRow> = db
            .read(|c| statements_of(c, &account.id))?
            .into_iter()
            .map(|s| StatementListRow {
                title: title(&s),
                period: period(&s, order),
                closing: eur(s.new_balance, format),
                lines: lines_text(counts.get(&s.id).copied().unwrap_or(0)),
                id: s.id,
            })
            .filter(|row| {
                needle.is_empty()
                    || account_matches
                    || [&row.title, &row.period]
                        .iter()
                        .any(|t| t.to_lowercase().contains(&needle))
            })
            .collect();
        if !statements.is_empty() {
            groups.push(AccountStatements {
                account_id: account.id,
                name: account.name,
                statements,
            });
        }
    }
    Ok(groups)
}

fn imported_on(millis: i64, order: DateOrder) -> String {
    chrono::Local
        .timestamp_millis_opt(millis)
        .single()
        .map(|t| date::day_month_year(t.date_naive(), order))
        .unwrap_or_default()
}

pub fn view(db: &Db, id: &str, format: NumberFormat, order: DateOrder) -> AppResult<StatementView> {
    let s = db.read(|c| statement_by_id(c, id))?.ok_or_else(|| {
        AppError::NotFound(tr!("No such statement", "Diesen Kontoauszug gibt es nicht"))
    })?;
    let account = db
        .read(accounts::rows)?
        .into_iter()
        .find(|a| a.id == s.account_id)
        .map(|a| a.name)
        .unwrap_or_default();
    let lines = line_counts(db)?.get(&s.id).copied().unwrap_or(0);
    let fact = |label: String, value: String| StatementFact { label, value };
    let facts = vec![
        fact(tr!("Account", "Konto"), account.clone()),
        fact(tr!("Number", "Nummer"), title(&s)),
        fact(tr!("Period", "Zeitraum"), period(&s, order)),
        fact(
            tr!("Opening balance", "Alter Saldo"),
            eur(s.old_balance, format),
        ),
        fact(tr!("Credits", "Gutschriften"), eur(s.credits, format)),
        fact(tr!("Debits", "Belastungen"), eur(s.debits, format)),
        fact(
            tr!("Closing balance", "Neuer Saldo"),
            eur(s.new_balance, format),
        ),
        fact(tr!("Lines", "Buchungen"), lines.to_string()),
        fact(tr!("Pages", "Seiten"), s.pages.to_string()),
        fact(
            tr!("Imported", "Importiert"),
            imported_on(s.imported_at, order),
        ),
    ];
    Ok(StatementView {
        title: format!("{account} · {}", title(&s)),
        id: s.id,
        account_id: s.account_id,
        facts,
    })
}
