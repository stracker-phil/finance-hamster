//! Documents against a temp database and folder: files moving with their facts, reading and adding an invoice
//! PDF, and the bank lines offered as its payment.

use std::path::Path;

use serde::Serialize;
use serde_json::json;
use tempfile::TempDir;

use super::add::{self, AddInput, AddPending, AddPreview};
use super::payments::LineChoice;
use super::store::{self, DocState, DocumentRow, LinkRow, VendorRow};
use super::{files, title_of};
use crate::date::DateOrder;
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::platform::pdf::write::{self, Font, Run};
use crate::rules::ItemOption;
use crate::settings::NumberFormat;
use crate::statements::file_hash;
use crate::statements::import_tests::open_with_accounts;
use crate::statements::store::{self as statements, StatementRow, TransactionRow};
use crate::taxyears::{self, TaxYearRow};
use crate::test_support::rules::{minimal_rules, rules_text, set};

const NOW: i64 = 1_700_000_000;
const DAY: &str = "2026-11-10";
const MOVED: &str = "docs/2026/2026-11-10-pixelforge-ltd.pdf";

const PIXELFORGE: [&str; 5] = [
    "Pixelforge Ltd.",
    "Invoice No: PF-2026-1109",
    "Date: 2026-11-09",
    "Subtotal USD 100.00",
    "Total USD 129.00",
];

/// The demo accounts and an open 2026 year with the items `books` and `phone` (40 % private).
fn setup() -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    let mut rules = minimal_rules(2026);
    set(
        &mut rules,
        "/items",
        json!([
            { "key": "books", "kz": "720", "name": "Books", "section": "main", "kind": "documents" },
            { "key": "phone", "kz": "721", "name": "Phone", "section": "main", "kind": "documents", "privateShare": 40 }
        ]),
    );
    let year = TaxYearRow::new(2026, "2026 v1".into(), rules_text(&rules), 1);
    insert(&db, taxyears::TABLE, &year);
    (dir, db)
}

fn insert(db: &Db, table: &str, row: &impl Serialize) {
    db.write(
        Op::new("Insert", vec![Change::insert(table, row).unwrap()]),
        Source::Seed,
    )
    .unwrap();
}

fn count(db: &Db, table: &str) -> i64 {
    db.read(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?))
        .unwrap()
}

fn pdf_with(lines: &[&str]) -> Vec<u8> {
    let page: Vec<Run> = lines
        .iter()
        .enumerate()
        .map(|(i, text)| Run {
            x: 50.0,
            y: 780.0 - i as f32 * 14.0,
            size: 10.0,
            font: Font::Courier,
            text: (*text).to_string(),
        })
        .collect();
    write::write(&[page])
}

fn vendor_row(id: &str, name: &str, default_item: Option<&str>) -> VendorRow {
    VendorRow {
        id: id.into(),
        name: name.into(),
        default_item_key: default_item.map(String::from),
        created_at: 1,
    }
}

fn blank_document(id: &str) -> DocumentRow {
    DocumentRow {
        id: id.into(),
        state: DocState::Draft,
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
    }
}

/// A document whose file sits in the inbox under `name`; `content` makes the bytes and the id `doc-<content>`.
fn stored_draft(
    db: &Db,
    dir: &Path,
    name: &str,
    content: &str,
    patch: impl FnOnce(&mut DocumentRow),
) -> DocumentRow {
    let bytes = format!("%PDF {content}").into_bytes();
    let hash = file_hash(&bytes);
    let mut doc = blank_document(&format!("doc-{content}"));
    doc.file = files::copy_in(dir, name, &bytes, &hash).unwrap();
    doc.original_name = name.into();
    doc.file_hash = hash;
    patch(&mut doc);
    insert(db, store::DOCUMENTS, &doc);
    doc
}

fn pixelforge_draft(db: &Db, dir: &Path, name: &str, content: &str) -> DocumentRow {
    stored_draft(db, dir, name, content, |d| {
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some(DAY.into());
    })
}

fn document(db: &Db, id: &str) -> DocumentRow {
    db.read(|c| store::document_by_id(c, id)).unwrap().unwrap()
}

fn exists(dir: &Path, relative: &str) -> bool {
    dir.join(relative).exists()
}

