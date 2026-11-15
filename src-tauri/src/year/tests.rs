//! The tax year page and its answers against a temp database with the demo accounts.

use serde::Serialize;
use serde_json::{Value, json};
use tempfile::TempDir;

use super::answers::{self, answer};
use super::page::{AttentionKind, AttentionView, RowKind, YearPage, YearRow, page};
use crate::date::DateOrder;
use crate::db::{Change, Db, Op, Source, Written};
use crate::documents::store::{self as documents, DocState, DocumentRow, LinkRow};
use crate::error::AppError;
use crate::rules::store::{self as decisions, Action, DecisionRow};
use crate::settings::NumberFormat;
use crate::statements::file_hash;
use crate::statements::import_tests::open_with_accounts;
use crate::statements::store::{self as statements, StatementRow, TransactionRow};
use crate::taxrules::model::Ruleset;
use crate::taxyears::{self, TaxYearRow, YearState};
use crate::test_support::rules::{minimal_rules, rules_text, set};

const NOW: i64 = 1_700_000_000;
const HOME_DAYS: &str = "Telearbeitstage (Homeoffice days)";

/// 2026 rules: books (cap 300), phone (40 % private), tips and parking (no document needed), a hidden item,
/// an item reported by others, two questions and a Werbungskostenpauschale of 132.00.
pub(crate) fn rules() -> Value {
    let mut rules = minimal_rules(2026);
    set(
        &mut rules,
        "/settings/werbungskostenpauschale",
        json!(132.0),
    );
    set(
        &mut rules,
        "/questions",
        json!([
            {
                "id": "telearbeitstage", "label": HOME_DAYS, "help": "Days worked from home",
                "type": "integer", "unit": "days"
            },
            { "id": "pauschale", "label": "Telearbeitspauschale (from the employer)", "type": "amount" }
        ]),
    );
    set(
        &mut rules,
        "/items",
        json!([
            {
                "key": "books", "kz": "720", "name": "Books", "section": "main", "kind": "documents",
                "estimate": [{ "type": "cap", "amount": 300.0 }]
            },
            {
                "key": "phone", "kz": "721", "name": "Phone", "section": "main", "kind": "documents",
                "privateShare": 40
            },
            {
                "key": "tips", "kz": "722", "name": "Tips", "section": "main", "kind": "amount",
                "evidence": "none", "privateShare": 20,
                "estimate": [{ "type": "requires", "question": "telearbeitstage", "min": 26 }]
            },
            {
                "key": "parking", "kz": "724", "name": "Parking", "section": "main", "kind": "trips",
                "evidence": "optional"
            },
            {
                "key": "secret", "kz": "725", "name": "Secret", "section": "main", "kind": "documents",
                "visible": false
            },
            {
                "key": "employer", "kz": "726", "name": "Employer", "section": "main",
                "kind": "reportedByOthers"
            }
        ]),
    );
    rules
}

pub(crate) fn ruleset() -> Ruleset {
    serde_json::from_value(rules()).unwrap()
}

pub(crate) fn insert(db: &Db, table: &str, row: &impl Serialize) {
    db.write(
        Op::new("Insert", vec![Change::insert(table, row).unwrap()]),
        Source::Seed,
    )
    .unwrap();
}

fn year_row(state: YearState, rules: &Value) -> TaxYearRow {
    TaxYearRow {
        state,
        closed_at: (state == YearState::Closed).then_some(2),
        ..TaxYearRow::new(2026, "2026 v1".into(), rules_text(rules), 1)
    }
}

/// The demo accounts and an open 2026 year with `rules`.
pub(crate) fn open_year_with(rules: &Value) -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    insert(&db, taxyears::TABLE, &year_row(YearState::Open, rules));
    (dir, db)
}

pub(crate) fn open_year() -> (TempDir, Db) {
    open_year_with(&rules())
}

pub(crate) fn document(id: &str, patch: impl FnOnce(&mut DocumentRow)) -> DocumentRow {
    let mut doc = DocumentRow {
        id: id.into(),
        state: DocState::Finished,
        file: format!("docs/inbox/{id}.pdf"),
        original_name: format!("{id}.pdf"),
        file_hash: file_hash(id.as_bytes()),
        vendor_id: None,
        pay_date: None,
        paid_cents: None,
        item_key: None,
        work_share: 100,
        note: String::new(),
        invoice_cents: None,
        invoice_currency: None,
        invoice_number: None,
        invoice_date: None,
        created_at: 1,
        finished_at: None,
        excluded_at: None,
        exclude_reason: None,
        useful_life: None,
    };
    patch(&mut doc);
    doc
}

pub(crate) fn add_document(db: &Db, id: &str, patch: impl FnOnce(&mut DocumentRow)) {
    insert(db, documents::DOCUMENTS, &document(id, patch));
}

/// A finished document of `item` paid on `date` for `cents` at `share` percent.
fn finished(db: &Db, id: &str, item: &str, date: &str, cents: i64, share: i64) {
    add_document(db, id, |d| {
        d.item_key = Some(item.into());
        d.pay_date = Some(date.into());
        d.paid_cents = Some(cents);
        d.work_share = share;
    });
}

pub(crate) fn debit(id: &str, date: &str, cents: i64, text: &str) -> TransactionRow {
    TransactionRow {
        id: id.into(),
        statement_id: "statement-1".into(),
        account_id: "account-1".into(),
        page: 1,
        row: 1,
        value_date: date.into(),
        cents,
        text: text.into(),
        extra: "[]".into(),
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
    }
}

/// The lines as an imported statement of `account_id`.
pub(crate) fn add_statement(
    db: &Db,
    statement_id: &str,
    account_id: &str,
    number: u32,
    lines: &[TransactionRow],
) {
    insert(
        db,
        statements::STATEMENTS,
        &StatementRow {
            id: statement_id.into(),
            account_id: account_id.into(),
            number,
            year: 2026,
            from_date: "2026-01-01".into(),
            to_date: "2026-12-31".into(),
            old_balance: 0,
            credits: 0,
            debits: 0,
            new_balance: 0,
            layout: "sparkasse".into(),
            file_hash: format!("hash-{statement_id}"),
            file: format!("statements/2026/{statement_id}.pdf"),
            pages: 1,
            imported_at: 1,
        },
    );
    for line in lines {
        insert(db, statements::TRANSACTIONS, line);
    }
}

fn add_lines(db: &Db, lines: &[TransactionRow]) {
    add_statement(db, "statement-1", "account-1", 1, lines);
}

fn decide(db: &Db, line_id: &str, kind: Action, item: Option<&str>) {
    insert(
        db,
        decisions::DECISIONS,
        &DecisionRow {
            id: line_id.into(),
            kind,
            item_key: item.map(String::from),
            subscription_id: None,
            document_id: None,
            rule_id: None,
            decided_at: 1,
        },
    );
}

fn link(db: &Db, document_id: &str, line_id: &str) {
    insert(
        db,
        documents::LINKS,
        &LinkRow {
            id: format!("link-{document_id}-{line_id}"),
            document_id: document_id.into(),
            transaction_id: line_id.into(),
            created_at: 1,
        },
    );
}

fn page_matching(db: &Db, query: &str) -> YearPage {
    page(
        db,
        2026,
        query,
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap()
}

fn year_page(db: &Db) -> YearPage {
    page_matching(db, "")
}

fn answer_with(db: &Db, question: &str, text: &str) -> Option<Written> {
    answer(db, &ruleset(), 2026, question, text, NOW, Source::Ui).unwrap()
}

fn kennzahlen(page: &YearPage) -> Vec<(&str, &str)> {
    page.kennzahlen
        .iter()
        .map(|k| (k.kz.as_str(), k.copy.as_str()))
        .collect()
}

fn rows_of<'a>(page: &'a YearPage, kz: &str) -> &'a [YearRow] {
    &page
        .kennzahlen
        .iter()
        .find(|k| k.kz == kz)
        .unwrap_or_else(|| panic!("no Kennzahl {kz}"))
        .rows
}

