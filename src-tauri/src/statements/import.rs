//! Importing a statement PDF: read and check it (`prepare`, nothing written yet), then copy the file unchanged
//! to `statements/<year>/` and store its lines once (`commit`, one undoable op).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;

use super::check::{self, Chain, Previous};
use super::parse::{
    self,
    model::{ParsedLine, ParsedStatement},
};
use super::store::{self, StatementRow, TransactionRow};
use crate::accounts::{self, AccountRow};
use crate::date::{self, DateOrder};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;
use crate::money::{Money, format_money, format_plain, format_signed};
use crate::platform::pdf::Pdf;
use crate::settings::NumberFormat;
use crate::store::json_file::create_private_dir;

pub const FOLDER: &str = "statements";

/// A read and checked statement waiting for the owner's Import.
#[derive(Clone, Debug)]
pub struct Pending {
    pub bytes: Vec<u8>,
    pub hash: String,
    pub pages: u32,
    pub parsed: ParsedStatement,
    pub account: AccountRow,
    pub file: String,
}

/// Statements read but not yet imported, by token; the page only ever holds the token.
#[derive(Default)]
pub struct ImportQueue(Mutex<HashMap<String, Pending>>);

impl ImportQueue {
    pub fn put(&self, pending: Pending) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.0.lock_unpoisoned().insert(token.clone(), pending);
        token
    }

    pub fn take(&self, token: &str) -> Option<Pending> {
        self.0.lock_unpoisoned().remove(token)
    }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum ChainKind {
    First,
    Continues,
    Gap,
}

/// What the Import sheet shows.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ImportPreview {
    pub token: String,
    pub file_name: String,
    /// `Kontoauszug Nr. 010/2026 · 4 pages · 1 Nov – 10 Nov 2026`.
    pub summary: String,
    pub account_name: String,
    pub lines: u32,
    /// `1,412.30 + 2,250.00 − 1,986.44 = 1,675.86`.
    pub balance: String,
    pub chain: ChainKind,
    pub chain_title: String,
    pub chain_detail: String,
    /// Where the PDF is copied, relative to the data folder.
    pub file: String,
}

fn number_label(number: u32, year: i32) -> String {
    format!("Nr. {number:03}/{year}")
}

/// `private-010` from the account's name: lower case, umlauts spelled out, anything else a dash.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            c if c.is_ascii_alphanumeric() => out.push(c),
            _ if !out.ends_with('-') => out.push('-'),
            _ => {}
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "account".into()
    } else {
        out
    }
}

/// Reads and checks a statement; refuses unreadable files, failed checks, unknown accounts and duplicates.
pub fn prepare(db: &Db, config_dir: &Path, bytes: Vec<u8>) -> AppResult<Pending> {
    let pdf = Pdf::from_bytes(&bytes)?;
    let pages = pdf.text_pages()?;
    let parsed = parse::parse(&pages).map_err(|e| {
        AppError::InvalidInput(format!("This isn't a statement Hamster can read: {e}"))
    })?;
    check::check(&parsed)
        .map_err(|e| AppError::InvalidInput(format!("The statement doesn't add up. {e}")))?;
    let hash = super::file_hash(&bytes);
    if let Some(existing) = db.read(|c| store::statement_by_hash(c, &hash))? {
        return Err(AppError::InvalidInput(format!(
            "This file was already imported as {}",
            number_label(existing.number, existing.year)
        )));
    }
    let account = db
        .read(accounts::rows)?
        .into_iter()
        .find(|a| a.iban == parsed.iban)
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "No account has the IBAN {}; add it in Settings → Accounts first",
                accounts::mask_iban(&parsed.iban)
            ))
        })?;
    let existing = db.read(|c| store::statements_of(c, &account.id))?;
    if existing
        .iter()
        .any(|s| s.year == parsed.year && s.number == parsed.number)
    {
        return Err(AppError::InvalidInput(format!(
            "{} of {} is already imported",
            number_label(parsed.number, parsed.year),
            account.name
        )));
    }
    let file = free_file(config_dir, &account, &parsed, &hash);
    Ok(Pending {
        bytes,
        hash,
        pages: pages.len() as u32,
        parsed,
        account,
        file,
    })
}

/// `statements/<year>/<account>-<nr>.pdf`, year of the statement's end; a taken name gets `-2`, `-3`.
fn free_file(
    config_dir: &Path,
    account: &AccountRow,
    parsed: &ParsedStatement,
    hash: &str,
) -> String {
    let base = format!(
        "{FOLDER}/{}/{}-{:03}",
        parsed.to.format("%Y"),
        slug(&account.name),
        parsed.number
    );
    let mut name = format!("{base}.pdf");
    let mut n = 2;
    loop {
        let path = config_dir.join(&name);
        let same = std::fs::read(&path).is_ok_and(|bytes| super::file_hash(&bytes) == hash);
        if !path.exists() || same {
            return name;
        }
        name = format!("{base}-{n}.pdf");
        n += 1;
    }
}

/// The previous statement of the account, the one this one should continue.
fn previous(db: &Db, account_id: &str, parsed: &ParsedStatement) -> AppResult<Option<Previous>> {
    let all = db.read(|c| store::statements_of(c, account_id))?;
    Ok(all
        .into_iter()
        .filter(|s| (s.year, s.number) < (parsed.year, parsed.number))
        .max_by_key(|s| (s.year, s.number))
        .map(|s| Previous {
            number: s.number,
            year: s.year,
            new_balance: s.new_balance,
        }))
}