fn with_pixelforge_vendor(db: &Db) {
    insert(
        db,
        store::VENDORS,
        &vendor_row("vendor-1", "Pixelforge Ltd.", None),
    );
}

#[test]
fn a_document_is_titled_by_its_vendor_or_its_file_name() {
    let vendors = vec![vendor_row("vendor-1", "Pixelforge Ltd.", None)];
    let doc = |vendor: Option<&str>, name: &str| DocumentRow {
        vendor_id: vendor.map(String::from),
        original_name: name.into(),
        ..blank_document("doc-1")
    };

    for (document, expected) in [
        (doc(Some("vendor-1"), "nov.pdf"), "Pixelforge Ltd. invoice"),
        (doc(None, "scan-001.pdf"), "scan-001"),
        (doc(Some("gone"), "scan-002.pdf"), "scan-002"),
        (doc(None, "plain"), "plain"),
    ] {
        assert_eq!(title_of(&document, &vendors), expected, "{document:?}");
    }
}

#[test]
fn copy_in_writes_the_bytes_to_the_inbox_under_the_original_name() {
    let dir = crate::test_support::temp_config_dir();
    let bytes = b"%PDF one";

    let name = files::copy_in(dir.path(), "invoice.pdf", bytes, &file_hash(bytes)).unwrap();

    assert_eq!(name, "docs/inbox/invoice.pdf");
    assert_eq!(std::fs::read(dir.path().join(&name)).unwrap(), bytes);
}

#[test]
fn a_different_file_with_the_same_name_gets_a_number_and_the_same_file_keeps_its_name() {
    let dir = crate::test_support::temp_config_dir();
    let copy =
        |bytes: &[u8]| files::copy_in(dir.path(), "invoice.pdf", bytes, &file_hash(bytes)).unwrap();

    assert_eq!(copy(b"%PDF one"), "docs/inbox/invoice.pdf");
    assert_eq!(copy(b"%PDF two"), "docs/inbox/invoice-2.pdf");
    assert_eq!(copy(b"%PDF three"), "docs/inbox/invoice-3.pdf");
    assert_eq!(copy(b"%PDF two"), "docs/inbox/invoice-2.pdf");
    assert_eq!(
        std::fs::read(dir.path().join("docs/inbox/invoice-2.pdf")).unwrap(),
        b"%PDF two"
    );
}

#[test]
fn a_draft_with_a_payment_date_moves_to_its_year_folder_named_by_date_and_vendor() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    pixelforge_draft(&db, dir.path(), "scan.pdf", "one");

    let written = files::relocate(&db, dir.path()).unwrap().unwrap();

    assert_eq!(written.label, "Move Document Files");
    assert_eq!(document(&db, "doc-one").file, MOVED);
    assert!(exists(dir.path(), MOVED));
    assert!(!exists(dir.path(), "docs/inbox/scan.pdf"));
}

#[test]
fn without_a_vendor_the_original_name_without_extension_names_the_moved_file() {
    let (dir, db) = setup();
    stored_draft(&db, dir.path(), "scan-001.pdf", "one", |d| {
        d.pay_date = Some(DAY.into());
    });

    files::relocate(&db, dir.path()).unwrap();

    assert_eq!(
        document(&db, "doc-one").file,
        "docs/2026/2026-11-10-scan-001.pdf"
    );
}

#[test]
fn nothing_moves_while_every_file_is_where_its_facts_say() {
    let (dir, db) = setup();
    stored_draft(&db, dir.path(), "scan.pdf", "one", |_| {});
    assert!(files::relocate(&db, dir.path()).unwrap().is_none());

    pixelforge_draft(&db, dir.path(), "other.pdf", "two");
    with_pixelforge_vendor(&db);
    assert!(files::relocate(&db, dir.path()).unwrap().is_some());
    assert!(files::relocate(&db, dir.path()).unwrap().is_none());
}

#[test]
fn the_move_is_an_op_of_the_app_that_undo_does_not_reach() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    pixelforge_draft(&db, dir.path(), "scan.pdf", "one");

    files::relocate(&db, dir.path()).unwrap();

    assert_eq!(db.undo().unwrap(), None);
    assert_eq!(document(&db, "doc-one").file, MOVED);
}