fn amounts(rows: &[YearRow]) -> Vec<&str> {
    rows.iter().map(|r| r.amount.as_str()).collect()
}

/// Four counted entries: two documents (one at 60 % work share) and two bank lines on items without documents.
fn counted_entries() -> (TempDir, Db) {
    let (dir, db) = open_year();
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
    finished(&db, "doc-b", "phone", "2026-11-02", 12_000, 60);
    finished(&db, "doc-old", "books", "2025-05-01", 5_000, 100);
    add_lines(
        &db,
        &[
            debit("l-tips", "2026-09-09", -5_000, "Lastschrift COPYSHOP"),
            debit("l-park", "2026-08-20", -2_500, "Parkhaus Mitte"),
        ],
    );
    decide(&db, "l-tips", Action::Anv, Some("tips"));
    decide(&db, "l-park", Action::Anv, Some("parking"));
    (dir, db)
}

#[test]
fn answers_start_empty_and_are_kept_per_year() {
    let (_dir, db) = open_year();
    assert!(answers::answers(&db, 2026).unwrap().is_empty());

    answer_with(&db, "telearbeitstage", "28");

    assert_eq!(
        answers::answers(&db, 2026).unwrap(),
        [("telearbeitstage".to_string(), 28)].into()
    );
    assert!(answers::answers(&db, 2025).unwrap().is_empty());
}

#[test]
fn answering_an_unknown_question_is_not_found() {
    let (_dir, db) = open_year();

    let err = answer(&db, &ruleset(), 2026, "nope", "1", NOW, Source::Ui).unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}

#[test]
fn an_answer_is_written_as_an_op_labelled_with_the_question() {
    let (_dir, db) = open_year();

    let written = answer_with(&db, "telearbeitstage", "28");

    assert_eq!(
        written,
        Some(Written {
            label: format!("Answer {HOME_DAYS}"),
            tables: vec!["answers".into()],
        })
    );
}

#[test]
fn an_integer_answer_is_a_whole_number_of_at_least_zero() {
    for (text, expected) in [("28", 28), (" 28 ", 28), ("0", 0), ("365", 365)] {
        let (_dir, db) = open_year();

        answer_with(&db, "telearbeitstage", text);

        assert_eq!(
            answers::answers(&db, 2026).unwrap()["telearbeitstage"],
            expected,
            "{text:?}"
        );
    }
}

#[test]
fn an_amount_answer_is_stored_as_cents_without_its_sign() {
    for (text, expected) in [
        ("120", 12_000),
        ("1,234.56", 123_456),
        ("1.234,56", 123_456),
        ("€ 99,5", 9_950),
        ("-50", 5_000),
        ("0", 0),
    ] {
        let (_dir, db) = open_year();

        answer_with(&db, "pauschale", text);

        assert_eq!(
            answers::answers(&db, 2026).unwrap()["pauschale"],
            expected,
            "{text:?}"
        );
    }
}

#[test]
fn an_answer_that_is_not_a_number_or_an_amount_names_its_field_and_stores_nothing() {
    for (question, text, message) in [
        ("telearbeitstage", "abc", "A whole number"),
        ("telearbeitstage", "-1", "A whole number"),
        ("telearbeitstage", "2.5", "A whole number"),
        ("telearbeitstage", "12 days", "A whole number"),
        ("pauschale", "abc", "This isn't an amount"),
        ("pauschale", "12 EUR", "This isn't an amount"),
        ("pauschale", "1,2,3", "This isn't an amount"),
    ] {
        let (_dir, db) = open_year();

        let err = answer(&db, &ruleset(), 2026, question, text, NOW, Source::Ui).unwrap_err();

        match err {
            AppError::InvalidField {
                field,
                message: shown,
            } => {
                assert_eq!(field, format!("answer.{question}"), "{text:?}");
                assert_eq!(shown, message, "{text:?}");
            }
            other => panic!("expected an invalid field for {text:?}, got {other:?}"),
        }
        assert!(answers::answers(&db, 2026).unwrap().is_empty(), "{text:?}");
    }
}

#[test]
fn a_changed_answer_replaces_the_old_one() {
    let (_dir, db) = open_year();
    answer_with(&db, "telearbeitstage", "28");

    let written = answer_with(&db, "telearbeitstage", "31");

    assert!(written.is_some());
    assert_eq!(
        answers::answers(&db, 2026).unwrap(),
        [("telearbeitstage".to_string(), 31)].into()
    );
}

#[test]
fn the_same_value_again_changes_nothing() {
    let (_dir, db) = open_year();
    answer_with(&db, "telearbeitstage", "28");

    assert_eq!(answer_with(&db, "telearbeitstage", "28"), None);
    assert_eq!(answer_with(&db, "telearbeitstage", " 28 "), None);

    answer_with(&db, "pauschale", "120");
    assert_eq!(answer_with(&db, "pauschale", "-120,00"), None);
}

#[test]
fn a_blank_answer_takes_an_existing_one_back_and_is_nothing_without_one() {
    let (_dir, db) = open_year();
    assert_eq!(answer_with(&db, "telearbeitstage", ""), None);
    answer_with(&db, "telearbeitstage", "28");

    let written = answer_with(&db, "telearbeitstage", "   ");

    assert_eq!(
        written,
        Some(Written {
            label: format!("Answer {HOME_DAYS}"),
            tables: vec!["answers".into()],
        })
    );
    assert!(answers::answers(&db, 2026).unwrap().is_empty());
    assert_eq!(answer_with(&db, "telearbeitstage", ""), None);
}

#[test]
fn undo_takes_back_an_answer_a_change_and_a_removal() {
    let (_dir, db) = open_year();
    let stored = || answers::answers(&db, 2026).unwrap();
    answer_with(&db, "telearbeitstage", "28");
    answer_with(&db, "telearbeitstage", "31");
    answer_with(&db, "telearbeitstage", "");
    assert!(stored().is_empty());

    db.undo().unwrap().unwrap();
    assert_eq!(stored()["telearbeitstage"], 31);

    db.undo().unwrap().unwrap();
    assert_eq!(stored()["telearbeitstage"], 28);

    db.undo().unwrap().unwrap();
    assert!(stored().is_empty());
}

