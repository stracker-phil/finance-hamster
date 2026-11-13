use std::path::Path;

use super::import::{self, ChainKind, ImportPreview, ImportQueue, Pending, slug};
use super::store::{self, StatementRow, TransactionRow};
use crate::accounts::{self, AccountRow};
use crate::date::DateOrder;
use crate::db::{Change, Db, Op, Source};
use crate::error::{AppError, AppResult};
use crate::platform::pdf::write::{self, Font, Run};
use crate::settings::NumberFormat;
use crate::test_support::demo::DEMO_ACCOUNTS;
use crate::test_support::statements::{Statement, StatementLine, demo_month};
use crate::test_support::temp_config_dir;

const NOW: i64 = 1_700_000_000;

/// A database holding the demo accounts as `account-1` (Private), `account-2` and `account-3`.
pub(crate) fn open_with_accounts() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    let changes = DEMO_ACCOUNTS
        .iter()
        .enumerate()
        .map(|(i, (name, iban))| {
            Change::insert(
                accounts::TABLE,
                &AccountRow {
                    id: format!("account-{}", i + 1),
                    name: (*name).into(),
                    bank: "Example Sparkasse".into(),
                    iban: (*iban).into(),
                    own: true,
                    created_at: i as i64,
                },
            )
        })
        .collect::<AppResult<Vec<_>>>()
        .unwrap();
    db.write(Op::new("Add accounts", changes), Source::Seed)
        .unwrap();
    (dir, db)
}

/// `open_with_accounts` plus an open 2026 tax year whose only item is `books` (KZ 720).
pub(crate) fn open_with_accounts_and_year() -> (tempfile::TempDir, Db) {
    use crate::taxyears::{self, TaxYearRow, YearState};
    let (dir, db) = open_with_accounts();
    let row = TaxYearRow {
        state: YearState::Open,
        closed_at: None,
        ..TaxYearRow::new(
            2026,
            "2026 v1".into(),
            crate::test_support::rules::minimal_text(2026),
            1,
        )
    };
    db.write(
        Op::new(
            "Add year",
            vec![Change::insert(taxyears::TABLE, &row).unwrap()],
        ),
        Source::App,
    )
    .unwrap();
    (dir, db)
}

pub(crate) fn october() -> Statement {
    demo_month(10, 2026, 10, 141_230)
}

fn prepared(db: &Db, dir: &Path, statement: &Statement) -> Pending {
    import::prepare(db, dir, statement.pdf()).unwrap()
}

fn imported(db: &Db, dir: &Path, statement: &Statement) -> StatementRow {
    let pending = prepared(db, dir, statement);
    import::commit(db, dir, pending, NOW, Source::Import)
        .unwrap()
        .1
}