#[test]
fn finished_and_excluded_documents_never_move() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    for (content, state) in [("one", DocState::Finished), ("two", DocState::Excluded)] {
        stored_draft(&db, dir.path(), &format!("{content}.pdf"), content, |d| {
            d.state = state;
            d.vendor_id = Some("vendor-1".into());
            d.pay_date = Some(DAY.into());
        });
    }

    assert!(files::relocate(&db, dir.path()).unwrap().is_none());
    assert_eq!(document(&db, "doc-one").file, "docs/inbox/one.pdf");
    assert_eq!(document(&db, "doc-two").file, "docs/inbox/two.pdf");
    assert!(exists(dir.path(), "docs/inbox/one.pdf"));
}

#[test]
fn a_taken_name_in_the_year_folder_gets_a_number() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    pixelforge_draft(&db, dir.path(), "a.pdf", "one");
    stored_draft(&db, dir.path(), "b.pdf", "two", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some(DAY.into());
        d.created_at = 2;
    });

    files::relocate(&db, dir.path()).unwrap();

    assert_eq!(document(&db, "doc-one").file, MOVED);
    assert_eq!(
        document(&db, "doc-two").file,
        "docs/2026/2026-11-10-pixelforge-ltd-2.pdf"
    );
    assert!(files::relocate(&db, dir.path()).unwrap().is_none());
}

#[test]
fn a_second_inbox_file_with_the_same_name_stays_where_it_is() {
    let (dir, db) = setup();
    stored_draft(&db, dir.path(), "invoice.pdf", "one", |_| {});
    stored_draft(&db, dir.path(), "invoice.pdf", "two", |d| d.created_at = 2);
    assert_eq!(document(&db, "doc-two").file, "docs/inbox/invoice-2.pdf");

    assert!(files::relocate(&db, dir.path()).unwrap().is_none());
    assert_eq!(document(&db, "doc-two").file, "docs/inbox/invoice-2.pdf");
}

#[test]
fn a_file_missing_from_its_path_is_found_by_its_hash_under_docs() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    pixelforge_draft(&db, dir.path(), "scan.pdf", "one");
    std::fs::create_dir_all(dir.path().join("docs/archive")).unwrap();
    std::fs::rename(
        dir.path().join("docs/inbox/scan.pdf"),
        dir.path().join("docs/archive/old.pdf"),
    )
    .unwrap();

    files::relocate(&db, dir.path()).unwrap();

    assert_eq!(document(&db, "doc-one").file, MOVED);
    assert!(exists(dir.path(), MOVED));
    assert!(!exists(dir.path(), "docs/archive/old.pdf"));
}

#[test]
fn a_draft_whose_file_is_gone_is_left_alone() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    pixelforge_draft(&db, dir.path(), "scan.pdf", "one");
    std::fs::remove_file(dir.path().join("docs/inbox/scan.pdf")).unwrap();

    assert!(files::relocate(&db, dir.path()).unwrap().is_none());
    assert_eq!(document(&db, "doc-one").file, "docs/inbox/scan.pdf");
}

#[test]
fn undoing_a_payment_date_edit_moves_the_file_back_on_the_next_relocate() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    let doc = stored_draft(&db, dir.path(), "scan.pdf", "one", |d| {
        d.vendor_id = Some("vendor-1".into());
    });
    let edited = DocumentRow {
        pay_date: Some(DAY.into()),
        ..doc.clone()
    };
    db.write(
        Op::new(
            "Edit Document",
            vec![Change::update(store::DOCUMENTS, &doc, &edited).unwrap()],
        ),
        Source::Ui,
    )
    .unwrap();
    files::relocate(&db, dir.path()).unwrap();
    assert_eq!(document(&db, "doc-one").file, MOVED);

    db.undo().unwrap();
    assert_eq!(document(&db, "doc-one").pay_date, None);
    files::relocate(&db, dir.path()).unwrap();

    assert_eq!(document(&db, "doc-one").file, "docs/inbox/scan.pdf");
    assert!(exists(dir.path(), "docs/inbox/scan.pdf"));
    assert!(!exists(dir.path(), MOVED));
}

fn pending(db: &Db, bytes: Vec<u8>, name: &str) -> AddPending {
    add::prepare(db, bytes, name.into()).unwrap()
}

fn pixelforge_pending(db: &Db) -> AddPending {
    pending(db, pdf_with(&PIXELFORGE), "pixelforge-nov.pdf")
}