#[test]
fn a_year_that_does_not_exist_is_not_found() {
    let (_dir, db) = open_year();

    let err = page(&db, 2031, "", NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}

#[test]
fn the_year_page_shows_german_question_texts_in_german_and_english_ones_otherwise() {
    let mut german = rules();
    set(
        &mut german,
        "/questions/0/de",
        json!({ "label": "Homeoffice-Tage", "help": "Tage im Homeoffice", "unit": "Tage" }),
    );
    let (_dir, db) = open_year_with(&german);

    let shown = crate::i18n::with_lang(crate::settings::Language::De, || year_page(&db));
    let questions: Vec<(&str, Option<&str>, Option<&str>)> = shown
        .questions
        .iter()
        .map(|q| (q.label.as_str(), q.help.as_deref(), q.unit.as_deref()))
        .collect();
    assert_eq!(
        questions,
        [
            ("Homeoffice-Tage", Some("Tage im Homeoffice"), Some("Tage")),
            ("Telearbeitspauschale (from the employer)", None, None)
        ]
    );

    let english = crate::i18n::with_lang(crate::settings::Language::En, || year_page(&db));
    assert_eq!(english.questions[0].label, HOME_DAYS);
    assert_eq!(
        english.questions[0].help.as_deref(),
        Some("Days worked from home")
    );
    assert_eq!(english.questions[0].unit.as_deref(), Some("days"));
}

#[test]
fn a_year_without_entries_is_all_zero_and_asks_for_its_answers() {
    let (_dir, db) = open_year();

    let page = year_page(&db);

    assert_eq!(page.year, 2026);
    assert_eq!(page.ruleset, "2026 v1");
    assert_eq!(page.state, YearState::Open);
    assert_eq!(page.total, "€0.00");
    assert_eq!(page.progress, "2 answers missing");
    assert_eq!(page.progress_percent, 0);
    assert_eq!(page.archived, 0);
    assert!(page.attention.is_empty());
    assert!(page.months.is_empty());
    assert_eq!(
        kennzahlen(&page),
        [
            ("720", "0,00"),
            ("721", "0,00"),
            ("722", "0,00"),
            ("724", "0,00"),
            ("726", "0,00")
        ]
    );
    let notes: Vec<(&str, bool)> = page
        .kennzahlen
        .iter()
        .map(|k| (k.kz.as_str(), k.note.is_some()))
        .collect();
    assert_eq!(
        notes,
        [
            ("720", false),
            ("721", false),
            ("722", false),
            ("724", false),
            ("726", true)
        ]
    );
}

#[test]
fn the_total_is_the_sum_of_what_every_entry_counts() {
    let (_dir, db) = counted_entries();

    let page = year_page(&db);

    assert_eq!(page.total, "€183.90");
    assert_eq!(page.pauschale_note, None);
}

#[test]
fn a_document_counts_what_was_paid_times_its_work_share_and_other_years_stay_out() {
    let (_dir, db) = counted_entries();

    let page = year_page(&db);

    assert_eq!(
        rows_of(&page, "720"),
        [YearRow {
            id: "doc-a".into(),
            kind: RowKind::Document,
            title: "doc-a".into(),
            detail: "14 Oct".into(),
            amount: "€46.90".into(),
            draft: false,
            carried_from: None,
            kz: "720".into(),
            attach: false,
        }]
    );
    assert_eq!(
        rows_of(&page, "721"),
        [YearRow {
            id: "doc-b".into(),
            kind: RowKind::Document,
            title: "doc-b".into(),
            detail: "2 Nov · 60 % of €120.00".into(),
            amount: "€72.00".into(),
            draft: false,
            carried_from: None,
            kz: "721".into(),
            attach: false,
        }]
    );
}

#[test]
fn an_anv_bank_line_on_an_item_without_a_document_counts_at_the_items_default_work_share() {
    let (_dir, db) = counted_entries();

    let page = year_page(&db);

    assert_eq!(
        rows_of(&page, "722"),
        [YearRow {
            id: "l-tips".into(),
            kind: RowKind::Line,
            title: "Lastschrift COPYSHOP".into(),
            detail: "9 Sep · bank line".into(),
            amount: "€40.00".into(),
            draft: false,
            carried_from: None,
            kz: "722".into(),
            attach: false,
        }]
    );
    assert_eq!(amounts(rows_of(&page, "724")), ["€25.00"]);
}

#[test]
fn a_subscription_charge_counts_on_the_item_of_its_subscription_and_one_without_an_item_does_not() {
    use crate::test_support::subscriptions::{insert_rows, plan_row, subscription_row};
    let (_dir, db) = open_year();
    let mut with_item = subscription_row("sub-copy", "Copy Club", "COPYSHOP");
    with_item.item_key = Some("tips".into());
    insert_rows(
        &db,
        &with_item,
        &[plan_row("plan-1", "sub-copy", 5_000, "2026-01-01", None)],
    );
    let without_item = subscription_row("sub-plain", "Plain Club", "PLAIN");
    insert_rows(
        &db,
        &without_item,
        &[plan_row("plan-2", "sub-plain", 1_000, "2026-01-01", None)],
    );
    add_lines(
        &db,
        &[
            debit("l-copy", "2026-09-09", -5_000, "Lastschrift COPYSHOP"),
            debit("l-plain", "2026-09-10", -1_000, "Lastschrift PLAIN"),
        ],
    );
    for (line, subscription) in [("l-copy", "sub-copy"), ("l-plain", "sub-plain")] {
        insert(
            &db,
            decisions::DECISIONS,
            &DecisionRow {
                id: line.into(),
                kind: Action::Subscription,
                item_key: None,
                subscription_id: Some(subscription.into()),
                document_id: None,
                rule_id: None,
                decided_at: 1,
            },
        );
    }

    let page = year_page(&db);

    assert_eq!(
        rows_of(&page, "722"),
        [YearRow {
            id: "l-copy".into(),
            kind: RowKind::Line,
            title: "Lastschrift COPYSHOP".into(),
            detail: "9 Sep · bank line".into(),
            amount: "€40.00".into(),
            draft: false,
            carried_from: None,
            kz: "722".into(),
            attach: false,
        }]
    );
}

#[test]
fn kennzahlen_follow_the_rules_file_with_a_copy_value_and_skip_hidden_items() {
    let (_dir, db) = counted_entries();
    finished(&db, "doc-h", "secret", "2026-02-02", 2_000, 100);

    let page = year_page(&db);

    assert_eq!(
        kennzahlen(&page),
        [
            ("720", "46,90"),
            ("721", "72,00"),
            ("722", "40,00"),
            ("724", "25,00"),
            ("726", "0,00")
        ]
    );
    let names: Vec<&str> = page.kennzahlen.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["Books", "Phone", "Tips", "Parking", "Employer"]);
    assert_eq!(page.total, "€183.90");
}

#[test]
fn a_large_total_is_copied_with_a_decimal_comma_and_no_grouping() {
    let (_dir, db) = open_year();
    finished(&db, "doc-big", "books", "2026-03-03", 122_840, 100);

    let page = year_page(&db);

    assert_eq!(rows_of(&page, "720")[0].amount, "€1,228.40");
    assert_eq!(kennzahlen(&page)[0], ("720", "1228,40"));
}

#[test]
fn a_kennzahl_carries_the_plain_label_the_examples_and_the_total_as_money() {
    let mut labelled = rules();
    set(&mut labelled, "/items/0/label", json!("Reading"));
    set(&mut labelled, "/items/0/examples", json!("Books, e-books"));
    let (_dir, db) = open_year_with(&labelled);
    finished(&db, "doc-big", "books", "2026-03-03", 122_840, 100);

    let page = year_page(&db);

    let books = &page.kennzahlen[0];
    assert_eq!(books.name, "Books");
    assert_eq!(books.label, "Reading");
    assert_eq!(books.examples.as_deref(), Some("Books, e-books"));
    assert_eq!(books.copy, "1228,40");
    assert_eq!(books.total, "€1,228.40");
}