fn refusal(result: AppResult<Pending>) -> String {
    match result {
        Err(AppError::InvalidInput(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn lines_of(db: &Db, statement_id: &str) -> Vec<TransactionRow> {
    db.read(|c| store::transactions_of(c, statement_id))
        .unwrap()
}

fn count(db: &Db, table: &str) -> i64 {
    db.read(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?))
        .unwrap()
}

fn preview_of(db: &Db, pending: &Pending) -> ImportPreview {
    import::preview(
        db,
        "token-1".into(),
        "october.pdf".into(),
        pending,
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap()
}

fn household_transfer() -> StatementLine {
    StatementLine {
        text: "Dauerauftrag".into(),
        extra: vec![
            "HOUSEHOLD".into(),
            "AT94 1999 9000 0000 0002".into(),
            "HAUSHALT".into(),
        ],
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
        cents: -50_000,
    }
}

#[test]
fn a_name_becomes_a_lower_case_file_slug() {
    for (name, expected) in [
        ("Private", "private"),
        ("Haushalt Ötzi", "haushalt-oetzi"),
        ("Große Brücke", "grosse-bruecke"),
        ("  ", "account"),
        ("", "account"),
        ("---", "account"),
        ("Savings -- (2026)!", "savings-2026"),
        ("  Business  ", "business"),
    ] {
        assert_eq!(slug(name), expected, "{name:?}");
    }
}

#[test]
fn a_demo_statement_is_prepared_for_the_account_with_its_iban() {
    let (dir, db) = open_with_accounts();
    let statement = october();
    let bytes = statement.pdf();

    let pending = import::prepare(&db, dir.path(), bytes.clone()).unwrap();

    assert_eq!(pending.account.id, "account-1");
    assert_eq!(pending.account.name, "Private");
    assert_eq!(pending.hash, super::file_hash(&bytes));
    assert_eq!(pending.bytes, bytes);
    assert_eq!(pending.pages, 1);
    assert_eq!(pending.parsed.number, 10);
    assert_eq!(pending.parsed.year, 2026);
    assert_eq!(pending.parsed.lines.len(), 7);
    assert_eq!(pending.file, "statements/2026/private-010.pdf");
    assert!(!dir.path().join("statements").exists());
}

#[test]
fn a_statement_goes_to_the_account_whose_iban_it_prints() {
    let (dir, db) = open_with_accounts();
    let mut statement = demo_month(3, 2026, 3, 141_230);
    statement.iban = "AT94 1999 9000 0000 0002".into();

    let pending = prepared(&db, dir.path(), &statement);

    assert_eq!(pending.account.id, "account-2");
    assert_eq!(pending.file, "statements/2026/household-003.pdf");
}

#[test]
fn bytes_that_are_not_a_pdf_are_refused() {
    let (dir, db) = open_with_accounts();
    let message = refusal(import::prepare(&db, dir.path(), b"just some text".to_vec()));
    assert_eq!(message, "The data isn't a readable PDF");
}

#[test]
fn a_pdf_that_is_not_a_statement_is_refused() {
    let (dir, db) = open_with_accounts();
    let page = vec![Run {
        x: 50.0,
        y: 700.0,
        size: 12.0,
        font: Font::Helvetica,
        text: "Shopping list: bread, milk".into(),
    }];
    let message = refusal(import::prepare(&db, dir.path(), write::write(&[page])));
    assert!(
        message.starts_with("This isn't a statement Hamster can read"),
        "{message}"
    );
}

#[test]
fn a_tampered_statement_is_refused() {
    let (dir, db) = open_with_accounts();
    let message = refusal(import::prepare(
        &db,
        dir.path(),
        october().tampered_pdf(1, -90_000),
    ));
    assert!(
        message.starts_with("The statement doesn't add up."),
        "{message}"
    );
    assert_eq!(count(&db, "statements"), 0);
}

#[test]
fn a_statement_of_an_unknown_iban_names_settings_and_masks_the_iban() {
    let (dir, db) = open_with_accounts();
    let mut statement = october();
    statement.iban = "AT61 1904 3002 3457 3201".into();

    let message = refusal(import::prepare(&db, dir.path(), statement.pdf()));

    assert_eq!(
        message,
        "No account has the IBAN AT•• •••• 3201; add it in Settings → Accounts first"
    );
}

#[test]
fn the_same_file_again_is_refused_with_the_number_it_was_imported_as() {
    let (dir, db) = open_with_accounts();
    imported(&db, dir.path(), &october());

    let message = refusal(import::prepare(&db, dir.path(), october().pdf()));

    assert_eq!(message, "This file was already imported as Nr. 010/2026");
}

#[test]
fn another_file_with_the_same_account_and_number_is_refused() {
    let (dir, db) = open_with_accounts();
    imported(&db, dir.path(), &october());
    let mut other = october();
    other.lines[1].cents = -90_000;

    let message = refusal(import::prepare(&db, dir.path(), other.pdf()));

    assert_eq!(message, "Nr. 010/2026 of Private is already imported");
}

#[test]
fn the_same_number_of_another_account_is_not_a_duplicate() {
    let (dir, db) = open_with_accounts();
    imported(&db, dir.path(), &october());
    let mut other = october();
    other.iban = "AT94 1999 9000 0000 0002".into();
    other.lines[1].cents = -90_000;

    let pending = prepared(&db, dir.path(), &other);

    assert_eq!(pending.file, "statements/2026/household-010.pdf");
}

#[test]
fn a_taken_file_name_with_different_content_gets_a_numbered_suffix() {
    let (dir, db) = open_with_accounts();
    let folder = dir.path().join("statements/2026");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("private-010.pdf"), b"another file").unwrap();

    assert_eq!(
        prepared(&db, dir.path(), &october()).file,
        "statements/2026/private-010-2.pdf"
    );

    std::fs::write(folder.join("private-010-2.pdf"), b"yet another").unwrap();
    assert_eq!(
        prepared(&db, dir.path(), &october()).file,
        "statements/2026/private-010-3.pdf"
    );
}

#[test]
fn a_leftover_file_with_the_same_content_keeps_its_name() {
    let (dir, db) = open_with_accounts();
    let folder = dir.path().join("statements/2026");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("private-010.pdf"), october().pdf()).unwrap();

    assert_eq!(
        prepared(&db, dir.path(), &october()).file,
        "statements/2026/private-010.pdf"
    );
}

#[test]
fn committing_copies_the_pdf_unchanged_and_stores_the_statement() {
    let (dir, db) = open_with_accounts();
    let statement = october();
    let bytes = statement.pdf();
    let pending = import::prepare(&db, dir.path(), bytes.clone()).unwrap();
    let hash = pending.hash.clone();

    let (written, row) = import::commit(&db, dir.path(), pending, NOW, Source::Import).unwrap();

    assert_eq!(written.label, "Import Statement Nr. 010/2026");
    assert_eq!(
        std::fs::read(dir.path().join("statements/2026/private-010.pdf")).unwrap(),
        bytes
    );
    let expected = StatementRow {
        id: row.id.clone(),
        account_id: "account-1".into(),
        number: 10,
        year: 2026,
        from_date: "2026-10-01".into(),
        to_date: "2026-10-28".into(),
        old_balance: 141_230,
        credits: 285_000,
        debits: -112_688,
        new_balance: 313_542,
        layout: "sparkasse-erste-1".into(),
        file_hash: hash,
        file: "statements/2026/private-010.pdf".into(),
        pages: 1,
        imported_at: NOW,
    };
    assert_eq!(row, expected);
    assert_eq!(
        db.read(|c| store::statements_of(c, "account-1")).unwrap(),
        vec![expected]
    );
}

#[test]
fn committing_stores_one_transaction_per_line_in_printed_order() {
    let (dir, db) = open_with_accounts();
    let row = imported(&db, dir.path(), &october());

    let lines = lines_of(&db, &row.id);

    let shown: Vec<(&str, i64, &str)> = lines
        .iter()
        .map(|l| (l.text.as_str(), l.cents, l.value_date.as_str()))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("Gehalt", 285_000, "2026-10-01"),
            ("Dauerauftrag", -98_000, "2026-10-03"),
            (
                "POS 58,30 AT K1 04.10. 17:40 TANKSTELLE RHEIN LUSTENAU 040",
                -5_830,
                "2026-10-05"
            ),
            (
                "E-COMM 12,99 AT K1 06.10. STREAMFLIX WIEN 040",
                -1_299,
                "2026-10-07"
            ),
            (
                "E-COMM 24,00 USD K1 08.10. LUMEN AI SAN FRANCISCO 840",
                -2_248,
                "2026-10-10"
            ),
            ("Lastschrift", -4_690, "2026-10-14"),
            (
                "POS 6,21 AT K1 20.10. 09:12 CAFE LINDE DORNBIRN 040",
                -621,
                "2026-10-21"
            ),
        ]
    );
    for line in &lines {
        assert_eq!(line.statement_id, row.id);
        assert_eq!(line.account_id, "account-1");
        assert_eq!(line.page, 1);
    }
}