fn refusal(result: AppResult<AddPending>) -> String {
    match result {
        Err(AppError::InvalidInput(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn input(vendor: &str, item: Option<&str>, line: Option<&str>) -> AddInput {
    AddInput {
        token: "token-1".into(),
        vendor: vendor.into(),
        item_key: item.map(String::from),
        line_id: line.map(String::from),
    }
}

fn commit(db: &Db, dir: &Path, pending: AddPending, input: &AddInput) -> AppResult<Written> {
    add::commit(db, dir, pending, input, NOW, Source::Ui)
}

fn failed_field(result: AppResult<Written>) -> String {
    match result {
        Err(AppError::InvalidField { field, .. }) => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn preview_in(db: &Db, pending: &AddPending, format: NumberFormat, order: DateOrder) -> AddPreview {
    add::preview(db, "token-1".into(), pending, format, order).unwrap()
}

fn preview_of(db: &Db, pending: &AddPending) -> AddPreview {
    preview_in(db, pending, NumberFormat::DotDecimal, DateOrder::DayFirst)
}

fn debit(id: &str, date: &str, cents: i64, text: &str) -> TransactionRow {
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

fn dollar_debit(id: &str, date: &str, cents: i64) -> TransactionRow {
    TransactionRow {
        merchant: Some("PIXELFORGE LTD".into()),
        original_cents: Some(12_900),
        original_currency: Some("USD".into()),
        ..debit(
            id,
            date,
            cents,
            "E-COMM 129,00 USD K1 09.11. PIXELFORGE LTD SAN FRANCISCO 840",
        )
    }
}

/// The lines as one imported statement of the Private account.
fn add_lines(db: &Db, lines: &[TransactionRow]) {
    let statement = StatementRow {
        id: "statement-1".into(),
        account_id: "account-1".into(),
        number: 11,
        year: 2026,
        from_date: "2026-11-01".into(),
        to_date: "2026-11-28".into(),
        old_balance: 0,
        credits: 0,
        debits: 0,
        new_balance: 0,
        layout: "sparkasse".into(),
        file_hash: "statement-hash".into(),
        file: "statements/2026/private-011.pdf".into(),
        pages: 1,
        imported_at: 1,
    };
    insert(db, statements::STATEMENTS, &statement);
    for line in lines {
        insert(db, statements::TRANSACTIONS, line);
    }
}

fn line_ids(preview: &AddPreview) -> Vec<&str> {
    preview.lines.iter().map(|l| l.id.as_str()).collect()
}

#[test]
fn only_a_pdf_is_read() {
    let (_dir, db) = setup();

    for (bytes, name) in [(&b"hello"[..], "notes.txt"), (&b""[..], "empty.pdf")] {
        assert_eq!(
            refusal(add::prepare(&db, bytes.to_vec(), name.into())),
            format!("{name} isn't a PDF")
        );
    }
}

#[test]
fn preparing_a_pdf_reads_its_facts_from_the_text() {
    let (_dir, db) = setup();
    let bytes = pdf_with(&PIXELFORGE);

    let pending = pending(&db, bytes.clone(), "pixelforge-nov.pdf");

    assert_eq!(pending.original_name, "pixelforge-nov.pdf");
    assert_eq!(pending.hash, file_hash(&bytes));
    assert_eq!(pending.bytes, bytes);
    assert_eq!(pending.guess.vendor.as_deref(), Some("Pixelforge Ltd."));
    assert_eq!(pending.guess.amount_cents, Some(12_900));
    assert_eq!(pending.guess.currency.as_deref(), Some("USD"));
    assert_eq!(
        pending.guess.invoice_number.as_deref(),
        Some("PF-2026-1109")
    );
    assert_eq!(
        pending.guess.invoice_date,
        chrono::NaiveDate::from_ymd_opt(2026, 11, 9)
    );
}

#[test]
fn a_known_vendor_in_the_text_is_preferred_to_the_first_line() {
    let (_dir, db) = setup();
    insert(
        &db,
        store::VENDORS,
        &vendor_row("vendor-2", "Nordbook", None),
    );
    let bytes = pdf_with(&[
        "Order confirmation",
        "sold by NORDBOOK verlag",
        "Total 24,90 EUR",
    ]);

    let pending = pending(&db, bytes, "order.pdf");

    assert_eq!(pending.guess.vendor.as_deref(), Some("Nordbook"));
    assert_eq!(pending.guess.amount_cents, Some(2_490));
}

#[test]
fn a_pdf_already_added_is_refused_with_its_title() {
    let (dir, db) = setup();
    let bytes = pdf_with(&PIXELFORGE);
    let first = pending(&db, bytes.clone(), "pixelforge-nov.pdf");
    commit(
        &db,
        dir.path(),
        first,
        &input("Pixelforge Ltd.", None, None),
    )
    .unwrap();

    assert_eq!(
        refusal(add::prepare(&db, bytes, "copy.pdf".into())),
        "Already added: Pixelforge Ltd. invoice"
    );
}

#[test]
fn an_unknown_vendor_is_suggested_from_the_pdf_with_its_invoice_facts() {
    let (_dir, db) = setup();
    let pending = pixelforge_pending(&db);

    assert_eq!(
        preview_of(&db, &pending),
        AddPreview {
            token: "token-1".into(),
            file_name: "pixelforge-nov.pdf".into(),
            near_duplicate: None,
            vendor: Some("Pixelforge Ltd.".into()),
            vendor_from_pdf: true,
            invoice: Some("PF-2026-1109 · 9 Nov 2026".into()),
            invoice_amount: Some("USD 129.00".into()),
            items: vec![
                ItemOption {
                    key: "books".into(),
                    label: "KZ 720 · Books".into(),
                    kz: "720".into()
                },
                ItemOption {
                    key: "phone".into(),
                    label: "KZ 721 · Phone".into(),
                    kz: "721".into()
                },
            ],
            item_key: None,
            item_why: None,
            lines: vec![],
        }
    );
}

#[test]
fn the_preview_reads_dates_and_amounts_in_the_chosen_formats() {
    let (_dir, db) = setup();
    let pending = pixelforge_pending(&db);

    let preview = preview_in(
        &db,
        &pending,
        NumberFormat::CommaDecimal,
        DateOrder::MonthFirst,
    );

    assert_eq!(
        preview.invoice.as_deref(),
        Some("PF-2026-1109 · Nov 9, 2026")
    );
    assert_eq!(preview.invoice_amount.as_deref(), Some("USD 129,00"));
}

#[test]
fn an_invoice_without_a_printed_currency_shows_euro() {
    let (_dir, db) = setup();
    let pending = pending(&db, pdf_with(&["Nordbook", "Total 5,00"]), "nordbook.pdf");

    let preview = preview_of(&db, &pending);

    assert_eq!(preview.invoice_amount.as_deref(), Some("€5.00"));
    assert_eq!(preview.invoice, None);
}

#[test]
fn a_pdf_without_readable_facts_gives_an_empty_preview() {
    let (_dir, db) = setup();
    let pending = pending(&db, pdf_with(&["12345"]), "scan.pdf");

    let preview = preview_of(&db, &pending);

    assert_eq!(preview.vendor, None);
    assert!(!preview.vendor_from_pdf);
    assert_eq!(preview.invoice, None);
    assert_eq!(preview.invoice_amount, None);
    assert_eq!(preview.lines, vec![]);
}

#[test]
fn a_known_vendor_is_not_marked_as_read_from_the_pdf_and_offers_its_default_item() {
    let (_dir, db) = setup();
    insert(
        &db,
        store::VENDORS,
        &vendor_row("vendor-1", "Pixelforge Ltd.", Some("phone")),
    );

    let preview = preview_of(&db, &pixelforge_pending(&db));

    assert_eq!(preview.vendor.as_deref(), Some("Pixelforge Ltd."));
    assert!(!preview.vendor_from_pdf);
    assert_eq!(preview.item_key.as_deref(), Some("phone"));
    assert_eq!(
        preview.item_why.as_deref(),
        Some("default for Pixelforge Ltd.")
    );
}

#[test]
fn without_a_default_the_item_of_the_vendors_newest_document_with_an_item_is_offered() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    for (id, item, created_at) in [
        ("doc-1", Some("books"), 1),
        ("doc-2", Some("phone"), 2),
        ("doc-3", None, 3),
    ] {
        insert(
            &db,
            store::DOCUMENTS,
            &DocumentRow {
                vendor_id: Some("vendor-1".into()),
                item_key: item.map(String::from),
                created_at,
                ..blank_document(id)
            },
        );
    }

    let preview = preview_of(&db, &pixelforge_pending(&db));

    assert_eq!(preview.item_key.as_deref(), Some("phone"));
    assert_eq!(
        preview.item_why.as_deref(),
        Some("last used for Pixelforge Ltd.")
    );
}

#[test]
fn the_same_vendor_and_amount_within_three_days_looks_like_a_duplicate() {
    struct Existing {
        invoice_cents: Option<i64>,
        invoice_date: Option<&'static str>,
        paid_cents: Option<i64>,
        pay_date: Option<&'static str>,
    }
    let existing = |invoice_cents, invoice_date, paid_cents, pay_date| Existing {
        invoice_cents,
        invoice_date,
        paid_cents,
        pay_date,
    };
    let message = "Looks like Pixelforge Ltd. invoice: same vendor and amount a few days apart";

    for (existing, expected) in [
        (
            existing(Some(12_900), Some("2026-11-08"), None, None),
            Some(message),
        ),
        (
            existing(Some(12_900), Some("2026-11-12"), None, None),
            Some(message),
        ),
        (existing(Some(12_900), Some("2026-11-13"), None, None), None),
        (existing(Some(12_800), Some("2026-11-09"), None, None), None),
        (existing(Some(12_900), None, None, None), None),
        (
            existing(None, None, Some(12_900), Some("2026-11-10")),
            Some(message),
        ),
    ] {
        let (_dir, db) = setup();
        with_pixelforge_vendor(&db);
        insert(
            &db,
            store::DOCUMENTS,
            &DocumentRow {
                vendor_id: Some("vendor-1".into()),
                invoice_cents: existing.invoice_cents,
                invoice_date: existing.invoice_date.map(String::from),
                paid_cents: existing.paid_cents,
                pay_date: existing.pay_date.map(String::from),
                ..blank_document("doc-1")
            },
        );

        let preview = preview_of(&db, &pixelforge_pending(&db));

        assert_eq!(
            preview.near_duplicate.as_deref(),
            expected,
            "{:?} {:?} {:?} {:?}",
            existing.invoice_cents,
            existing.invoice_date,
            existing.paid_cents,
            existing.pay_date
        );
    }
}

#[test]
fn a_similar_document_of_another_vendor_is_no_duplicate() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    insert(
        &db,
        store::VENDORS,
        &vendor_row("vendor-2", "Nordbook", None),
    );
    insert(
        &db,
        store::DOCUMENTS,
        &DocumentRow {
            vendor_id: Some("vendor-2".into()),
            invoice_cents: Some(12_900),
            invoice_date: Some("2026-11-09".into()),
            ..blank_document("doc-1")
        },
    );

    assert_eq!(
        preview_of(&db, &pixelforge_pending(&db)).near_duplicate,
        None
    );
}

#[test]
fn a_foreign_invoice_is_matched_by_original_amount_and_currency_and_the_vendors_name() {
    let (_dir, db) = setup();
    insert(&db, store::DOCUMENTS, &blank_document("doc-linked"));
    add_lines(
        &db,
        &[
            dollar_debit("l-fx", "2026-11-10", -11_871),
            debit(
                "l-eur",
                "2026-11-09",
                -12_900,
                "Lastschrift OFFICE SUPPLIES",
            ),
            debit(
                "l-token",
                "2026-11-12",
                -3_000,
                "E-COMM PIXELFORGE SUBSCRIPTION",
            ),
            dollar_debit("l-far", "2026-11-25", -11_871),
            dollar_debit("l-credit", "2026-11-10", 11_871),
            dollar_debit("l-linked", "2026-11-11", -11_871),
        ],
    );
    insert(
        &db,
        store::LINKS,
        &LinkRow {
            id: "link-1".into(),
            document_id: "doc-linked".into(),
            transaction_id: "l-linked".into(),
            created_at: 1,
        },
    );

    let preview = preview_of(&db, &pixelforge_pending(&db));

    assert_eq!(
        preview.lines,
        vec![
            LineChoice {
                id: "l-fx".into(),
                title: "PIXELFORGE LTD".into(),
                detail: "Private · Tue, 10 Nov · USD 129.00".into(),
                amount: "\u{2212}€118.71".into(),
            },
            LineChoice {
                id: "l-token".into(),
                title: "E-COMM PIXELFORGE SUBSCRIPTION".into(),
                detail: "Private · Thu, 12 Nov".into(),
                amount: "\u{2212}€30.00".into(),
            },
        ]
    );
}

#[test]
fn a_euro_invoice_is_matched_by_the_booked_amount_and_at_most_three_lines_are_offered_closest_first()
 {
    let (_dir, db) = setup();
    add_lines(
        &db,
        &[
            debit("far", "2026-11-19", -12_000, "Lastschrift SHOP D"),
            debit("near", "2026-11-12", -12_000, "Lastschrift SHOP C"),
            debit("exact", "2026-11-09", -12_000, "Lastschrift SHOP A"),
            debit("other", "2026-11-09", -12_001, "Lastschrift SHOP E"),
            debit("next", "2026-11-10", -12_000, "Lastschrift SHOP B"),
            debit("later", "2026-11-15", -12_000, "Lastschrift SHOP F"),
        ],
    );
    let bytes = pdf_with(&["Nordbook", "Date: 2026-11-09", "Total 120,00 EUR"]);

    let preview = preview_of(&db, &pending(&db, bytes, "nordbook.pdf"));

    assert_eq!(line_ids(&preview), ["exact", "next", "near"]);
}

#[test]
fn without_an_invoice_date_only_a_line_with_the_same_amount_is_offered() {
    let (_dir, db) = setup();
    add_lines(
        &db,
        &[
            debit("same", "2026-11-09", -12_000, "Lastschrift SHOP"),
            debit("token", "2026-11-09", -500, "Lastschrift NORDBOOK VERLAG"),
        ],
    );
    let bytes = pdf_with(&["Nordbook", "Total 120,00 EUR"]);

    let preview = preview_of(&db, &pending(&db, bytes, "nordbook.pdf"));

    assert_eq!(line_ids(&preview), ["same"]);
}

#[test]
fn an_add_preview_and_input_travel_in_camel_case() {
    let (_dir, db) = setup();
    let preview = preview_of(&db, &pixelforge_pending(&db));

    assert_eq!(
        serde_json::to_value(&preview).unwrap(),
        json!({
            "token": "token-1",
            "fileName": "pixelforge-nov.pdf",
            "nearDuplicate": null,
            "vendor": "Pixelforge Ltd.",
            "vendorFromPdf": true,
            "invoice": "PF-2026-1109 · 9 Nov 2026",
            "invoiceAmount": "USD 129.00",
            "items": [
                { "key": "books", "label": "KZ 720 · Books", "kz": "720" },
                { "key": "phone", "label": "KZ 721 · Phone", "kz": "721" }
            ],
            "itemKey": null,
            "itemWhy": null,
            "lines": []
        })
    );
    assert_eq!(
        serde_json::from_value::<AddInput>(json!({
            "token": "token-1",
            "vendor": "Pixelforge Ltd.",
            "itemKey": "phone",
            "lineId": null
        }))
        .unwrap(),
        input("Pixelforge Ltd.", Some("phone"), None)
    );
}

#[test]
fn adding_with_a_line_stores_a_draft_a_new_vendor_and_the_link_in_one_op() {
    let (dir, db) = setup();
    add_lines(&db, &[dollar_debit("l-fx", "2026-11-10", -11_871)]);
    let bytes = pdf_with(&PIXELFORGE);
    let pending = pending(&db, bytes.clone(), "pixelforge-nov.pdf");

    let written = commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", Some("phone"), Some("l-fx")),
    )
    .unwrap();

    assert_eq!(written.label, "Add Document Pixelforge Ltd.");
    let vendors = db.read(store::vendors).unwrap();
    assert_eq!(vendors.len(), 1);
    assert_eq!(
        vendors[0],
        VendorRow {
            id: vendors[0].id.clone(),
            name: "Pixelforge Ltd.".into(),
            default_item_key: None,
            created_at: NOW,
        }
    );
    let documents = db.read(store::documents).unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(
        documents[0],
        DocumentRow {
            id: documents[0].id.clone(),
            state: DocState::Draft,
            file: "docs/inbox/pixelforge-nov.pdf".into(),
            original_name: "pixelforge-nov.pdf".into(),
            file_hash: file_hash(&bytes),
            vendor_id: Some(vendors[0].id.clone()),
            pay_date: Some("2026-11-10".into()),
            paid_cents: None,
            item_key: Some("phone".into()),
            work_share: 60,
            note: String::new(),
            invoice_cents: Some(12_900),
            invoice_currency: Some("USD".into()),
            invoice_number: Some("PF-2026-1109".into()),
            invoice_date: Some("2026-11-09".into()),
            created_at: NOW,
            finished_at: None,
            excluded_at: None,
            exclude_reason: None,
        }
    );
    let links = db.read(store::links).unwrap();
    assert_eq!(
        links,
        vec![LinkRow {
            id: links[0].id.clone(),
            document_id: documents[0].id.clone(),
            transaction_id: "l-fx".into(),
            created_at: NOW,
        }]
    );
    assert_eq!(
        std::fs::read(dir.path().join("docs/inbox/pixelforge-nov.pdf")).unwrap(),
        bytes
    );
}

#[test]
fn adding_without_a_line_or_item_leaves_the_payment_date_open_and_the_whole_amount_to_work() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);

    commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", None, None),
    )
    .unwrap();

    let doc = db.read(store::documents).unwrap().remove(0);
    assert_eq!(doc.pay_date, None);
    assert_eq!(doc.item_key, None);
    assert_eq!(doc.work_share, 100);
    assert_eq!(count(&db, store::LINKS), 0);
}

#[test]
fn an_item_without_a_private_share_counts_fully() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);

    commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", Some("books"), None),
    )
    .unwrap();

    assert_eq!(db.read(store::documents).unwrap()[0].work_share, 100);
}