#[test]
fn a_kennzahl_without_a_label_shows_its_name_and_without_examples_none() {
    let (_dir, db) = counted_entries();

    let page = year_page(&db);

    let shown: Vec<(&str, &str, Option<&str>, &str)> = page
        .kennzahlen
        .iter()
        .map(|k| {
            (
                k.name.as_str(),
                k.label.as_str(),
                k.examples.as_deref(),
                k.total.as_str(),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("Books", "Books", None, "€46.90"),
            ("Phone", "Phone", None, "€72.00"),
            ("Tips", "Tips", None, "€40.00"),
            ("Parking", "Parking", None, "€25.00"),
            ("Employer", "Employer", None, "€0.00")
        ]
    );
}

#[test]
fn an_old_ask_hint_stands_in_for_missing_examples_and_examples_win_when_present() {
    let mut older = rules();
    set(
        &mut older,
        "/items/0/ask",
        json!({ "order": 1, "question": "Did you buy books?", "hint": "Novels, manuals" }),
    );
    set(
        &mut older,
        "/items/1/ask",
        json!({ "order": 2, "question": "Did you buy a phone?", "hint": "Old hint" }),
    );
    set(&mut older, "/items/1/examples", json!("Phone, case"));
    let (_dir, db) = open_year_with(&older);

    let page = year_page(&db);

    assert_eq!(
        page.kennzahlen[0].examples.as_deref(),
        Some("Novels, manuals")
    );
    assert_eq!(page.kennzahlen[1].examples.as_deref(), Some("Phone, case"));
}

#[test]
fn a_kennzahl_reads_its_label_examples_and_hint_in_german_when_the_language_is_german() {
    let mut german = rules();
    set(&mut german, "/items/0/label", json!("Reading"));
    set(&mut german, "/items/0/examples", json!("Books"));
    set(
        &mut german,
        "/items/0/de",
        json!({ "name": "Bücher", "label": "Lesen", "examples": "Bücher, E-Books" }),
    );
    set(
        &mut german,
        "/items/1/ask",
        json!({
            "order": 1, "question": "Did you buy a phone?", "hint": "Phone",
            "de": { "hint": "Handy, Hülle" }
        }),
    );
    let (_dir, db) = open_year_with(&german);

    let page = crate::i18n::with_lang(crate::settings::Language::De, || year_page(&db));

    let books = &page.kennzahlen[0];
    assert_eq!(
        (
            books.name.as_str(),
            books.label.as_str(),
            books.examples.as_deref()
        ),
        ("Bücher", "Lesen", Some("Bücher, E-Books"))
    );
    assert_eq!(page.kennzahlen[1].examples.as_deref(), Some("Handy, Hülle"));
}

#[test]
fn a_kennzahl_serializes_its_label_examples_and_total_in_camel_case() {
    let (_dir, db) = open_year();

    let value = serde_json::to_value(&year_page(&db).kennzahlen[0]).unwrap();

    assert_eq!(value["label"], json!("Books"));
    assert_eq!(value["examples"], json!(null));
    assert_eq!(value["total"], json!("€0.00"));
}

#[test]
fn a_kennzahl_has_a_note_only_when_it_has_rows() {
    let (_dir, db) = counted_entries();

    let page = year_page(&db);

    let notes: Vec<Option<&str>> = page.kennzahlen.iter().map(|k| k.note.as_deref()).collect();
    assert_eq!(
        notes,
        [
            None,
            None,
            Some("Enter your Telearbeitstage first."),
            None,
            Some("Reported by others; shown for reference, never counted here")
        ]
    );
}

#[test]
fn notes_follow_the_answers_of_the_year() {
    let (_dir, db) = counted_entries();
    answer_with(&db, "telearbeitstage", "12");

    let page = year_page(&db);

    assert_eq!(
        page.kennzahlen[2].note.as_deref(),
        Some("Fewer than 26 Telearbeitstage: the tax office won't accept this.")
    );
}

#[test]
fn a_document_paid_over_several_payments_counts_only_its_lines_in_this_year() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-1", "2026-12-30", -3_000, "Rate 1"),
            debit("l-2", "2027-01-02", -2_000, "Rate 2"),
            debit("l-3", "2025-12-31", -1_000, "Rate 0"),
            debit("l-4", "2026-01-05", -2_000, "Rate 3"),
        ],
    );
    add_document(&db, "doc-c", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-12-30".into());
        d.paid_cents = Some(99_999);
    });
    link(&db, "doc-c", "l-1");
    link(&db, "doc-c", "l-2");
    add_document(&db, "doc-d", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2025-12-31".into());
    });
    link(&db, "doc-d", "l-3");
    link(&db, "doc-d", "l-4");

    let page = year_page(&db);

    assert_eq!(amounts(rows_of(&page, "720")), ["€30.00", "€20.00"]);
    assert_eq!(page.total, "€50.00");
}

#[test]
fn a_document_is_listed_under_the_month_of_its_first_payment_in_this_year() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-0", "2025-12-31", -1_000, "Rate 0"),
            debit("l-2", "2026-03-03", -1_000, "Rate 2"),
            debit("l-1", "2026-01-05", -2_000, "Rate 1"),
        ],
    );
    add_document(&db, "doc-d", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2025-12-31".into());
    });
    for line in ["l-0", "l-1", "l-2"] {
        link(&db, "doc-d", line);
    }

    let page = year_page(&db);

    let months: Vec<(&str, &str, Vec<&str>)> = page
        .months
        .iter()
        .map(|m| {
            (
                m.label.as_str(),
                m.total.as_str(),
                m.rows.iter().map(|r| r.detail.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(months, [("January", "€30.00", vec!["5 Jan"])]);
}

#[test]
fn a_document_without_lines_is_listed_under_its_payment_date() {
    let (_dir, db) = open_year();
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);

    let page = year_page(&db);

    assert_eq!(page.months.len(), 1);
    assert_eq!(page.months[0].label, "October");
}

#[test]
fn a_document_on_an_item_reported_by_others_is_never_placed() {
    let (_dir, db) = open_year();
    finished(&db, "doc-e", "employer", "2026-02-02", 2_000, 100);

    let page = year_page(&db);

    assert_eq!(page.total, "€0.00");
    assert!(page.months.is_empty());
    assert_eq!(
        kennzahlen(&page),
        [
            ("720", "0,00"),
            ("721", "0,00"),
            ("722", "0,00"),
            ("724", "0,00"),
            ("726", "0,00")
        ]
    );
    assert!(rows_of(&page, "726").is_empty());
}

#[test]
fn an_anv_line_decided_on_an_item_reported_by_others_is_never_placed() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[debit("l-e", "2026-02-02", -2_000, "Lastschrift EMPLOYER")],
    );
    decide(&db, "l-e", Action::Anv, Some("employer"));

    let page = year_page(&db);

    assert_eq!(page.total, "€0.00");
    assert!(page.months.is_empty());
    assert!(page.attention.is_empty());
    assert!(page.kennzahlen.iter().all(|k| k.rows.is_empty()));
    assert!(rows_of(&page, "726").is_empty());
}

#[test]
fn a_document_whose_linked_lines_are_all_in_other_years_does_not_belong_to_this_year() {
    let (_dir, db) = open_year();
    add_lines(&db, &[debit("l-3", "2025-12-31", -1_000, "Rate 0")]);
    add_document(&db, "doc-f", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
        d.paid_cents = Some(1_000);
    });
    link(&db, "doc-f", "l-3");

    let page = year_page(&db);

    assert!(page.attention.is_empty());
    assert!(rows_of(&page, "720").is_empty());
    assert!(page.months.is_empty());
    assert_eq!(page.total, "€0.00");
    assert_eq!(page.progress, "2 answers missing");
}

#[test]
fn a_document_paid_in_another_year_without_lines_is_not_part_of_this_one() {
    let (_dir, db) = open_year();
    finished(&db, "doc-2025", "books", "2025-05-01", 5_000, 100);
    finished(&db, "doc-2027", "books", "2027-01-15", 5_000, 100);

    let page = year_page(&db);

    assert_eq!(page.total, "€0.00");
    assert!(rows_of(&page, "720").is_empty());
    assert!(page.attention.is_empty());
}

