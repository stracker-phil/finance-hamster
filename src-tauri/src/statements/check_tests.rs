use super::check::{Chain, CheckError, Previous, chain, check};
use super::parse::model::ParsedStatement;
use super::parse::parse;
use crate::money::{Money, format_signed};
use crate::platform::pdf::Pdf;
use crate::settings::NumberFormat;
use crate::test_support::statements::{Statement, StatementLine, demo_month};

fn demo() -> Statement {
    demo_month(10, 2026, 10, 141_230)
}

fn long_statement() -> Statement {
    let mut statement = demo();
    for i in 0..30 {
        statement.lines.push(StatementLine {
            text: format!("Rate {i}"),
            extra: vec!["A".into(), "B".into()],
            value_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(),
            cents: -1_000,
        });
    }
    statement
}

fn euros(cents: i64) -> String {
    format_signed(&Money::eur(cents), NumberFormat::default())
}

fn read(statement: &Statement) -> ParsedStatement {
    parse(&statement.text_pages()).unwrap()
}

fn read_forged(statement: &Statement, line: usize, cents: i64) -> ParsedStatement {
    let pages = Pdf::from_bytes(&statement.tampered_pdf(line, cents))
        .unwrap()
        .text_pages()
        .unwrap();
    parse(&pages).unwrap()
}

fn with_line_cents(statement: &Statement, line: usize, cents: i64) -> Statement {
    let mut changed = statement.clone();
    changed.lines[line].cents = cents;
    changed
}

fn numbered(number: u32, year: i32, old_balance: i64) -> ParsedStatement {
    read(&demo_month(number, year, 10, old_balance))
}

fn previous(number: u32, year: i32, new_balance: i64) -> Previous {
    Previous {
        number,
        year,
        new_balance,
    }
}

#[test]
fn a_statement_whose_lines_and_sums_agree_passes() {
    assert_eq!(check(&read(&demo())), Ok(()));
    assert_eq!(check(&read(&long_statement())), Ok(()));
}

#[test]
fn a_changed_debit_with_the_original_sums_printed_fails_naming_both_sums() {
    for statement in [demo(), long_statement()] {
        let forged = read_forged(&statement, 1, -90_000);
        let changed = with_line_cents(&statement, 1, -90_000);
        assert_eq!(
            check(&forged),
            Err(CheckError::Debits {
                page: forged.sums_page,
                lines: euros(changed.debits()),
                printed: euros(statement.debits()),
            })
        );
    }
}

#[test]
fn a_changed_credit_with_the_original_sums_printed_fails_naming_both_sums() {
    let statement = demo();
    let forged = read_forged(&statement, 0, 280_000);
    let changed = with_line_cents(&statement, 0, 280_000);
    assert_eq!(
        check(&forged),
        Err(CheckError::Credits {
            page: forged.sums_page,
            lines: euros(changed.credits()),
            printed: euros(statement.credits()),
        })
    );
}

#[test]
fn the_failure_names_the_first_page_where_the_sums_are_printed() {
    let statement = long_statement();
    let forged = read_forged(&statement, 10, -2_000);
    assert!(statement.text_pages().len() >= 2);
    assert_eq!(forged.sums_page, 1);
    assert!(matches!(
        check(&forged),
        Err(CheckError::Debits { page: 1, .. })
    ));
}

#[test]
fn a_printed_new_balance_that_does_not_add_up_fails_with_the_difference() {
    let mut pages = demo().text_pages();
    let line = pages
        .iter_mut()
        .flatten()
        .find(|l| l.trim_start().starts_with("Neuer Kontostand") && l.contains("3.135,42"))
        .unwrap();
    *line = line.replace("3.135,42", "3.135,00");
    let parsed = parse(&pages).unwrap();
    assert_eq!(
        check(&parsed),
        Err(CheckError::Balance {
            page: 1,
            difference: euros(42),
        })
    );
}

#[test]
fn the_balance_is_checked_before_the_lines() {
    let mut parsed = read(&demo());
    parsed.credits += 100;
    assert!(matches!(check(&parsed), Err(CheckError::Balance { .. })));
}

#[test]
fn the_first_statement_of_an_account_starts_the_chain() {
    assert_eq!(chain(None, &numbered(1, 2026, 0)), Chain::First);
}

#[test]
fn a_statement_continues_the_chain_with_the_next_number_or_a_new_year() {
    for (prev, statement) in [
        (previous(9, 2026, 100), numbered(10, 2026, 100)),
        (previous(12, 2025, 500), numbered(1, 2026, 500)),
    ] {
        assert_eq!(chain(Some(&prev), &statement), Chain::Continues, "{prev:?}");
    }
}

#[test]
fn skipped_or_repeated_numbers_name_the_number_that_was_expected() {
    for (prev, statement, expected) in [
        (previous(9, 2026, 100), numbered(11, 2026, 100), 10),
        (previous(9, 2026, 100), numbered(10, 2025, 100), 1),
        (previous(10, 2026, 100), numbered(10, 2026, 100), 11),
        (previous(12, 2025, 500), numbered(2, 2026, 500), 1),
        (previous(9, 2026, 100), numbered(10, 2028, 100), 1),
    ] {
        assert_eq!(
            chain(Some(&prev), &statement),
            Chain::MissingNumbers { expected },
            "{prev:?} then {}/{}",
            statement.number,
            statement.year
        );
    }
}

#[test]
fn a_matching_number_with_a_different_old_balance_is_a_balance_gap() {
    for (prev, statement) in [
        (previous(9, 2026, 100), numbered(10, 2026, 200)),
        (previous(12, 2025, 500), numbered(1, 2026, 0)),
    ] {
        assert_eq!(
            chain(Some(&prev), &statement),
            Chain::BalanceGap {
                previous: prev.new_balance
            },
            "{prev:?}"
        );
    }
}

#[test]
fn missing_numbers_are_reported_before_a_balance_gap() {
    assert_eq!(
        chain(Some(&previous(9, 2026, 100)), &numbered(12, 2026, 999)),
        Chain::MissingNumbers { expected: 10 }
    );
}
