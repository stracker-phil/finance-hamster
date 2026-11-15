//! The `statements` and `transactions` rows. Lines are stored once, at import, with the layout that read them,
//! and never parsed again.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

pub const STATEMENTS: &str = "statements";
pub const TRANSACTIONS: &str = "transactions";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StatementRow {
    pub id: String,
    pub account_id: String,
    pub number: u32,
    pub year: i32,
    /// `YYYY-MM-DD`.
    pub from_date: String,
    pub to_date: String,
    pub old_balance: i64,
    pub credits: i64,
    pub debits: i64,
    pub new_balance: i64,
    pub layout: String,
    pub file_hash: String,
    /// Relative to the data folder: `statements/2026/private-010.pdf`.
    pub file: String,
    pub pages: u32,
    pub imported_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TransactionRow {
    pub id: String,
    pub statement_id: String,
    pub account_id: String,
    pub page: u32,
    pub row: u32,
    /// The payment date, `YYYY-MM-DD`.
    pub value_date: String,
    pub cents: i64,
    pub text: String,
    /// The indented detail lines, as JSON text.
    pub extra: String,
    pub counterparty: Option<String>,
    pub counterparty_iban: Option<String>,
    /// `online` or `terminal` for card payments.
    pub card_kind: Option<String>,
    pub card_date: Option<String>,
    pub card_time: Option<String>,
    pub merchant: Option<String>,
    pub original_cents: Option<i64>,
    pub original_currency: Option<String>,
    pub fx_fee: Option<i64>,
    pub fx_rate: Option<String>,
    pub fx_date: Option<String>,
    /// Between two of the owner's accounts; never counts twice.
    pub transfer: bool,
}

impl TransactionRow {
    /// What left the account: a charge is positive, a refund or other credit negative.
    pub fn outflow_cents(&self) -> i64 {
        -self.cents
    }
}

/// What a set of lines paid, refunds netted against charges.
pub fn net_outflow(lines: &[impl std::borrow::Borrow<TransactionRow>]) -> i64 {
    lines.iter().map(|l| l.borrow().outflow_cents()).sum()
}

const STATEMENT_COLUMNS: &str = "id, account_id, number, year, from_date, to_date, old_balance, credits, debits, \
     new_balance, layout, file_hash, file, pages, imported_at";

fn statement(row: &rusqlite::Row) -> rusqlite::Result<StatementRow> {
    Ok(StatementRow {
        id: row.get(0)?,
        account_id: row.get(1)?,
        number: row.get(2)?,
        year: row.get(3)?,
        from_date: row.get(4)?,
        to_date: row.get(5)?,
        old_balance: row.get(6)?,
        credits: row.get(7)?,
        debits: row.get(8)?,
        new_balance: row.get(9)?,
        layout: row.get(10)?,
        file_hash: row.get(11)?,
        file: row.get(12)?,
        pages: row.get(13)?,
        imported_at: row.get(14)?,
    })
}

fn statements_where(
    conn: &Connection,
    clause: &str,
    param: &dyn rusqlite::ToSql,
) -> AppResult<Vec<StatementRow>> {
    let sql = format!(
        "SELECT {STATEMENT_COLUMNS} FROM {STATEMENTS} WHERE {clause} ORDER BY year DESC, number DESC"
    );
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([param], statement)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// An account's statements, newest first.
pub fn statements_of(conn: &Connection, account_id: &str) -> AppResult<Vec<StatementRow>> {
    statements_where(conn, "account_id = ?", &account_id)
}

pub fn statement_by_id(conn: &Connection, id: &str) -> AppResult<Option<StatementRow>> {
    Ok(statements_where(conn, "id = ?", &id)?.into_iter().next())
}

pub fn statement_by_hash(conn: &Connection, hash: &str) -> AppResult<Option<StatementRow>> {
    Ok(statements_where(conn, "file_hash = ?", &hash)?
        .into_iter()
        .next())
}

const TRANSACTION_COLUMNS: &str = "id, statement_id, account_id, page, row, value_date, cents, text, extra, \
     counterparty, counterparty_iban, card_kind, card_date, card_time, merchant, original_cents, \
     original_currency, fx_fee, fx_rate, fx_date, transfer";

fn transaction(row: &rusqlite::Row) -> rusqlite::Result<TransactionRow> {
    Ok(TransactionRow {
        id: row.get(0)?,
        statement_id: row.get(1)?,
        account_id: row.get(2)?,
        page: row.get(3)?,
        row: row.get(4)?,
        value_date: row.get(5)?,
        cents: row.get(6)?,
        text: row.get(7)?,
        extra: row.get(8)?,
        counterparty: row.get(9)?,
        counterparty_iban: row.get(10)?,
        card_kind: row.get(11)?,
        card_date: row.get(12)?,
        card_time: row.get(13)?,
        merchant: row.get(14)?,
        original_cents: row.get(15)?,
        original_currency: row.get(16)?,
        fx_fee: row.get(17)?,
        fx_rate: row.get(18)?,
        fx_date: row.get(19)?,
        transfer: row.get(20)?,
    })
}

/// A statement's lines in printed order.
pub fn transactions_of(conn: &Connection, statement_id: &str) -> AppResult<Vec<TransactionRow>> {
    let sql = format!(
        "SELECT {TRANSACTION_COLUMNS} FROM {TRANSACTIONS} WHERE statement_id = ? ORDER BY page, row"
    );
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([statement_id], transaction)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn transaction_by_id(conn: &Connection, id: &str) -> AppResult<Option<TransactionRow>> {
    let sql = format!("SELECT {TRANSACTION_COLUMNS} FROM {TRANSACTIONS} WHERE id = ?");
    let mut query = conn.prepare(&sql)?;
    let mut rows = query.query_map([id], transaction)?;
    Ok(rows.next().transpose()?)
}

/// Every line, oldest first.
pub fn all_transactions(conn: &Connection) -> AppResult<Vec<TransactionRow>> {
    let sql = format!(
        "SELECT {TRANSACTION_COLUMNS} FROM {TRANSACTIONS} ORDER BY value_date, statement_id, page, row"
    );
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], transaction)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Change, Db, Op, Source};
    use crate::statements::import_tests::open_with_accounts;

    fn statement_row(id: &str, account_id: &str, number: u32, year: i32) -> StatementRow {
        StatementRow {
            id: id.into(),
            account_id: account_id.into(),
            number,
            year,
            from_date: format!("{year}-01-01"),
            to_date: format!("{year}-01-31"),
            old_balance: 100_000,
            credits: 50_000,
            debits: -20_000,
            new_balance: 130_000,
            layout: "sparkasse-erste-1".into(),
            file_hash: format!("hash-{id}"),
            file: format!("statements/{year}/private-{number:03}.pdf"),
            pages: 2,
            imported_at: 1_700_000_000,
        }
    }

    fn transaction_row(id: &str, statement_id: &str, page: u32, row: u32) -> TransactionRow {
        TransactionRow {
            id: id.into(),
            statement_id: statement_id.into(),
            account_id: "account-1".into(),
            page,
            row,
            value_date: "2026-01-05".into(),
            cents: -1_299,
            text: "E-COMM 12,99 AT K1 04.01. STREAMFLIX WIEN 040".into(),
            extra: r#"["SPESEN: 0,49"]"#.into(),
            counterparty: Some("STREAMFLIX WIEN".into()),
            counterparty_iban: Some("AT091999900000000077".into()),
            card_kind: Some("online".into()),
            card_date: Some("2026-01-04".into()),
            card_time: Some("17:40".into()),
            merchant: Some("STREAMFLIX WIEN".into()),
            original_cents: Some(1_500),
            original_currency: Some("USD".into()),
            fx_fee: Some(49),
            fx_rate: Some("1,0912".into()),
            fx_date: Some("2026-01-03".into()),
            transfer: true,
        }
    }

    fn insert_all<T: serde::Serialize>(db: &Db, table: &str, rows: &[T]) {
        let changes = rows
            .iter()
            .map(|row| Change::insert(table, row).unwrap())
            .collect();
        db.write(Op::new("Add", changes), Source::Ui).unwrap();
    }

    fn statements_of_account(db: &Db, account_id: &str) -> Vec<(i32, u32)> {
        db.read(|c| statements_of(c, account_id))
            .unwrap()
            .iter()
            .map(|s| (s.year, s.number))
            .collect()
    }

    #[test]
    fn an_accounts_statements_come_newest_first_by_year_then_number() {
        let (_dir, db) = open_with_accounts();
        insert_all(
            &db,
            STATEMENTS,
            &[
                statement_row("a", "account-1", 12, 2025),
                statement_row("b", "account-1", 1, 2026),
                statement_row("c", "account-1", 10, 2026),
                statement_row("d", "account-1", 2, 2026),
                statement_row("e", "account-2", 5, 2026),
            ],
        );

        assert_eq!(
            statements_of_account(&db, "account-1"),
            vec![(2026, 10), (2026, 2), (2026, 1), (2025, 12)]
        );
        assert_eq!(statements_of_account(&db, "account-2"), vec![(2026, 5)]);
        assert_eq!(statements_of_account(&db, "account-3"), vec![]);
    }

    #[test]
    fn a_statement_row_round_trips_and_is_found_by_id_and_by_hash() {
        let (_dir, db) = open_with_accounts();
        let row = statement_row("a", "account-1", 3, 2026);
        insert_all(&db, STATEMENTS, std::slice::from_ref(&row));

        assert_eq!(
            db.read(|c| statement_by_id(c, "a")).unwrap(),
            Some(row.clone())
        );
        assert_eq!(
            db.read(|c| statement_by_hash(c, "hash-a")).unwrap(),
            Some(row)
        );
    }

    #[test]
    fn an_unknown_id_or_hash_finds_no_statement() {
        let (_dir, db) = open_with_accounts();
        insert_all(&db, STATEMENTS, &[statement_row("a", "account-1", 3, 2026)]);

        assert_eq!(db.read(|c| statement_by_id(c, "nope")).unwrap(), None);
        assert_eq!(db.read(|c| statement_by_hash(c, "nope")).unwrap(), None);
    }

    #[test]
    fn a_line_is_found_by_id_and_an_unknown_id_finds_none() {
        let (_dir, db) = open_with_accounts();
        insert_all(&db, STATEMENTS, &[statement_row("a", "account-1", 1, 2026)]);
        let first = transaction_row("t1", "a", 1, 1);
        insert_all(
            &db,
            TRANSACTIONS,
            &[first.clone(), transaction_row("t2", "a", 1, 2)],
        );

        assert_eq!(
            db.read(|c| transaction_by_id(c, "t1")).unwrap(),
            Some(first)
        );
        assert_eq!(db.read(|c| transaction_by_id(c, "nope")).unwrap(), None);
    }

    #[test]
    fn a_statements_lines_come_in_page_then_row_order_and_only_its_own() {
        let (_dir, db) = open_with_accounts();
        insert_all(
            &db,
            STATEMENTS,
            &[
                statement_row("a", "account-1", 1, 2026),
                statement_row("b", "account-1", 2, 2026),
            ],
        );
        insert_all(
            &db,
            TRANSACTIONS,
            &[
                transaction_row("t3", "a", 2, 1),
                transaction_row("t2", "a", 1, 12),
                transaction_row("t1", "a", 1, 3),
                transaction_row("other", "b", 1, 1),
            ],
        );

        let ids: Vec<String> = db
            .read(|c| transactions_of(c, "a"))
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();

        assert_eq!(ids, ["t1", "t2", "t3"]);
    }

    #[test]
    fn all_lines_come_oldest_first_then_by_statement_page_and_row() {
        let (_dir, db) = open_with_accounts();
        assert_eq!(db.read(all_transactions).unwrap(), vec![]);
        insert_all(
            &db,
            STATEMENTS,
            &[
                statement_row("a", "account-1", 1, 2026),
                statement_row("b", "account-1", 2, 2026),
            ],
        );
        let on = |id: &str, statement: &str, page: u32, row: u32, date: &str| TransactionRow {
            value_date: date.into(),
            ..transaction_row(id, statement, page, row)
        };
        insert_all(
            &db,
            TRANSACTIONS,
            &[
                on("late", "b", 1, 1, "2026-02-03"),
                on("a-page-2", "a", 2, 1, "2026-01-05"),
                on("b-same-day", "b", 1, 3, "2026-01-05"),
                on("a-row-2", "a", 1, 2, "2026-01-05"),
                on("early", "b", 1, 2, "2026-01-02"),
                on("a-row-1", "a", 1, 1, "2026-01-05"),
            ],
        );

        let ids: Vec<String> = db
            .read(all_transactions)
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();

        assert_eq!(
            ids,
            [
                "early",
                "a-row-1",
                "a-row-2",
                "a-page-2",
                "b-same-day",
                "late"
            ]
        );
    }

    #[test]
    fn a_transaction_row_round_trips_with_every_optional_field() {
        let (_dir, db) = open_with_accounts();
        insert_all(&db, STATEMENTS, &[statement_row("a", "account-1", 1, 2026)]);
        let full = transaction_row("full", "a", 1, 1);
        let bare = TransactionRow {
            id: "bare".into(),
            row: 2,
            counterparty: None,
            counterparty_iban: None,
            card_kind: None,
            card_date: None,
            card_time: None,
            merchant: None,
            original_cents: None,
            original_currency: None,
            fx_fee: None,
            fx_rate: None,
            fx_date: None,
            transfer: false,
            ..full.clone()
        };
        insert_all(&db, TRANSACTIONS, &[full.clone(), bare.clone()]);

        assert_eq!(
            db.read(|c| transactions_of(c, "a")).unwrap(),
            vec![full, bare]
        );
    }
}
