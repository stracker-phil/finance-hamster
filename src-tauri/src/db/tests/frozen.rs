use rusqlite::params;
use serde_json::{Value, json};

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::test_support::temp_config_dir;

const CLOSED_2025: &str = "Tax year 2025 is closed; reopen it to change what it counts";

fn add_year(db: &Db, year: i32) {
    db.read(|conn| {
        conn.execute(
            "INSERT INTO tax_years (id, year, state, ruleset, rules, rules_loaded_at) \
             VALUES (?1, ?2, 'open', 'v1', '{}', 1)",
            params![year.to_string(), year],
        )?;
        Ok(())
    })
    .unwrap();
}

fn add_line(db: &Db, id: &str, value_date: &str) {
    db.read(|conn| {
        conn.execute(
            "INSERT INTO transactions \
             (id, statement_id, account_id, page, row, value_date, cents, text, extra, transfer) \
             VALUES (?1, 'statement-1', 'account-1', 1, 1, ?2, -1299, 'Streamflix', '[]', 0)",
            params![id, value_date],
        )?;
        Ok(())
    })
    .unwrap();
}

fn add_subscription(db: &Db, id: &str, item_key: Option<&str>) {
    db.read(|conn| {
        conn.execute(
            "INSERT INTO subscriptions \
             (id, name, tone, category, reminder, item_key, match_text, created_at) \
             VALUES (?1, 'Cloudbox', 0, 'software', 'none', ?2, 'CLOUDBOX', 1)",
            params![id, item_key],
        )?;
        Ok(())
    })
    .unwrap();
}

fn document(id: &str, state: &str, pay_date: Option<&str>) -> Value {
    json!({
        "id": id, "state": state, "file": format!("documents/{id}.pdf"),
        "original_name": format!("{id}.pdf"), "file_hash": format!("hash-{id}"),
        "pay_date": pay_date, "work_share": 100, "note": "", "created_at": 1
    })
}

fn decision(line: &str, kind: &str, subscription: Option<&str>) -> Value {
    json!({ "id": line, "kind": kind, "subscription_id": subscription, "decided_at": 1 })
}

fn entry(id: &str, year: i32) -> Value {
    json!({
        "id": id, "year": year, "item_key": "books", "kind": "amount",
        "date": format!("{year}-03-01"), "cents": 1_000, "text": "Fachbuch", "created_at": 1
    })
}

fn answer(id: &str, year: i32) -> Value {
    json!({ "id": id, "year": year, "question": "home-office-days", "value": 10, "answered_at": 1 })
}

fn payslip(id: &str, year: i32, month: u32, gross: i64) -> Value {
    json!({
        "id": id, "year": year, "month": month, "gross": gross, "note": "", "created_at": 1
    })
}

fn link(id: &str, document: &str, line: &str) -> Value {
    json!({ "id": id, "document_id": document, "transaction_id": line, "created_at": 1 })
}

fn insert(table: &str, row: &Value) -> Op {
    Op::new("Add", vec![Change::insert(table, row).unwrap()])
}

fn delete(table: &str, row: &Value) -> Op {
    Op::new("Remove", vec![Change::delete(table, row).unwrap()])
}

fn update(table: &str, before: &Value, after: &Value) -> Op {
    Op::new("Edit", vec![Change::update(table, before, after).unwrap()])
}

fn year_state(year: i32, from: &str, to: &str) -> Change {
    let row = |state: &str| json!({ "id": year.to_string(), "state": state });
    Change::update("tax_years", &row(from), &row(to)).unwrap()
}

fn close(db: &Db, year: i32) {
    db.write(
        Op::new("Close year", vec![year_state(year, "open", "closed")]),
        Source::App,
    )
    .unwrap();
}

fn reopen(db: &Db, year: i32) {
    db.write(
        Op::new("Reopen year", vec![year_state(year, "closed", "open")]),
        Source::App,
    )
    .unwrap();
}

fn count(db: &Db, table: &str) -> i64 {
    db.read(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?))
        .unwrap()
}

