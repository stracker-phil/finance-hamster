use chrono::TimeZone;
use serde_json::json;

use super::documents::{self, AccountStatements, StatementFact, StatementListRow};
use super::import;
use super::import_tests::{october, open_with_accounts};
use super::store::{STATEMENTS, StatementRow};
use crate::date::DateOrder;
use crate::db::{Change, Db, Op, Source};
use crate::error::AppError;
use crate::i18n::with_lang;
use crate::settings::{Language, NumberFormat};

const DOT: NumberFormat = NumberFormat::DotDecimal;
const DAY_FIRST: DateOrder = DateOrder::DayFirst;

fn row(id: &str, account_id: &str, number: u32, year: i32, from: &str, to: &str) -> StatementRow {
    StatementRow {
        id: id.into(),
        account_id: account_id.into(),
        number,
        year,
        from_date: from.into(),
        to_date: to.into(),
        old_balance: 100_000,
        credits: 50_000,
        debits: -20_000,
        new_balance: 130_000,
        layout: "sparkasse-erste-1".into(),
        file_hash: format!("hash-{id}"),
        file: format!("statements/{year}/{id}.pdf"),
        pages: 2,
        imported_at: 1_700_000_000_000,
    }
}

fn insert(db: &Db, rows: &[StatementRow]) {
    let changes = rows
        .iter()
        .map(|r| Change::insert(STATEMENTS, r).unwrap())
        .collect();
    db.write(Op::new("Add statements", changes), Source::Ui)
        .unwrap();
}

fn list(db: &Db, query: &str) -> Vec<AccountStatements> {
    documents::list(db, query, DOT, DAY_FIRST).unwrap()
}

fn titles(groups: &[AccountStatements]) -> Vec<(String, Vec<String>)> {
    groups
        .iter()
        .map(|g| {
            (
                g.name.clone(),
                g.statements.iter().map(|s| s.title.clone()).collect(),
            )
        })
        .collect()
}

fn three_statements() -> (tempfile::TempDir, Db) {
    let (dir, db) = open_with_accounts();
    insert(
        &db,
        &[
            row("s-a", "account-1", 12, 2025, "2025-11-03", "2025-12-30"),
            row("s-b", "account-1", 3, 2026, "2026-09-07", "2026-11-10"),
            row("s-c", "account-3", 1, 2026, "2026-01-02", "2026-02-27"),
        ],
    );
    (dir, db)
}

#[test]
fn no_statements_list_no_accounts() {
    let (_dir, db) = open_with_accounts();

    assert_eq!(list(&db, ""), vec![]);
}

#[test]
fn accounts_come_in_their_order_without_the_ones_that_have_no_statements() {
    let (_dir, db) = three_statements();

    let groups = list(&db, "");

    assert_eq!(
        groups
            .iter()
            .map(|g| (g.account_id.as_str(), g.name.as_str()))
            .collect::<Vec<_>>(),
        [("account-1", "Private"), ("account-3", "Business")]
    );
}

#[test]
fn an_accounts_statements_come_newest_first() {
    let (_dir, db) = three_statements();

    assert_eq!(
        titles(&list(&db, "")),
        [
            (
                "Private".to_string(),
                vec!["Nr. 003/2026".to_string(), "Nr. 012/2025".to_string()]
            ),
            ("Business".to_string(), vec!["Nr. 001/2026".to_string()]),
        ]
    );
}

#[test]
fn a_list_row_carries_title_period_closing_balance_and_line_count() {
    let (_dir, db) = three_statements();

    let rows = &list(&db, "")[0].statements;

    assert_eq!(
        rows[0],
        StatementListRow {
            id: "s-b".into(),
            title: "Nr. 003/2026".into(),
            period: "7 Sep 2026 – 10 Nov 2026".into(),
            closing: "€1,300.00".into(),
            lines: "0 lines".into(),
        }
    );
}

#[test]
fn the_line_count_comes_from_the_imported_transactions() {
    let (dir, db) = open_with_accounts();
    let pending = import::prepare(&db, dir.path(), october().pdf()).unwrap();
    import::commit(&db, dir.path(), pending, 0, Source::Import).unwrap();

    let english = list(&db, "");
    let german = with_lang(Language::De, || list(&db, ""));

    assert_eq!(english[0].statements[0].lines, "7 lines");
    assert_eq!(german[0].statements[0].lines, "7 Buchungen");
    assert_eq!(english[0].statements[0].closing, "€3,135.42");
}

#[test]
fn the_period_follows_the_date_order_and_the_closing_balance_the_number_format() {
    let (_dir, db) = three_statements();

    let rows = documents::list(&db, "", NumberFormat::CommaDecimal, DateOrder::MonthFirst)
        .unwrap()
        .remove(0)
        .statements;

    assert_eq!(rows[0].period, "Sep 7, 2026 – Nov 10, 2026");
    assert_eq!(rows[0].closing, "€1.300,00");
}