#[test]
fn an_invoice_amount_without_a_printed_currency_is_stored_in_euro_and_none_stays_none() {
    for (lines, cents, currency) in [
        (vec!["Nordbook", "Total 5,00"], Some(500), Some("EUR")),
        (vec!["12345"], None, None),
    ] {
        let (dir, db) = setup();
        let pending = pending(&db, pdf_with(&lines), "scan.pdf");

        commit(&db, dir.path(), pending, &input("Nordbook", None, None)).unwrap();

        let doc = db.read(store::documents).unwrap().remove(0);
        assert_eq!(doc.invoice_cents, cents, "{lines:?}");
        assert_eq!(doc.invoice_currency.as_deref(), currency, "{lines:?}");
    }
}

#[test]
fn a_known_vendor_is_reused_ignoring_case_and_surrounding_spaces() {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    let pending = pixelforge_pending(&db);

    let written = commit(
        &db,
        dir.path(),
        pending,
        &input("  pixelforge ltd.  ", None, None),
    )
    .unwrap();

    assert_eq!(written.label, "Add Document pixelforge ltd.");
    assert_eq!(count(&db, store::VENDORS), 1);
    assert_eq!(
        db.read(store::documents).unwrap()[0].vendor_id.as_deref(),
        Some("vendor-1")
    );
}