#[test]
fn drafts_are_listed_under_attention_and_marked_in_their_rows() {
    let (_dir, db) = open_year();
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
    });
    add_document(&db, "doc-d2", |d| {
        d.state = DocState::Draft;
        d.pay_date = Some("2026-04-04".into());
        d.paid_cents = Some(1_000);
    });

    let page = year_page(&db);

    let mut attention: Vec<&AttentionView> = page.attention.iter().collect();
    attention.sort_by_key(|a| a.title.clone());
    let expected = |id: &str| AttentionView {
        kind: AttentionKind::Draft,
        title: id.into(),
        detail: "Finish it to lock its values for the tax year".into(),
        document_id: Some(id.into()),
        entry_id: None,
        line_id: None,
    };
    assert_eq!(attention, [&expected("doc-d1"), &expected("doc-d2")]);
    assert_eq!(
        rows_of(&page, "720"),
        [YearRow {
            id: "doc-d1".into(),
            kind: RowKind::Document,
            title: "doc-d1".into(),
            detail: "3 Mar · amount paid missing".into(),
            amount: "€0.00".into(),
            draft: true,
            carried_from: None,
            kz: "720".into(),
            attach: false,
        }]
    );
    assert_eq!(page.progress, "2 drafts to finish · 2 answers missing");
}

#[test]
fn a_document_without_a_payment_date_is_listed_as_undated_and_counts_nowhere() {
    let (_dir, db) = open_year();
    add_document(&db, "doc-u", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.paid_cents = Some(9_000);
    });

    let page = year_page(&db);

    assert_eq!(
        page.attention,
        [AttentionView {
            kind: AttentionKind::Undated,
            title: "doc-u".into(),
            detail: "Set its payment date to place it in a tax year".into(),
            document_id: Some("doc-u".into()),
            entry_id: None,
            line_id: None,
        }]
    );
    assert_eq!(page.total, "€0.00");
    assert_eq!(page.progress, "2 answers missing");
}

#[test]
fn an_anv_line_on_an_item_that_needs_an_invoice_is_missing_its_invoice_and_does_not_count() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-m", "2026-10-05", -5_830, "Lastschrift SEITENWEISE"),
            debit("l-linked", "2026-10-06", -1_000, "Lastschrift NORDBUCH"),
            debit("l-ign", "2026-10-07", -1_100, "Lastschrift A"),
            debit("l-sub", "2026-10-08", -1_200, "Lastschrift B"),
            debit("l-secret", "2026-10-09", -1_300, "Lastschrift C"),
            debit("l-old", "2025-12-31", -1_400, "Lastschrift D"),
            debit("l-open", "2026-10-10", -1_500, "Lastschrift E"),
        ],
    );
    decide(&db, "l-m", Action::Anv, Some("books"));
    decide(&db, "l-linked", Action::Anv, Some("books"));
    decide(&db, "l-ign", Action::Ignore, None);
    decide(&db, "l-sub", Action::Subscription, None);
    decide(&db, "l-secret", Action::Anv, Some("secret"));
    decide(&db, "l-old", Action::Anv, Some("books"));
    add_document(&db, "doc-a", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-10-06".into());
    });
    link(&db, "doc-a", "l-linked");

    let page = year_page(&db);

    assert_eq!(
        page.attention,
        [AttentionView {
            kind: AttentionKind::MissingInvoice,
            title: "Lastschrift SEITENWEISE · €58.30".into(),
            detail: "5 Oct · Books counts once the invoice is attached".into(),
            document_id: None,
            entry_id: None,
            line_id: Some("l-m".into()),
        }]
    );
    assert_eq!(amounts(rows_of(&page, "720")), ["€10.00"]);
    assert_eq!(page.total, "€10.00");
    assert_eq!(page.progress, "1 invoice and 2 answers missing");
}

#[test]
fn bank_line_details_read_in_german_when_the_language_is_german() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-m", "2026-01-05", -5_830, "Lastschrift SEITENWEISE"),
            debit("l-tips", "2026-01-10", -5_000, "Lastschrift COPYSHOP"),
        ],
    );
    decide(&db, "l-m", Action::Anv, Some("books"));
    decide(&db, "l-tips", Action::Anv, Some("tips"));

    let page = crate::i18n::with_lang(crate::settings::Language::De, || year_page(&db));

    assert_eq!(
        page.attention[0].detail,
        "5. Jan. · Books zählt, sobald die Rechnung angehängt ist"
    );
    assert_eq!(rows_of(&page, "722")[0].detail, "10. Jan. · Buchung");
}

#[test]
fn progress_names_drafts_then_missing_invoices_and_answers_with_the_share_done() {
    let (_dir, db) = open_year();
    for id in ["doc-d1", "doc-d2"] {
        add_document(&db, id, |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
            d.pay_date = Some("2026-03-03".into());
            d.paid_cents = Some(1_000);
        });
    }
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
    add_lines(
        &db,
        &[debit(
            "l-m",
            "2026-10-05",
            -5_830,
            "Lastschrift SEITENWEISE",
        )],
    );
    decide(&db, "l-m", Action::Anv, Some("books"));

    let page = year_page(&db);

    assert_eq!(
        page.progress,
        "2 drafts to finish · 1 invoice and 2 answers missing"
    );
    assert_eq!(page.progress_percent, 16);
}

#[test]
fn progress_uses_the_singular_for_one_draft_and_one_answer() {
    let (_dir, db) = open_year();
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
    });
    answer_with(&db, "telearbeitstage", "28");

    let page = year_page(&db);

    assert_eq!(page.progress, "1 draft to finish · 1 answer missing");
    assert_eq!(page.progress_percent, 33);
}

#[test]
fn progress_reads_in_german_with_the_verb_agreeing_in_number() {
    use crate::i18n::with_lang;
    use crate::settings::Language;
    let progress = |answered: bool| {
        let (_dir, db) = open_year();
        add_document(&db, "doc-d1", |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
            d.pay_date = Some("2026-03-03".into());
        });
        if answered {
            answer_with(&db, "telearbeitstage", "28");
        }
        with_lang(Language::De, || year_page(&db).progress)
    };

    assert_eq!(progress(true), "1 Entwurf fertigzumachen · 1 Antwort fehlt");
    assert_eq!(
        progress(false),
        "1 Entwurf fertigzumachen · 2 Antworten fehlen"
    );
}

#[test]
fn a_finished_year_is_ready_to_close_in_german() {
    let (_dir, db) = open_year();
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
    answer_with(&db, "telearbeitstage", "28");
    answer_with(&db, "pauschale", "120");

    let page = crate::i18n::with_lang(crate::settings::Language::De, || year_page(&db));

    assert_eq!(page.progress, "Bereit zum Abschließen");
}

#[test]
fn a_year_with_everything_finished_and_answered_is_ready_to_close() {
    let (_dir, db) = open_year();
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
    answer_with(&db, "telearbeitstage", "28");
    answer_with(&db, "pauschale", "120");

    let page = year_page(&db);

    assert_eq!(page.progress, "Ready to close");
    assert_eq!(page.progress_percent, 100);
}

#[test]
fn a_closed_year_says_closed_whatever_is_left() {
    let (_dir, db) = open_year();
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.pay_date = Some("2026-03-03".into());
    });
    db.write(
        Op::new(
            "Close year",
            vec![
                Change::update(
                    taxyears::TABLE,
                    &year_row(YearState::Open, &rules()),
                    &year_row(YearState::Closed, &rules()),
                )
                .unwrap(),
            ],
        ),
        Source::App,
    )
    .unwrap();

    let page = year_page(&db);

    assert_eq!(page.state, YearState::Closed);
    assert_eq!(page.progress, "Closed");
}