#[test]
fn a_transfer_line_keeps_its_counterparty_and_detail_lines() {
    let (dir, db) = open_with_accounts();
    let row = imported(&db, dir.path(), &october());

    let lines = lines_of(&db, &row.id);

    let salary = &lines[0];
    assert_eq!(
        salary.extra,
        r#"["NORDWIND SOFTWARE GMBH","AT94 1999 9000 0000 0099","REF: LOHN"]"#
    );
    assert_eq!(
        salary.counterparty.as_deref(),
        Some("NORDWIND SOFTWARE GMBH")
    );
    assert_eq!(
        salary.counterparty_iban.as_deref(),
        Some("AT941999900000000099")
    );
    assert_eq!(salary.card_kind, None);
    assert_eq!(salary.merchant, None);
}

#[test]
fn card_payments_store_their_kind_date_time_merchant_and_foreign_amount() {
    let (dir, db) = open_with_accounts();
    let row = imported(&db, dir.path(), &october());
    let lines = lines_of(&db, &row.id);
    let by_merchant = |merchant: &str| {
        lines
            .iter()
            .find(|l| l.merchant.as_deref() == Some(merchant))
            .unwrap_or_else(|| panic!("no line of {merchant}"))
    };

    let terminal = by_merchant("TANKSTELLE RHEIN LUSTENAU");
    assert_eq!(terminal.card_kind.as_deref(), Some("terminal"));
    assert_eq!(terminal.card_date.as_deref(), Some("2026-10-04"));
    assert_eq!(terminal.card_time.as_deref(), Some("17:40"));
    assert_eq!(terminal.counterparty, None);
    assert_eq!(
        (terminal.original_cents, &terminal.original_currency),
        (None, &None)
    );
    assert_eq!(
        (terminal.fx_fee, &terminal.fx_rate, &terminal.fx_date),
        (None, &None, &None)
    );

    let online = by_merchant("STREAMFLIX WIEN");
    assert_eq!(online.card_kind.as_deref(), Some("online"));
    assert_eq!(online.card_time, None);

    let foreign = by_merchant("LUMEN AI SAN FRANCISCO");
    assert_eq!(foreign.card_kind.as_deref(), Some("online"));
    assert_eq!(foreign.card_date.as_deref(), Some("2026-10-08"));
    assert_eq!(foreign.original_cents, Some(2_400));
    assert_eq!(foreign.original_currency.as_deref(), Some("USD"));
    assert_eq!(foreign.fx_fee, Some(49));
    assert_eq!(foreign.fx_rate.as_deref(), Some("1,0912"));
    assert_eq!(foreign.fx_date.as_deref(), Some("2026-10-06"));
}