#[test]
fn a_blank_vendor_is_refused_before_anything_is_written() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);

    let result = commit(&db, dir.path(), pending, &input("   ", None, None));

    assert_eq!(failed_field(result), "document.vendor");
    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert!(!exists(dir.path(), "docs"));
}

#[test]
fn an_item_the_year_does_not_offer_is_refused_before_anything_is_written() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);

    let result = commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", Some("nope"), None),
    );

    assert_eq!(failed_field(result), "document.item");
    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(count(&db, store::VENDORS), 0);
    assert!(!exists(dir.path(), "docs"));
}

#[test]
fn a_line_that_does_not_exist_is_not_found() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);

    let result = commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", None, Some("missing")),
    );

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
    assert_eq!(count(&db, store::DOCUMENTS), 0);
}

#[test]
fn undoing_an_add_removes_its_rows_and_keeps_the_file() {
    let (dir, db) = setup();
    add_lines(&db, &[dollar_debit("l-fx", "2026-11-10", -11_871)]);
    let pending = pixelforge_pending(&db);
    commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", Some("phone"), Some("l-fx")),
    )
    .unwrap();

    db.undo().unwrap();

    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(count(&db, store::VENDORS), 0);
    assert_eq!(count(&db, store::LINKS), 0);
    assert!(exists(dir.path(), "docs/inbox/pixelforge-nov.pdf"));
}