#[test]
fn months_group_the_rows_with_a_total_and_the_month_name() {
    let (_dir, db) = counted_entries();
    finished(&db, "doc-c", "books", "2026-10-30", 1_000, 100);

    let page = year_page(&db);

    let months: Vec<(&str, &str, Vec<&str>)> = page
        .months
        .iter()
        .map(|m| {
            (
                m.label.as_str(),
                m.total.as_str(),
                m.rows.iter().map(|r| r.id.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        months,
        [
            ("August", "€25.00", vec!["l-park"]),
            ("September", "€40.00", vec!["l-tips"]),
            ("October", "€56.90", vec!["doc-c", "doc-a"]),
            ("November", "€72.00", vec!["doc-b"]),
        ]
    );
}

#[test]
fn the_pauschale_note_shows_while_the_total_is_below_it() {
    for (paid, note) in [
        (
            13_199,
            Some("Below the €132.00 Werbungskostenpauschale, which the tax office applies anyway."),
        ),
        (13_200, None),
        (20_000, None),
    ] {
        let (_dir, db) = open_year();
        finished(&db, "doc-a", "books", "2026-10-14", paid, 100);

        let page = year_page(&db);

        assert_eq!(page.pauschale_note.as_deref(), note, "{paid}");
    }
}

/// Rules for `year` with a computer item (3 years) and a books item, spreading purchases above `gwg`.
fn depreciating_rules(year: i32, gwg: f64) -> Value {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/settings/gwgLimit", json!(gwg));
    set(&mut rules, "/usefulLife", json!({ "computer": 3 }));
    set(
        &mut rules,
        "/items",
        json!([
            {
                "key": "computer", "kz": "720", "name": "Computer", "section": "main",
                "kind": "documents", "depreciation": { "assetClass": "computer" }
            },
            { "key": "books", "kz": "721", "name": "Books", "section": "main", "kind": "documents" }
        ]),
    );
    rules
}

/// 2026 (GWG limit 400) to 2029 (GWG limit 2,000), each an open year with its own rules.
fn depreciation_years() -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    for (year, gwg) in [
        (2026, 400.0),
        (2027, 2_000.0),
        (2028, 2_000.0),
        (2029, 2_000.0),
    ] {
        insert(
            &db,
            taxyears::TABLE,
            &TaxYearRow::new(
                year,
                format!("{year} v1"),
                rules_text(&depreciating_rules(year, gwg)),
                1,
            ),
        );
    }
    (dir, db)
}

fn computer(db: &Db, id: &str, date: &str, cents: i64, patch: impl FnOnce(&mut DocumentRow)) {
    add_document(db, id, |d| {
        d.item_key = Some("computer".into());
        d.pay_date = Some(date.into());
        d.paid_cents = Some(cents);
        patch(d);
    });
}

fn page_of(db: &Db, year: i32) -> YearPage {
    page(db, year, "", NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap()
}

fn slice_row(
    id: &str,
    detail: &str,
    amount: &str,
    carried_from: Option<i32>,
    draft: bool,
) -> YearRow {
    YearRow {
        id: id.into(),
        kind: RowKind::Document,
        title: id.into(),
        detail: detail.into(),
        amount: amount.into(),
        draft,
        carried_from,
        kz: "720".into(),
        attach: false,
    }
}

#[test]
fn a_depreciated_purchase_counts_its_first_slice_in_the_purchase_year_dated_by_its_pay_date() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |_| {});

    let page = page_of(&db, 2026);

    assert_eq!(
        rows_of(&page, "720"),
        [slice_row(
            "pc",
            "10 Mar · year 1 of depreciating €1,500.00 over 3 years",
            "€500.00",
            None,
            false
        )]
    );
    assert_eq!(page.total, "€500.00");
    assert_eq!(kennzahlen(&page), [("720", "500,00"), ("721", "0,00")]);
    assert_eq!(page.months.len(), 1);
    assert_eq!(page.months[0].label, "March");
}

#[test]
fn later_years_count_the_next_slice_as_a_carried_row_listed_first_under_the_whole_year() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |_| {});
    add_document(&db, "book", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2027-02-05".into());
        d.paid_cents = Some(1_000);
    });

    let page = page_of(&db, 2027);

    assert_eq!(
        rows_of(&page, "720"),
        [slice_row(
            "pc",
            "From 2026 · year 2 of depreciating €1,500.00 over 3 years",
            "€500.00",
            Some(2026),
            false
        )]
    );
    let months: Vec<(&str, &str, Vec<&str>)> = page
        .months
        .iter()
        .map(|m| {
            (
                m.label.as_str(),
                m.total.as_str(),
                m.rows.iter().map(|r| r.id.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        months,
        [
            ("Whole year", "€500.00", vec!["pc"]),
            ("February", "€10.00", vec!["book"]),
        ]
    );
    assert_eq!(page.total, "€510.00");
}

#[test]
fn years_without_a_slice_show_nothing_of_the_purchase() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |_| {});

    let last = page_of(&db, 2028);
    let after = page_of(&db, 2029);

    assert_eq!(amounts(rows_of(&last, "720")), ["€500.00"]);
    assert!(rows_of(&after, "720").is_empty());
    assert!(after.months.is_empty());
    assert_eq!(after.total, "€0.00");
}

#[test]
fn a_purchase_in_the_second_half_counts_half_slices_and_runs_one_year_longer() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-09-01", 150_000, |_| {});

    let slices: Vec<(i32, Vec<String>)> = [2026, 2027, 2028, 2029]
        .into_iter()
        .map(|year| {
            let page = page_of(&db, year);
            (
                year,
                rows_of(&page, "720")
                    .iter()
                    .map(|r| r.amount.clone())
                    .collect(),
            )
        })
        .collect();

    assert_eq!(
        slices,
        [
            (2026, vec!["€250.00".to_string()]),
            (2027, vec!["€500.00".to_string()]),
            (2028, vec!["€500.00".to_string()]),
            (2029, vec!["€250.00".to_string()]),
        ]
    );
    assert_eq!(
        rows_of(&page_of(&db, 2029), "720")[0].detail,
        "From 2026 · year 4 of depreciating €1,500.00 over 3 years"
    );
}

#[test]
fn the_documents_own_useful_life_sets_how_many_years_it_spreads_over() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |d| {
        d.useful_life = Some(2)
    });

    let first = page_of(&db, 2026);
    let second = page_of(&db, 2027);
    let third = page_of(&db, 2028);

    assert_eq!(
        rows_of(&first, "720")[0].detail,
        "10 Mar · year 1 of depreciating €1,500.00 over 2 years"
    );
    assert_eq!(amounts(rows_of(&first, "720")), ["€750.00"]);
    assert_eq!(amounts(rows_of(&second, "720")), ["€750.00"]);
    assert!(rows_of(&third, "720").is_empty());
}

#[test]
fn a_draft_needs_attention_only_in_the_purchase_year_but_its_slices_stay_marked_as_drafts() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |d| {
        d.state = DocState::Draft
    });

    let purchase = page_of(&db, 2026);
    let later = page_of(&db, 2027);

    assert_eq!(
        purchase.attention,
        [AttentionView {
            kind: AttentionKind::Draft,
            title: "pc".into(),
            detail: "Finish it to lock its values for the tax year".into(),
            document_id: Some("pc".into()),
            entry_id: None,
            line_id: None,
        }]
    );
    assert!(later.attention.is_empty());
    assert!(rows_of(&purchase, "720")[0].draft);
    assert!(rows_of(&later, "720")[0].draft);
}

#[test]
fn the_purchase_years_gwg_limit_decides_whatever_the_later_years_limits_are() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc-2026", "2026-03-10", 150_000, |_| {});
    computer(&db, "pc-2027", "2027-01-10", 150_000, |_| {});

    let in_2027 = page_of(&db, 2027);

    assert_eq!(
        rows_of(&in_2027, "720"),
        [
            slice_row(
                "pc-2026",
                "From 2026 · year 2 of depreciating €1,500.00 over 3 years",
                "€500.00",
                Some(2026),
                false
            ),
            slice_row("pc-2027", "10 Jan", "€1,500.00", None, false),
        ]
    );
    assert_eq!(in_2027.total, "€2,000.00");
}