#[test]
fn a_line_to_another_own_account_is_a_transfer_and_others_are_not() {
    let (dir, db) = open_with_accounts();
    let mut statement = october();
    statement.lines.insert(1, household_transfer());

    let row = imported(&db, dir.path(), &statement);

    let lines = lines_of(&db, &row.id);
    let transfers: Vec<Option<&str>> = lines
        .iter()
        .filter(|l| l.transfer)
        .map(|l| l.counterparty.as_deref())
        .collect();
    assert_eq!(transfers, vec![Some("HOUSEHOLD")]);
    assert_eq!(lines.iter().filter(|l| !l.transfer).count(), 7);
}

#[test]
fn a_line_to_an_account_that_is_not_the_owners_is_no_transfer() {
    let (dir, db) = open_with_accounts();
    db.write(
        Op::new(
            "Add account",
            vec![
                Change::insert(
                    accounts::TABLE,
                    &AccountRow {
                        id: "other".into(),
                        name: "Bookshop".into(),
                        bank: "Example Sparkasse".into(),
                        iban: "AT091999900000000077".into(),
                        own: false,
                        created_at: 10,
                    },
                )
                .unwrap(),
            ],
        ),
        Source::Ui,
    )
    .unwrap();

    let row = imported(&db, dir.path(), &october());

    assert!(lines_of(&db, &row.id).iter().all(|l| !l.transfer));
}

#[test]
fn undoing_an_import_removes_its_rows_and_keeps_the_copied_file() {
    let (dir, db) = open_with_accounts();
    imported(&db, dir.path(), &october());
    assert_eq!(count(&db, "statements"), 1);
    assert_eq!(count(&db, "transactions"), 7);

    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.label, "Import Statement Nr. 010/2026");
    assert_eq!(count(&db, "statements"), 0);
    assert_eq!(count(&db, "transactions"), 0);
    assert!(dir.path().join("statements/2026/private-010.pdf").exists());
}

#[test]
fn a_statement_can_be_imported_again_after_undo_and_reuses_the_kept_file() {
    let (dir, db) = open_with_accounts();
    imported(&db, dir.path(), &october());
    db.undo().unwrap();

    let pending = prepared(&db, dir.path(), &october());

    assert_eq!(pending.file, "statements/2026/private-010.pdf");
    import::commit(&db, dir.path(), pending, NOW, Source::Import).unwrap();
    assert_eq!(count(&db, "statements"), 1);
}

