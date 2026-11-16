use rusqlite::Connection;

use super::years::shown;
use crate::db::migrations::migrate;

const THIS_YEAR: i32 = 2026;

fn empty() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
    conn
}

fn statement(from: &str, to: &str, hash: &str) -> String {
    format!(
        "INSERT INTO statements (id, account_id, number, year, from_date, to_date, old_balance, credits,
            debits, new_balance, layout, file_hash, file, pages, imported_at)
         VALUES ('{hash}', 'account-1', 1, 2000, '{from}', '{to}', 0, 0, 0, 0, 'demo', '{hash}', 'a.pdf', 1, 0)"
    )
}

fn transaction(value_date: &str) -> String {
    format!(
        "INSERT INTO transactions (id, statement_id, account_id, page, row, value_date, cents, text, extra,
            transfer)
         VALUES ('t-{value_date}', 's', 'account-1', 1, 1, '{value_date}', -100, 'Entgelt', '[]', 0)"
    )
}

fn document(pay_date: &str) -> String {
    let value = if pay_date.is_empty() {
        "NULL".to_string()
    } else {
        format!("'{pay_date}'")
    };
    format!(
        "INSERT INTO documents (id, state, file, original_name, file_hash, pay_date, work_share, note,
            created_at)
         VALUES ('d-{pay_date}', 'open', 'a.pdf', 'a.pdf', 'd-{pay_date}', {value}, 100, '', 0)"
    )
}

fn other(date: &str) -> String {
    let value = if date.is_empty() {
        "NULL".to_string()
    } else {
        format!("'{date}'")
    };
    format!(
        "INSERT INTO others (id, title, date, file, file_hash, created_at)
         VALUES ('o-{date}', 'Contract', {value}, 'a.pdf', 'o-{date}', 0)"
    )
}

fn entry(year: i32) -> String {
    format!(
        "INSERT INTO entries (id, year, item_key, kind, date, text, created_at)
         VALUES ('e-{year}', {year}, 'item', 'amount', '{year}-03-01', 'Desk', 0)"
    )
}

fn payslip(year: i32) -> String {
    format!("INSERT INTO payslips (id, year, month, created_at) VALUES ('p-{year}', {year}, 1, 0)")
}

fn tax_year(year: i32) -> String {
    format!(
        "INSERT INTO tax_years (id, year, state, ruleset, rules, rules_loaded_at)
         VALUES ('y-{year}', {year}, 'open', 'at', '{{}}', 0)"
    )
}

#[test]
fn an_empty_database_shows_only_this_year() {
    assert_eq!(shown(&empty(), THIS_YEAR).unwrap(), [THIS_YEAR]);
}

#[test]
fn every_kind_of_data_adds_the_year_it_carries() {
    let cases = [
        ("tax year", tax_year(2019)),
        ("statement", statement("2019-03-01", "2019-03-31", "s1")),
        ("transaction", transaction("2019-05-17")),
        ("document pay date", document("2019-06-02")),
        ("entry", entry(2019)),
        ("payslip", payslip(2019)),
        ("other with a date", other("2019-09-30")),
    ];
    for (kind, sql) in cases {
        let conn = empty();
        conn.execute_batch(&sql).unwrap();

        assert_eq!(
            shown(&conn, THIS_YEAR).unwrap(),
            [2019, THIS_YEAR],
            "{kind}"
        );
    }
}

#[test]
fn a_statement_spanning_a_year_boundary_adds_both_years() {
    let conn = empty();
    conn.execute_batch(&statement("2023-12-20", "2024-01-19", "s1"))
        .unwrap();

    assert_eq!(shown(&conn, THIS_YEAR).unwrap(), [2023, 2024, THIS_YEAR]);
}

#[test]
fn documents_without_a_pay_date_and_others_without_a_date_add_no_year() {
    let conn = empty();
    conn.execute_batch(&document("")).unwrap();
    conn.execute_batch(&other("")).unwrap();

    assert_eq!(shown(&conn, THIS_YEAR).unwrap(), [THIS_YEAR]);
}

#[test]
fn years_come_oldest_first_without_duplicates_and_this_year_stays_in_place() {
    let conn = empty();
    for sql in [
        tax_year(2027),
        entry(2021),
        payslip(2021),
        statement("2024-01-01", "2024-01-31", "s1"),
        transaction("2024-01-15"),
        other("2022-02-02"),
    ] {
        conn.execute_batch(&sql).unwrap();
    }

    assert_eq!(
        shown(&conn, THIS_YEAR).unwrap(),
        [2021, 2022, 2024, THIS_YEAR, 2027]
    );
}