#[test]
fn a_purchase_at_the_gwg_limit_counts_fully_and_one_cent_above_it_spreads() {
    for (cents, expected, detail) in [
        (40_000, "€400.00", "10 Mar"),
        (
            40_001,
            "€133.33",
            "10 Mar · year 1 of depreciating €400.01 over 3 years",
        ),
    ] {
        let (_dir, db) = depreciation_years();
        computer(&db, "pc", "2026-03-10", cents, |_| {});

        let page = page_of(&db, 2026);

        assert_eq!(amounts(rows_of(&page, "720")), [expected], "{cents}");
        assert_eq!(rows_of(&page, "720")[0].detail, detail, "{cents}");
    }
}

#[test]
fn the_work_share_is_applied_before_the_gwg_limit_is_compared() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 100_000, |d| d.work_share = 40);

    let page = page_of(&db, 2026);

    assert_eq!(
        rows_of(&page, "720"),
        [slice_row(
            "pc",
            "10 Mar · 40 % of €1,000.00",
            "€400.00",
            None,
            false
        )]
    );
    assert!(rows_of(&page_of(&db, 2027), "720").is_empty());
}

#[test]
fn without_a_payment_date_in_any_year_a_depreciable_document_is_only_listed_as_undated() {
    let (_dir, db) = depreciation_years();
    add_document(&db, "pc", |d| {
        d.item_key = Some("computer".into());
        d.paid_cents = Some(150_000);
    });

    let page = page_of(&db, 2026);

    assert_eq!(page.total, "€0.00");
    assert_eq!(page.attention.len(), 1);
    assert_eq!(page.attention[0].kind, AttentionKind::Undated);
}

#[test]
fn a_carried_row_travels_with_its_purchase_year_in_camel_case() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 150_000, |_| {});

    let page = page_of(&db, 2027);

    assert_eq!(
        serde_json::to_value(&rows_of(&page, "720")[0]).unwrap(),
        json!({
            "id": "pc",
            "kind": "document",
            "title": "pc",
            "detail": "From 2026 · year 2 of depreciating €1,500.00 over 3 years",
            "amount": "€500.00",
            "draft": false,
            "carriedFrom": 2026,
            "kz": "720",
            "attach": false
        })
    );
}

#[test]
fn without_a_pauschale_in_the_rules_there_is_no_note() {
    let mut rules = rules();
    rules["settings"]
        .as_object_mut()
        .unwrap()
        .remove("werbungskostenpauschale");
    let (_dir, db) = open_year_with(&rules);

    assert_eq!(year_page(&db).pauschale_note, None);
}

#[test]
fn archived_counts_the_excluded_documents_paid_in_the_year() {
    let (_dir, db) = open_year();
    for (id, date) in [
        ("doc-x1", Some("2026-02-02")),
        ("doc-x2", Some("2026-06-06")),
        ("doc-x3", Some("2025-06-06")),
        ("doc-x4", None),
    ] {
        add_document(&db, id, |d| {
            d.state = DocState::Excluded;
            d.excluded_at = Some(5);
            d.item_key = Some("books".into());
            d.pay_date = date.map(String::from);
            d.paid_cents = Some(1_000);
        });
    }

    let page = year_page(&db);

    assert_eq!(page.archived, 2);
    assert_eq!(page.total, "€0.00");
    assert!(page.attention.is_empty());
}

#[test]
fn a_query_keeps_the_rows_matching_title_kennzahl_or_item_name_and_drops_the_rest() {
    let (_dir, db) = counted_entries();

    for (query, expected) in [
        ("doc-b", vec![("721", "72,00")]),
        ("720", vec![("720", "46,90")]),
        ("PARKING", vec![("724", "25,00")]),
        ("  copyshop ", vec![("722", "40,00")]),
        ("doc-", vec![("720", "46,90"), ("721", "72,00")]),
        ("no such row", vec![]),
    ] {
        let page = page_matching(&db, query);

        assert_eq!(kennzahlen(&page), expected, "{query:?}");
        assert!(
            page.kennzahlen.iter().all(|k| !k.rows.is_empty()),
            "{query:?}"
        );
    }
}

#[test]
fn a_query_filters_the_months_but_not_the_total() {
    let (_dir, db) = counted_entries();

    let page = page_matching(&db, "doc-b");

    assert_eq!(page.total, "€183.90");
    let months: Vec<(&str, &str)> = page
        .months
        .iter()
        .map(|m| (m.label.as_str(), m.total.as_str()))
        .collect();
    assert_eq!(months, [("November", "€72.00")]);
    assert!(page_matching(&db, "no such row").months.is_empty());
}

#[test]
fn questions_show_their_answers_as_the_field_does() {
    let (_dir, db) = open_year();
    answer_with(&db, "telearbeitstage", "28");
    answer_with(&db, "pauschale", "1234.5");

    let page = year_page(&db);
    let comma = page_comma(&db);

    let shown: Vec<(&str, &str, bool)> = page
        .questions
        .iter()
        .map(|q| (q.id.as_str(), q.value.as_str(), q.answered))
        .collect();
    assert_eq!(
        shown,
        [
            ("telearbeitstage", "28", true),
            ("pauschale", "1,234.50", true)
        ]
    );
    assert_eq!(comma.questions[1].value, "1.234,50");
    assert_eq!(page.progress, "Ready to close");
}

fn page_comma(db: &Db) -> YearPage {
    page(
        db,
        2026,
        "",
        NumberFormat::CommaDecimal,
        DateOrder::DayFirst,
    )
    .unwrap()
}

#[test]
fn an_unanswered_question_shows_empty_with_its_label_help_and_unit() {
    let (_dir, db) = open_year();

    let page = year_page(&db);

    assert_eq!(
        serde_json::to_value(&page.questions).unwrap(),
        json!([
            {
                "id": "telearbeitstage", "label": HOME_DAYS, "help": "Days worked from home",
                "unit": "days", "value": "", "answered": false
            },
            {
                "id": "pauschale", "label": "Telearbeitspauschale (from the employer)", "help": null,
                "unit": null, "value": "", "answered": false
            }
        ])
    );
}

#[test]
fn the_page_travels_with_camel_case_field_names_and_snake_case_words() {
    let (_dir, db) = open_year();
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.pay_date = Some("2026-03-03".into());
    });

    let value = serde_json::to_value(year_page(&db)).unwrap();

    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "archived",
            "attention",
            "kennzahlen",
            "months",
            "pauschaleNote",
            "progress",
            "progressPercent",
            "questions",
            "ruleset",
            "state",
            "total",
            "year"
        ]
    );
    assert_eq!(value["state"], "open");
    assert_eq!(value["attention"][0]["kind"], "draft");
    assert_eq!(value["attention"][0]["documentId"], "doc-d1");
}

#[test]
fn a_refund_linked_to_a_document_is_netted_against_its_charge_in_the_year() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-charge", "2026-10-14", -10_000, "Lastschrift SEITENWEISE"),
            debit("l-refund", "2026-10-20", 3_000, "Gutschrift SEITENWEISE"),
        ],
    );
    add_document(&db, "doc-p", |d| {
        d.item_key = Some("phone".into());
        d.pay_date = Some("2026-10-14".into());
        d.work_share = 60;
    });
    link(&db, "doc-p", "l-charge");
    link(&db, "doc-p", "l-refund");

    let page = year_page(&db);

    assert_eq!(
        rows_of(&page, "721"),
        [YearRow {
            id: "doc-p".into(),
            kind: RowKind::Document,
            title: "doc-p".into(),
            detail: "14 Oct · 60 % of €70.00".into(),
            amount: "€42.00".into(),
            draft: false,
            carried_from: None,
            kz: "721".into(),
            attach: false,
        }]
    );
    assert_eq!(page.total, "€42.00");
    assert!(kennzahlen(&page).contains(&("721", "42,00")));
}