fn state_of(db: &Db, year: i32) -> String {
    db.read(|c| {
        Ok(
            c.query_row("SELECT state FROM tax_years WHERE year = ?", [year], |r| {
                r.get(0)
            })?,
        )
    })
    .unwrap()
}

fn refusal(result: AppResult<Written>) -> String {
    match result {
        Err(AppError::InvalidInput(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Years 2025 and 2026, both open, with two lines in 2025 and one in 2026, a Werbungskosten and a plain subscription, and a
/// document per year plus an excluded one paid in 2025.
fn world() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.read(|conn| {
        conn.pragma_update(None, "foreign_keys", false)?;
        Ok(())
    })
    .unwrap();
    add_year(&db, 2025);
    add_year(&db, 2026);
    add_line(&db, "line-2025", "2025-06-01");
    add_line(&db, "line-2025b", "2025-07-01");
    add_line(&db, "line-2026", "2026-06-01");
    add_subscription(&db, "sub-item", Some("books"));
    add_subscription(&db, "sub-plain", None);
    for row in [
        document("doc-2025", "draft", Some("2025-03-03")),
        document("doc-2026", "draft", Some("2026-03-03")),
        document("doc-excluded-2025", "excluded", Some("2025-04-04")),
    ] {
        db.write(insert("documents", &row), Source::Seed).unwrap();
    }
    (dir, db)
}

fn journal_rows(db: &Db) -> i64 {
    count(db, "journal")
}

#[test]
fn a_closed_year_refuses_every_row_that_counts_in_it() {
    let (_dir, db) = world();
    close(&db, 2025);
    for (name, table, row) in [
        (
            "a document paid in the year",
            "documents",
            document("doc-new", "draft", Some("2025-09-09")),
        ),
        ("an entry of the year", "entries", entry("entry-1", 2025)),
        ("an answer of the year", "answers", answer("answer-1", 2025)),
        (
            "an ANV decision on a line of the year",
            "decisions",
            decision("line-2025", "anv", None),
        ),
        (
            "a subscription decision with an item on a line of the year",
            "decisions",
            decision("line-2025", "subscription", Some("sub-item")),
        ),
        (
            "a link to a line of the year",
            "document_links",
            link("link-1", "doc-2026", "line-2025"),
        ),
        (
            "a link to a document paid in the year",
            "document_links",
            link("link-2", "doc-2025", "line-2026"),
        ),
    ] {
        let rows = count(&db, table);
        let journal = journal_rows(&db);

        let message = refusal(db.write(insert(table, &row), Source::Ui));

        assert_eq!(message, CLOSED_2025, "{name}");
        assert_eq!(count(&db, table), rows, "{name}");
        assert_eq!(journal_rows(&db), journal, "{name}");
    }
}

#[test]
fn a_closed_year_refuses_adding_changing_and_deleting_its_payslips() {
    let (_dir, db) = world();
    let march = payslip("payslip-march", 2025, 3, 300_000);
    db.write(insert("payslips", &march), Source::Ui).unwrap();
    close(&db, 2025);

    for (name, op) in [
        (
            "adding one",
            insert("payslips", &payslip("payslip-april", 2025, 4, 310_000)),
        ),
        (
            "changing one",
            update(
                "payslips",
                &march,
                &payslip("payslip-march", 2025, 3, 320_000),
            ),
        ),
        ("deleting one", delete("payslips", &march)),
    ] {
        assert_eq!(refusal(db.write(op, Source::Ui)), CLOSED_2025, "{name}");
    }
    assert_eq!(count(&db, "payslips"), 1);
}

#[test]
fn an_open_year_allows_adding_changing_and_deleting_its_payslips() {
    let (_dir, db) = world();
    close(&db, 2025);
    let march = payslip("payslip-march", 2026, 3, 300_000);

    db.write(insert("payslips", &march), Source::Ui).unwrap();
    assert_eq!(count(&db, "payslips"), 1);

    db.write(
        update(
            "payslips",
            &march,
            &payslip("payslip-march", 2026, 3, 320_000),
        ),
        Source::Ui,
    )
    .unwrap();
    let gross: i64 = db
        .read(|c| Ok(c.query_row("SELECT gross FROM payslips", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(gross, 320_000);

    db.write(
        delete("payslips", &payslip("payslip-march", 2026, 3, 320_000)),
        Source::Ui,
    )
    .unwrap();
    assert_eq!(count(&db, "payslips"), 0);
}

#[test]
fn a_payslip_moving_between_years_is_refused_when_either_year_is_closed() {
    let (_dir, db) = world();
    let december = payslip("payslip-dec", 2026, 12, 300_000);
    db.write(insert("payslips", &december), Source::Ui).unwrap();
    close(&db, 2025);

    let moved = payslip("payslip-dec", 2025, 12, 300_000);

    assert_eq!(
        refusal(db.write(update("payslips", &december, &moved), Source::Ui)),
        CLOSED_2025
    );
}

#[test]
fn a_closed_year_refuses_removing_or_moving_what_counts_in_it() {
    let (_dir, db) = world();
    close(&db, 2025);
    let doc_2025 = document("doc-2025", "draft", Some("2025-03-03"));
    let doc_2026 = document("doc-2026", "draft", Some("2026-03-03"));
    let cases = [
        ("removing a document", delete("documents", &doc_2025)),
        (
            "moving a document out of the year",
            update(
                "documents",
                &doc_2025,
                &document("doc-2025", "draft", Some("2026-01-02")),
            ),
        ),
        (
            "moving a document into the year",
            update(
                "documents",
                &doc_2026,
                &document("doc-2026", "draft", Some("2025-12-31")),
            ),
        ),
        (
            "excluding a document of the year",
            update(
                "documents",
                &doc_2025,
                &document("doc-2025", "excluded", Some("2025-03-03")),
            ),
        ),
        (
            "restoring an excluded document of the year",
            update(
                "documents",
                &document("doc-excluded-2025", "excluded", Some("2025-04-04")),
                &document("doc-excluded-2025", "draft", Some("2025-04-04")),
            ),
        ),
    ];
    for (name, op) in cases {
        assert_eq!(refusal(db.write(op, Source::Ui)), CLOSED_2025, "{name}");
    }
    assert_eq!(count(&db, "documents"), 3);
}

#[test]
fn removing_or_changing_a_decision_that_counted_is_refused_in_a_closed_year() {
    let (_dir, db) = world();
    let anv = decision("line-2025", "anv", None);
    db.write(insert("decisions", &anv), Source::Ui).unwrap();
    close(&db, 2025);

    for (name, op) in [
        ("removing it", delete("decisions", &anv)),
        (
            "turning it into an ignore",
            update("decisions", &anv, &decision("line-2025", "ignore", None)),
        ),
    ] {
        assert_eq!(refusal(db.write(op, Source::Ui)), CLOSED_2025, "{name}");
    }
    assert_eq!(count(&db, "decisions"), 1);
}

#[test]
fn rows_that_count_nowhere_are_never_refused_in_a_closed_year() {
    let (_dir, db) = world();
    close(&db, 2025);
    let vendor = json!({ "id": "vendor-1", "name": "Buchhandlung", "created_at": 1 });
    for (name, table, row) in [
        (
            "an ignore decision on a line of the year",
            "decisions",
            decision("line-2025", "ignore", None),
        ),
        (
            "a subscription decision without an item",
            "decisions",
            decision("line-2025b", "subscription", Some("sub-plain")),
        ),
        (
            "an excluded document paid in the year",
            "documents",
            document("doc-x", "excluded", Some("2025-05-05")),
        ),
        (
            "a document without a payment date",
            "documents",
            document("doc-undated", "draft", None),
        ),
        (
            "a link of an excluded document to a line of an open year",
            "document_links",
            link("link-1", "doc-excluded-2025", "line-2026"),
        ),
        ("a vendor", "vendors", vendor),
    ] {
        db.write(insert(table, &row), Source::Ui)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
}

#[test]
fn a_new_line_dated_in_a_closed_year_is_stored() {
    let (_dir, db) = world();
    close(&db, 2025);
    let line = json!({
        "id": "line-late", "statement_id": "statement-1", "account_id": "account-1", "page": 1,
        "row": 2, "value_date": "2025-12-30", "cents": -500, "text": "Late", "extra": "[]",
        "transfer": 0
    });

    db.write(insert("transactions", &line), Source::Import)
        .unwrap();

    assert_eq!(count(&db, "transactions"), 4);
}

#[test]
fn open_years_and_years_without_a_row_are_not_refused_while_another_is_closed() {
    let (_dir, db) = world();
    close(&db, 2025);
    for (table, row) in [
        (
            "documents",
            document("doc-new", "draft", Some("2026-09-09")),
        ),
        (
            "documents",
            document("doc-old", "draft", Some("2024-09-09")),
        ),
        ("entries", entry("entry-1", 2026)),
        ("entries", entry("entry-2", 2031)),
        ("answers", answer("answer-1", 2026)),
        ("decisions", decision("line-2026", "anv", None)),
    ] {
        db.write(insert(table, &row), Source::Ui)
            .unwrap_or_else(|e| panic!("{table} {row}: {e:?}"));
    }
}

fn decide_by_hand(db: &Db, line: &str, subscription: &str) {
    db.write(
        insert(
            "decisions",
            &decision(line, "subscription", Some(subscription)),
        ),
        Source::Seed,
    )
    .unwrap();
}

fn delete_subscription(db: &Db, id: &str) -> AppResult<Written> {
    crate::subscriptions::edit::delete(db, id, 5, Source::Ui)
}

#[test]
fn deleting_a_subscription_with_an_item_and_a_hand_decision_in_a_closed_year_is_refused() {
    let (_dir, db) = world();
    decide_by_hand(&db, "line-2025", "sub-item");
    close(&db, 2025);

    assert_eq!(refusal(delete_subscription(&db, "sub-item")), CLOSED_2025);

    assert_eq!(count(&db, "subscriptions"), 2);
    assert_eq!(count(&db, "decisions"), 1);
}

#[test]
fn deleting_a_subscription_without_an_item_succeeds_despite_a_hand_decision_in_a_closed_year() {
    let (_dir, db) = world();
    decide_by_hand(&db, "line-2025b", "sub-plain");
    close(&db, 2025);

    delete_subscription(&db, "sub-plain").unwrap();

    assert_eq!(count(&db, "subscriptions"), 1);
    assert_eq!(count(&db, "decisions"), 0);
}

#[test]
fn deleting_a_subscription_with_an_item_succeeds_when_its_decisions_are_in_open_years() {
    let (_dir, db) = world();
    decide_by_hand(&db, "line-2026", "sub-item");
    close(&db, 2025);

    delete_subscription(&db, "sub-item").unwrap();

    assert_eq!(count(&db, "subscriptions"), 1);
    assert_eq!(count(&db, "decisions"), 0);
}

#[test]
fn a_decision_of_a_subscription_that_no_longer_exists_counts_in_its_closed_year() {
    let (_dir, db) = world();
    decide_by_hand(&db, "line-2025b", "sub-plain");
    close(&db, 2025);

    let gone = decision("line-2025b", "subscription", Some("sub-plain"));
    db.read(|c| {
        c.execute("DELETE FROM subscriptions WHERE id = 'sub-plain'", [])?;
        Ok(())
    })
    .unwrap();

    assert_eq!(
        refusal(db.write(delete("decisions", &gone), Source::Ui)),
        CLOSED_2025
    );
}

#[test]
fn reopening_a_year_lets_its_rows_change_again() {
    let (_dir, db) = world();
    close(&db, 2025);
    let entry = entry("entry-1", 2025);
    assert!(db.write(insert("entries", &entry), Source::Ui).is_err());

    reopen(&db, 2025);
    db.write(insert("entries", &entry), Source::Ui).unwrap();

    assert_eq!(count(&db, "entries"), 1);
}

#[test]
fn a_refused_op_writes_none_of_its_changes_and_no_journal_row() {
    let (dir, db) = world();
    close(&db, 2025);
    let journal = journal_rows(&db);
    let journal_file = std::fs::read_to_string(dir.path().join(crate::db::journal::FILE_NAME)).ok();
    let op = Op::new(
        "Add entries",
        vec![
            Change::insert("entries", &entry("entry-open", 2026)).unwrap(),
            Change::insert("answers", &answer("answer-open", 2026)).unwrap(),
            Change::insert("entries", &entry("entry-closed", 2025)).unwrap(),
        ],
    );

    assert_eq!(refusal(db.write(op, Source::Ui)), CLOSED_2025);

    assert_eq!((count(&db, "entries"), count(&db, "answers")), (0, 0));
    assert_eq!(journal_rows(&db), journal);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(crate::db::journal::FILE_NAME)).ok(),
        journal_file
    );
    assert_eq!(db.undo().unwrap(), None);
}

#[test]
fn closing_the_year_as_the_last_change_keeps_the_rows_written_before_it() {
    let (_dir, db) = world();
    let op = Op::new(
        "Add and close",
        vec![
            Change::insert("entries", &entry("entry-1", 2025)).unwrap(),
            year_state(2025, "open", "closed"),
        ],
    );

    db.write(op, Source::App).unwrap();

    assert_eq!(count(&db, "entries"), 1);
    assert_eq!(state_of(&db, 2025), "closed");
}

#[test]
fn closing_the_year_first_refuses_the_rows_after_it_and_leaves_the_year_open() {
    let (_dir, db) = world();
    let op = Op::new(
        "Close and add",
        vec![
            year_state(2025, "open", "closed"),
            Change::insert("entries", &entry("entry-1", 2025)).unwrap(),
        ],
    );

    assert_eq!(refusal(db.write(op, Source::App)), CLOSED_2025);

    assert_eq!(state_of(&db, 2025), "open");
    assert_eq!(count(&db, "entries"), 0);
}

#[test]
fn reopening_the_year_first_lets_the_same_op_change_its_rows() {
    let (_dir, db) = world();
    close(&db, 2025);
    let op = Op::new(
        "Reopen and add",
        vec![
            year_state(2025, "closed", "open"),
            Change::insert("entries", &entry("entry-1", 2025)).unwrap(),
        ],
    );

    db.write(op, Source::App).unwrap();

    assert_eq!(state_of(&db, 2025), "open");
    assert_eq!(count(&db, "entries"), 1);
}

#[test]
fn undoing_an_op_made_before_the_year_closed_is_refused_until_it_reopens() {
    let (_dir, db) = world();
    db.write(insert("entries", &entry("entry-1", 2025)), Source::Ui)
        .unwrap();
    close(&db, 2025);

    assert_eq!(refusal(db.undo().map(|w| w.unwrap())), CLOSED_2025);
    assert_eq!(count(&db, "entries"), 1);

    reopen(&db, 2025);
    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.tables, vec!["entries".to_string()]);
    assert_eq!(count(&db, "entries"), 0);
}

#[test]
fn redoing_an_op_is_refused_while_its_year_is_closed_until_it_reopens() {
    let (_dir, db) = world();
    db.write(insert("entries", &entry("entry-1", 2025)), Source::Ui)
        .unwrap();
    db.undo().unwrap().unwrap();
    close(&db, 2025);

    assert_eq!(refusal(db.redo().map(|w| w.unwrap())), CLOSED_2025);
    assert_eq!(count(&db, "entries"), 0);

    reopen(&db, 2025);
    db.redo().unwrap().unwrap();

    assert_eq!(count(&db, "entries"), 1);
}
