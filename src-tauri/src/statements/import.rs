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
use crate::rules;
use crate::settings::NumberFormat;
use crate::store::json_file::write_private_file;
use crate::tr;

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

    pub fn clear(&self) {
        self.0.lock_unpoisoned().clear();
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
        AppError::InvalidInput(tr!(
            "This isn't a statement Hamster can read: {e}",
            "Das ist kein Kontoauszug, den Hamster lesen kann: {e}"
        ))
    })?;
    check::check(&parsed).map_err(|e| {
        AppError::InvalidInput(tr!(
            "The statement doesn't add up. {e}",
            "Der Kontoauszug geht nicht auf. {e}"
        ))
    })?;
    let hash = super::file_hash(&bytes);
    if let Some(existing) = db.read(|c| store::statement_by_hash(c, &hash))? {
        let label = number_label(existing.number, existing.year);
        return Err(AppError::InvalidInput(tr!(
            "This file was already imported as {label}",
            "Diese Datei wurde schon als {label} importiert"
        )));
    }
    let account = db
        .read(accounts::rows)?
        .into_iter()
        .find(|a| a.iban == parsed.iban)
        .ok_or_else(|| {
            let iban = accounts::mask_iban(&parsed.iban);
            AppError::InvalidInput(tr!(
                "No account has the IBAN {iban}; add it in Settings → Accounts first",
                "Kein Konto hat die IBAN {iban}; lege es zuerst unter Einstellungen → Konten an"
            ))
        })?;
    let existing = db.read(|c| store::statements_of(c, &account.id))?;
    if existing
        .iter()
        .any(|s| s.year == parsed.year && s.number == parsed.number)
    {
        let (label, name) = (number_label(parsed.number, parsed.year), &account.name);
        return Err(AppError::InvalidInput(tr!(
            "{label} of {name} is already imported",
            "{label} von {name} ist schon importiert"
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
            tr!(
                "The account's first statement",
                "Der erste Kontoauszug des Kontos"
            ),
            tr!(
                "Later statements are checked against it.",
                "Spätere Kontoauszüge werden daran geprüft."
            ),
        ),
        Chain::Continues => {
            let prev = previous
                .as_ref()
                .map(|q| number_label(q.number, q.year))
                .unwrap_or_default();
            (
                ChainKind::Continues,
                tr!(
                    "Continues {prev} without a gap",
                    "Schließt lückenlos an {prev} an"
                ),
                tr!(
                    "Old balance {} equals the last new balance.",
                    "Der alte Saldo {} entspricht dem letzten neuen Saldo.",
                    format_money(&Money::eur(p.old_balance), format)
                ),
            )
        }
        Chain::MissingNumbers { expected } => (
            ChainKind::Gap,
            tr!(
                "Statements are missing before this one",
                "Vor diesem fehlen Kontoauszüge"
            ),
            tr!(
                "Expected {} next; it imports, and the account shows the gap.",
                "Erwartet war {}; er wird importiert, und das Konto zeigt die Lücke.",
                number_label(expected, p.year)
            ),
        ),
        Chain::BalanceGap { previous } => (
            ChainKind::Gap,
            tr!(
                "The old balance doesn't continue the last statement",
                "Der alte Saldo schließt nicht an den letzten Kontoauszug an"
            ),
            tr!(
                "The last new balance was {}; it imports, and the account shows the gap.",
                "Der letzte neue Saldo war {}; er wird importiert, und das Konto zeigt die Lücke.",
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
            if pending.pages == 1 {
                tr!("page", "Seite")
            } else {
                tr!("pages", "Seiten")
            },
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

/// `own`: the owner's accounts; a line paying another of them is a transfer.
fn transaction_row(
    statement: &StatementRow,
    line: &ParsedLine,
    own: &[rules::engine::OwnAccount],
) -> AppResult<TransactionRow> {
    let card = line.card.as_ref();
    let original = card.and_then(|c| c.original.clone());
    let mut row = TransactionRow {
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
        transfer: false,
    };
    row.transfer = rules::engine::pays_own(&row, own);
    Ok(row)
}

/// Copies the PDF and stores the statement with its lines as one op; transfers to the owner's other accounts
/// are tagged and the rules decide what they can.
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
    if created {
        write_private_file(&path, &pending.bytes)?;
    }
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
    let rules = db.read(rules::store::rules)?;
    let own_accounts = db.read(rules::apply::own_accounts)?;
    let closed = db.read(crate::taxyears::closed_years)?;
    let mut changes = vec![Change::insert(store::STATEMENTS, &statement)?];
    let mut decisions = Vec::new();
    for line in &p.lines {
        let row = transaction_row(&statement, line, &own_accounts)?;
        changes.push(Change::insert(store::TRANSACTIONS, &row)?);
        if crate::taxyears::in_closed(&closed, &row.value_date) {
            continue;
        }
        if let Some(decision) = rules::engine::decide(&rules, &row, &own_accounts, now) {
            decisions.push(Change::insert(rules::store::DECISIONS, &decision)?);
        }
    }
    changes.extend(decisions);
    let op = Op::new(
        tr!(
            "Import Statement {}",
            "Kontoauszug importieren {}",
            number_label(p.number, p.year)
        ),
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