#[test]
fn a_refund_in_another_year_does_not_reduce_this_years_payment() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-charge", "2026-12-20", -10_000, "Lastschrift SEITENWEISE"),
            debit("l-refund", "2027-01-08", 3_000, "Gutschrift SEITENWEISE"),
        ],
    );
    add_document(&db, "doc-b", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-12-20".into());
    });
    link(&db, "doc-b", "l-charge");
    link(&db, "doc-b", "l-refund");

    let page = year_page(&db);

    assert_eq!(amounts(rows_of(&page, "720")), ["€100.00"]);
    assert_eq!(page.total, "€100.00");
}

#[test]
fn a_credit_line_decided_anv_without_a_document_reduces_the_years_total() {
    let (_dir, db) = open_year();
    add_lines(
        &db,
        &[
            debit("l-tips", "2026-09-09", -5_000, "Lastschrift COPYSHOP"),
            debit("l-back", "2026-09-12", 1_000, "Gutschrift COPYSHOP"),
        ],
    );
    decide(&db, "l-tips", Action::Anv, Some("tips"));
    decide(&db, "l-back", Action::Anv, Some("tips"));

    let page = year_page(&db);

    let credit = rows_of(&page, "722")
        .iter()
        .find(|r| r.id == "l-back")
        .unwrap();
    assert_eq!(credit.amount, "\u{2212}€8.00");
    assert_eq!(credit.kind, RowKind::Line);
    assert_eq!(page.total, "€32.00");
    assert!(kennzahlen(&page).contains(&("722", "32,00")));
}

#[test]
fn a_depreciable_document_whose_refunds_cover_its_cost_counts_as_a_whole_without_slices() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 1, |_| {});
    add_lines(
        &db,
        &[
            debit("l-charge", "2026-03-10", -150_000, "Lastschrift PIXELFORGE"),
            debit("l-refund", "2026-03-20", 150_000, "Gutschrift PIXELFORGE"),
        ],
    );
    link(&db, "pc", "l-charge");
    link(&db, "pc", "l-refund");

    let this_year = page_of(&db, 2026);
    let next_year = page_of(&db, 2027);

    assert_eq!(
        rows_of(&this_year, "720"),
        [slice_row("pc", "10 Mar", "€0.00", None, false)]
    );
    assert_eq!(this_year.total, "€0.00");
    assert!(rows_of(&next_year, "720").is_empty());
    assert_eq!(next_year.total, "€0.00");
}

#[test]
fn a_depreciable_document_is_spread_over_what_is_left_after_a_partial_refund() {
    let (_dir, db) = depreciation_years();
    computer(&db, "pc", "2026-03-10", 1, |_| {});
    add_lines(
        &db,
        &[
            debit("l-charge", "2026-03-10", -170_000, "Lastschrift PIXELFORGE"),
            debit("l-refund", "2026-03-12", 20_000, "Gutschrift PIXELFORGE"),
        ],
    );
    link(&db, "pc", "l-charge");
    link(&db, "pc", "l-refund");

    let page = page_of(&db, 2026);

    assert_eq!(
        rows_of(&page, "720"),
        [slice_row(
            "pc",
            "10 Mar · year 1 of depreciating €1,500.00 over 3 years",
            "€500.00",
            None,
            false
        )]
    );
}

fn closed_subscription_year() -> (TempDir, Db) {
    use crate::test_support::subscriptions::{insert_rows, plan_row, subscription_row};
    let (dir, db) = open_year();
    let mut copy_club = subscription_row("sub-copy", "Copy Club", "COPYSHOP");
    copy_club.item_key = Some("tips".into());
    insert_rows(
        &db,
        &copy_club,
        &[plan_row("plan-1", "sub-copy", 5_000, "2026-01-01", None)],
    );
    add_lines(
        &db,
        &[debit(
            "l-copy",
            "2026-09-09",
            -5_000,
            "Lastschrift COPYSHOP",
        )],
    );
    insert(
        &db,
        decisions::DECISIONS,
        &DecisionRow {
            id: "l-copy".into(),
            kind: Action::Subscription,
            item_key: None,
            subscription_id: Some("sub-copy".into()),
            document_id: None,
            rule_id: None,
            decided_at: 1,
        },
    );
    finished(&db, "doc-a", "books", "2026-10-14", 4_690, 100);
    crate::year::close::close(
        &db,
        dir.path(),
        2026,
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
        "Closed 13 Nov 2026",
        500,
        Source::Ui,
    )
    .unwrap();
    (dir, db)
}

/// The Copy Club subscription no longer counts for any item.
fn drop_the_subscriptions_item(db: &Db) {
    use crate::subscriptions::edit;
    use crate::test_support::subscriptions::{form, today};
    edit::update(
        db,
        "sub-copy",
        &form("Copy Club", "COPYSHOP", "50.00"),
        today(),
        NOW,
        Source::Ui,
    )
    .unwrap();
}

fn changed_since_closing(kz: &str, filed: &str, now: &str) -> AttentionView {
    AttentionView {
        kind: AttentionKind::ChangedSinceClosing,
        title: format!("KZ {kz} changed since closing"),
        detail: format!(
            "Filed {filed}; the data now gives {now}. Reopen the year to look into it."
        ),
        document_id: None,
        entry_id: None,
        line_id: None,
    }
}

#[test]
fn a_closed_year_whose_data_did_not_change_has_nothing_changed_since_closing() {
    let (_dir, db) = closed_subscription_year();

    let page = year_page(&db);

    assert_eq!(page.state, YearState::Closed);
    assert_eq!(page.attention, Vec::<AttentionView>::new());
    assert!(kennzahlen(&page).contains(&("722", "40,00")));
}

#[test]
fn a_closed_year_keeps_the_filed_value_and_names_what_the_data_now_gives() {
    let (_dir, db) = closed_subscription_year();
    drop_the_subscriptions_item(&db);

    let page = year_page(&db);

    assert_eq!(
        kennzahlen(&page),
        [
            ("720", "46,90"),
            ("721", "0,00"),
            ("722", "40,00"),
            ("724", "0,00"),
            ("726", "0,00")
        ]
    );
    assert_eq!(
        page.attention,
        [changed_since_closing("722", "40,00", "0,00")]
    );
}

#[test]
fn a_filtered_page_of_a_closed_year_does_not_report_changes_since_closing() {
    let (_dir, db) = closed_subscription_year();
    drop_the_subscriptions_item(&db);

    let page = page_matching(&db, "books");

    assert_eq!(page.attention, Vec::<AttentionView>::new());
}

#[test]
fn a_reopened_year_shows_the_live_values_and_no_change_since_closing() {
    let (_dir, db) = closed_subscription_year();
    drop_the_subscriptions_item(&db);
    crate::year::close::reopen(&db, 2026, Source::Ui).unwrap();

    let page = year_page(&db);

    assert_eq!(page.state, YearState::Open);
    assert!(kennzahlen(&page).contains(&("722", "0,00")));
    assert!(
        page.attention
            .iter()
            .all(|a| a.kind != AttentionKind::ChangedSinceClosing)
    );
}

#[test]
fn a_change_since_closing_travels_as_a_snake_case_kind() {
    let (_dir, db) = closed_subscription_year();
    drop_the_subscriptions_item(&db);

    let value = serde_json::to_value(year_page(&db)).unwrap();

    assert_eq!(
        value["attention"][0],
        json!({
            "kind": "changed_since_closing",
            "title": "KZ 722 changed since closing",
            "detail": "Filed 40,00; the data now gives 0,00. Reopen the year to look into it.",
            "documentId": null,
            "entryId": null,
            "lineId": null
        })
    );
}