#[test]
fn the_first_statement_of_an_account_previews_with_its_balance_equation() {
    let (dir, db) = open_with_accounts();
    let pending = prepared(&db, dir.path(), &october());

    let preview = preview_of(&db, &pending);

    assert_eq!(
        preview,
        ImportPreview {
            token: "token-1".into(),
            file_name: "october.pdf".into(),
            summary: "Kontoauszug Nr. 010/2026 · 1 page · 1 Oct – 28 Oct 2026".into(),
            account_name: "Private".into(),
            lines: 7,
            balance: "1,412.30 + 2,850.00 − 1,126.88 = 3,135.42".into(),
            chain: ChainKind::First,
            chain_title: "The account's first statement".into(),
            chain_detail: "Later statements are checked against it.".into(),
            file: "statements/2026/private-010.pdf".into(),
        }
    );
}

#[test]
fn the_summary_follows_the_date_order_and_the_balance_the_number_format() {
    let (dir, db) = open_with_accounts();
    let pending = prepared(&db, dir.path(), &october());

    let preview = import::preview(
        &db,
        "t".into(),
        "october.pdf".into(),
        &pending,
        NumberFormat::CommaDecimal,
        DateOrder::MonthFirst,
    )
    .unwrap();

    assert_eq!(
        preview.summary,
        "Kontoauszug Nr. 010/2026 · 1 page · Oct 1 – Oct 28, 2026"
    );
    assert_eq!(preview.balance, "1.412,30 + 2.850,00 − 1.126,88 = 3.135,42");
}

#[test]
fn a_statement_that_continues_the_last_one_says_so() {
    let (dir, db) = open_with_accounts();
    let september = demo_month(9, 2026, 9, 141_230);
    imported(&db, dir.path(), &september);
    let october = demo_month(10, 2026, 10, september.new_balance());
    let pending = prepared(&db, dir.path(), &october);

    let preview = preview_of(&db, &pending);

    assert_eq!(preview.chain, ChainKind::Continues);
    assert_eq!(preview.chain_title, "Continues Nr. 009/2026 without a gap");
    assert_eq!(
        preview.chain_detail,
        "Old balance €3,135.42 equals the last new balance."
    );
}

#[test]
fn a_skipped_number_previews_as_a_gap() {
    let (dir, db) = open_with_accounts();
    let august = demo_month(8, 2026, 8, 141_230);
    imported(&db, dir.path(), &august);
    let october = demo_month(10, 2026, 10, august.new_balance());
    let pending = prepared(&db, dir.path(), &october);

    let preview = preview_of(&db, &pending);

    assert_eq!(preview.chain, ChainKind::Gap);
    assert_eq!(
        preview.chain_title,
        "Statements are missing before this one"
    );
    assert_eq!(
        preview.chain_detail,
        "Expected Nr. 009/2026 next; it imports, and the account shows the gap."
    );
}

#[test]
fn balances_that_do_not_connect_preview_as_a_gap() {
    let (dir, db) = open_with_accounts();
    let september = demo_month(9, 2026, 9, 141_230);
    imported(&db, dir.path(), &september);
    let october = demo_month(10, 2026, 10, 100_000);
    let pending = prepared(&db, dir.path(), &october);

    let preview = preview_of(&db, &pending);

    assert_eq!(preview.chain, ChainKind::Gap);
    assert_eq!(
        preview.chain_title,
        "The old balance doesn't continue the last statement"
    );
    assert_eq!(
        preview.chain_detail,
        "The last new balance was +€3,135.42; it imports, and the account shows the gap."
    );
}

#[test]
fn only_the_same_accounts_earlier_statements_count_for_the_chain() {
    let (dir, db) = open_with_accounts();
    let mut household = demo_month(9, 2026, 9, 141_230);
    household.iban = "AT94 1999 9000 0000 0002".into();
    imported(&db, dir.path(), &household);
    let pending = prepared(&db, dir.path(), &october());

    assert_eq!(preview_of(&db, &pending).chain, ChainKind::First);
}

#[test]
fn a_queued_statement_is_taken_once_by_its_token() {
    let (dir, db) = open_with_accounts();
    let queue = ImportQueue::default();
    let first = queue.put(prepared(&db, dir.path(), &october()));
    let second = queue.put(prepared(&db, dir.path(), &october()));

    assert_ne!(first, second);
    let taken = queue.take(&first).unwrap();
    assert_eq!(taken.file, "statements/2026/private-010.pdf");
    assert!(queue.take(&first).is_none());
    assert!(queue.take(&second).is_some());
    assert!(queue.take("no-such-token").is_none());
}