pub fn preview(
    db: &Db,
    token: String,
    file_name: String,
    pending: &Pending,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<ImportPreview> {
    let p = &pending.parsed;
    let balance = format!(
        "{} + {} − {} = {}",
        format_plain(p.old_balance, format),
        format_plain(p.credits, format),
        format_plain(p.debits.abs(), format),
        format_plain(p.new_balance, format)
    );
    let previous = previous(db, &pending.account.id, p)?;
    let (chain, chain_title, chain_detail) = match check::chain(previous.as_ref(), p) {
        Chain::First => (
            ChainKind::First,
            "The account's first statement".to_string(),
            "Later statements are checked against it.".to_string(),
        ),
        Chain::Continues => {
            let prev = previous
                .as_ref()
                .map(|q| number_label(q.number, q.year))
                .unwrap_or_default();
            (
                ChainKind::Continues,
                format!("Continues {prev} without a gap"),
                format!(
                    "Old balance {} equals the last new balance.",
                    format_money(&Money::eur(p.old_balance), format)
                ),
            )
        }
        Chain::MissingNumbers { expected } => (
            ChainKind::Gap,
            "Statements are missing before this one".to_string(),
            format!(
                "Expected {} next; it imports, and the account shows the gap.",
                number_label(expected, p.year)
            ),
        ),
        Chain::BalanceGap { previous } => (
            ChainKind::Gap,
            "The old balance doesn't continue the last statement".to_string(),
            format!(
                "The last new balance was {}; it imports, and the account shows the gap.",
                format_signed(&Money::eur(previous), format)
            ),
        ),
    };
    Ok(ImportPreview {
        token,
        file_name,
        summary: format!(
            "Kontoauszug {} · {} {} · {} – {}",
            number_label(p.number, p.year),
            pending.pages,
            if pending.pages == 1 { "page" } else { "pages" },
            date::day_month(p.from, order),
            date::day_month_year(p.to, order)
        ),
        account_name: pending.account.name.clone(),
        lines: p.lines.len() as u32,
        balance,
        chain,
        chain_title,
        chain_detail,
        file: pending.file.clone(),
    })
}

/// `own_ibans`: the owner's other accounts; a line paying one of them is a transfer.
fn transaction_row(
    statement: &StatementRow,
    line: &ParsedLine,
    own_ibans: &[String],
) -> AppResult<TransactionRow> {
    let card = line.card.as_ref();
    let original = card.and_then(|c| c.original.clone());
    Ok(TransactionRow {
        id: uuid::Uuid::new_v4().to_string(),
        statement_id: statement.id.clone(),
        account_id: statement.account_id.clone(),
        page: line.page as u32,
        row: line.row as u32,
        value_date: date::iso(line.value_date),
        cents: line.cents,
        text: line.text.clone(),
        extra: serde_json::to_string(&line.extra)?,
        counterparty: line.counterparty.clone(),
        counterparty_iban: line.counterparty_iban.clone(),
        card_kind: card.map(|c| c.kind.as_str().to_string()),
        card_date: card.map(|c| date::iso(c.date)),
        card_time: card.and_then(|c| c.time.clone()),
        merchant: card.map(|c| c.merchant.clone()),
        original_cents: original.as_ref().map(|(cents, _)| *cents),
        original_currency: original.map(|(_, code)| code),
        fx_fee: line.fx.as_ref().map(|f| f.fee),
        fx_rate: line.fx.as_ref().map(|f| f.rate.clone()),
        fx_date: line.fx.as_ref().map(|f| date::iso(f.rate_date)),
        transfer: line
            .counterparty_iban
            .as_ref()
            .is_some_and(|iban| own_ibans.contains(iban)),
    })
}

/// Copies the PDF and stores the statement with its lines as one op; transfers to the owner's other accounts
/// are tagged.
pub fn commit(
    db: &Db,
    config_dir: &Path,
    pending: Pending,
    now: i64,
    source: Source,
) -> AppResult<(Written, StatementRow)> {
    let p = &pending.parsed;
    let path = config_dir.join(&pending.file);
    let created = !path.exists();
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    if created {
        std::fs::write(&path, &pending.bytes)?;
    }
    let own: Vec<String> = db
        .read(accounts::rows)?
        .into_iter()
        .filter(|a| a.own && a.id != pending.account.id)
        .map(|a| a.iban)
        .collect();
    let statement = StatementRow {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: pending.account.id.clone(),
        number: p.number,
        year: p.year,
        from_date: date::iso(p.from),
        to_date: date::iso(p.to),
        old_balance: p.old_balance,
        credits: p.credits,
        debits: p.debits,
        new_balance: p.new_balance,
        layout: p.layout.to_string(),
        file_hash: pending.hash.clone(),
        file: pending.file.clone(),
        pages: pending.pages,
        imported_at: now,
    };
    let mut changes = vec![Change::insert(store::STATEMENTS, &statement)?];
    for line in &p.lines {
        let row = transaction_row(&statement, line, &own)?;
        changes.push(Change::insert(store::TRANSACTIONS, &row)?);
    }
    let op = Op::new(
        format!("Import Statement {}", number_label(p.number, p.year)),
        changes,
    );
    match db.write(op, source) {
        Ok(written) => Ok((written, statement)),
        Err(e) => {
            if created {
                let _ = std::fs::remove_file(&path);
            }
            Err(e)
        }
    }
}