#[test]
fn a_query_matches_the_number_the_period_and_the_account_name_ignoring_case() {
    let (_dir, db) = three_statements();

    for (query, expected) in [
        ("003", vec![("Private", vec!["Nr. 003/2026"])]),
        ("nr. 012", vec![("Private", vec!["Nr. 012/2025"])]),
        ("7 sep 2026", vec![("Private", vec!["Nr. 003/2026"])]),
        ("feb 2026", vec![("Business", vec!["Nr. 001/2026"])]),
        ("  BUSINESS ", vec![("Business", vec!["Nr. 001/2026"])]),
        (
            "private",
            vec![("Private", vec!["Nr. 003/2026", "Nr. 012/2025"])],
        ),
        ("nothing like it", vec![]),
    ] {
        let expected: Vec<(String, Vec<String>)> = expected
            .into_iter()
            .map(|(name, t)| (name.to_string(), t.into_iter().map(String::from).collect()))
            .collect();
        assert_eq!(titles(&list(&db, query)), expected, "{query}");
    }
}

#[test]
fn the_list_serializes_with_camel_case_names() {
    let (_dir, db) = three_statements();

    let value = serde_json::to_value(&list(&db, "business")).unwrap();

    assert_eq!(
        value,
        json!([{
            "accountId": "account-3",
            "name": "Business",
            "statements": [{
                "id": "s-c",
                "title": "Nr. 001/2026",
                "period": "2 Jan 2026 – 27 Feb 2026",
                "closing": "€1,300.00",
                "lines": "0 lines",
            }],
        }])
    );
}

fn imported_midday(db: &Db, dir: &std::path::Path) -> String {
    let at = chrono::Local
        .with_ymd_and_hms(2026, 11, 3, 12, 0, 0)
        .unwrap()
        .timestamp_millis();
    let pending = import::prepare(db, dir, october().pdf()).unwrap();
    import::commit(db, dir, pending, at, Source::Import)
        .unwrap()
        .1
        .id
}

fn fact(label: &str, value: &str) -> StatementFact {
    StatementFact {
        label: label.into(),
        value: value.into(),
    }
}

#[test]
fn a_statement_view_names_the_account_and_number_and_lists_ten_facts_in_order() {
    let (dir, db) = open_with_accounts();
    let id = imported_midday(&db, dir.path());

    let view = documents::view(&db, &id, DOT, DAY_FIRST).unwrap();

    assert_eq!(view.id, id);
    assert_eq!(view.account_id, "account-1");
    assert_eq!(view.title, "Private · Nr. 010/2026");
    assert_eq!(
        view.facts,
        vec![
            fact("Account", "Private"),
            fact("Number", "Nr. 010/2026"),
            fact("Period", "1 Oct 2026 – 21 Oct 2026"),
            fact("Opening balance", "€1,412.30"),
            fact("Credits", "€2,850.00"),
            fact("Debits", "\u{2212}€1,126.88"),
            fact("Closing balance", "€3,135.42"),
            fact("Lines", "7"),
            fact("Pages", "1"),
            fact("Imported", "3 Nov 2026"),
        ]
    );
}

#[test]
fn the_german_view_has_german_labels_and_dates() {
    let (dir, db) = open_with_accounts();
    let id = imported_midday(&db, dir.path());

    let view = with_lang(Language::De, || {
        documents::view(&db, &id, NumberFormat::CommaDecimal, DAY_FIRST)
    })
    .unwrap();

    let labels: Vec<&str> = view.facts.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Konto",
            "Nummer",
            "Zeitraum",
            "Alter Saldo",
            "Gutschriften",
            "Belastungen",
            "Neuer Saldo",
            "Buchungen",
            "Seiten",
            "Importiert",
        ]
    );
    assert_eq!(view.facts[3].value, "€1.412,30");
    assert_eq!(view.facts[9].value, "3. Nov. 2026");
}

#[test]
fn a_statement_view_serializes_with_camel_case_names() {
    let (_dir, db) = three_statements();

    let value = serde_json::to_value(documents::view(&db, "s-b", DOT, DAY_FIRST).unwrap()).unwrap();

    assert_eq!(value["accountId"], "account-1");
    assert_eq!(
        value["facts"][0],
        json!({ "label": "Account", "value": "Private" })
    );
}

#[test]
fn an_unknown_statement_is_not_found() {
    let (_dir, db) = open_with_accounts();

    let err = documents::view(&db, "nope", DOT, DAY_FIRST).unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}
