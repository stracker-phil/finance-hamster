//! Documents against a temp database and folder: files moving with their facts, reading and adding an invoice
//! PDF, and the bank lines offered as its payment.

use std::path::Path;

use serde::Serialize;
use serde_json::json;
use tempfile::TempDir;

use super::add::{self, AddInput, AddPending, AddPreview};
use super::amounts::Estimate;
use super::edit::{self, DocumentEdit};
use super::history::{self, HistoryKind};
use super::lifecycle::{self, ArchivedDocument, DeletionRow, DeletionView};
use super::list::{DocumentGroup, DocumentRowView};
use super::payments::{self, LineChoice};
use super::store::{self, DocState, DocumentRow, LinkRow, VendorRow};
use super::vendors::{self, VendorView, VendorsView};
use super::view::{self, DepreciationView, DocumentView, SliceView, VendorOption};
use super::{files, list, title_of};
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
        useful_life: None,
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
            useful_life: None,
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

const FORMAT: NumberFormat = NumberFormat::DotDecimal;
const ORDER: DateOrder = DateOrder::DayFirst;

fn draft(db: &Db, id: &str, patch: impl FnOnce(&mut DocumentRow)) -> DocumentRow {
    let mut doc = blank_document(id);
    patch(&mut doc);
    insert(db, store::DOCUMENTS, &doc);
    doc
}

fn link(db: &Db, document_id: &str, line_id: &str) {
    insert(
        db,
        store::LINKS,
        &LinkRow {
            id: format!("link-{document_id}-{line_id}"),
            document_id: document_id.into(),
            transaction_id: line_id.into(),
            created_at: 1,
        },
    );
}

fn page(db: &Db, id: &str) -> DocumentView {
    view::document(db, id, FORMAT, ORDER).unwrap()
}

fn apply(db: &Db, id: &str, change: DocumentEdit) -> AppResult<Option<Written>> {
    edit::edit(db, id, &change, NOW, Source::Ui)
}

fn rejected(result: AppResult<Option<Written>>) -> (String, String) {
    match result {
        Err(AppError::InvalidField { field, message }) => (field, message),
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn two_october_lines(db: &Db) {
    add_lines(
        db,
        &[
            debit("l-1", "2026-10-14", -4_690, "Lastschrift SEITENWEISE"),
            debit("l-2", "2026-10-20", -1_000, "Lastschrift SEITENWEISE"),
        ],
    );
}

fn items_of_setup() -> Vec<ItemOption> {
    vec![
        ItemOption {
            key: "books".into(),
            label: "KZ 720 · Books".into(),
            kz: "720".into(),
        },
        ItemOption {
            key: "phone".into(),
            label: "KZ 721 · Phone".into(),
            kz: "721".into(),
        },
    ]
}

fn pay_date_edit(value: &str) -> DocumentEdit {
    DocumentEdit::PayDate {
        value: Some(value.into()),
    }
}

#[test]
fn a_document_that_does_not_exist_is_not_found_by_page_options_and_edit() {
    let (_dir, db) = setup();

    assert!(matches!(
        view::document(&db, "nope", FORMAT, ORDER),
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        view::line_options(&db, "nope", "", FORMAT, ORDER),
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        apply(&db, "nope", DocumentEdit::Note { text: "x".into() }),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn the_page_of_a_dated_document_shows_its_lines_amount_vendor_items_and_estimate() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    insert(
        &db,
        store::VENDORS,
        &vendor_row("vendor-2", "Nordbook", None),
    );
    two_october_lines(&db);
    draft(&db, "doc-a", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some("2026-10-14".into());
        d.item_key = Some("phone".into());
        d.work_share = 60;
        d.note = "Seminar".into();
        d.invoice_cents = Some(4_690);
    });
    link(&db, "doc-a", "l-2");
    link(&db, "doc-a", "l-1");

    assert_eq!(
        page(&db, "doc-a"),
        DocumentView {
            id: "doc-a".into(),
            title: "Pixelforge Ltd. invoice".into(),
            state: DocState::Draft,
            file: "docs/inbox/doc-a.pdf".into(),
            pay_date: Some("2026-10-14".into()),
            year: Some(2026),
            lines: vec![
                LineChoice {
                    id: "l-1".into(),
                    title: "Lastschrift SEITENWEISE".into(),
                    detail: "Private · Wed, 14 Oct".into(),
                    amount: "\u{2212}€46.90".into(),
                },
                LineChoice {
                    id: "l-2".into(),
                    title: "Lastschrift SEITENWEISE".into(),
                    detail: "Private · Tue, 20 Oct".into(),
                    amount: "\u{2212}€10.00".into(),
                },
            ],
            paid: Some("€56.90".into()),
            paid_from_lines: true,
            paid_text: None,
            vendor: Some("Pixelforge Ltd.".into()),
            vendors: vec![
                VendorOption {
                    name: "Nordbook".into(),
                    detail: "0 documents".into()
                },
                VendorOption {
                    name: "Pixelforge Ltd.".into(),
                    detail: "1 document".into()
                },
            ],
            items: items_of_setup(),
            item_key: Some("phone".into()),
            work_share: 60,
            estimate: Some(Estimate {
                work_share: 60,
                share_hint: Some("from the item: 40 % private".into()),
                anv: "€34.14".into(),
                formula: "€56.90 × 60 %".into(),
                gwg: Some(
                    "€34.14 is below the GWG limit (€1,000.00): counts fully in 2026.".into()
                ),
            }),
            note: "Seminar".into(),
            invoice_amount: Some("46.90".into()),
            invoice_currency: "EUR".into(),
            missing: vec![],
            finished_on: None,
            excluded_on: None,
            exclude_reason: None,
            depreciation: None,
        }
    );
}

#[test]
fn an_empty_document_lists_every_missing_fact_in_order_and_has_no_items_or_estimate() {
    let (_dir, db) = setup();
    draft(&db, "doc-b", |_| {});

    let page = page(&db, "doc-b");

    assert_eq!(
        page.missing,
        ["payment date", "amount paid", "vendor", "item"]
    );
    assert_eq!(page.title, "doc-b");
    assert_eq!(page.year, None);
    assert_eq!(page.items, vec![]);
    assert_eq!(page.estimate, None);
    assert_eq!(page.paid, None);
    assert!(!page.paid_from_lines);
    assert_eq!(page.invoice_amount, None);
    assert_eq!(page.invoice_currency, "EUR");
}

#[test]
fn each_fact_leaves_the_missing_list_once_it_is_set() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);

    for (patch, expected) in [
        (
            (Some("2026-10-14"), None, None, None),
            vec!["amount paid", "vendor", "item"],
        ),
        (
            (None, Some(4_690), None, None),
            vec!["payment date", "vendor", "item"],
        ),
        (
            (None, None, Some("vendor-1"), None),
            vec!["payment date", "amount paid", "item"],
        ),
        (
            (None, None, None, Some("books")),
            vec!["payment date", "amount paid", "vendor"],
        ),
    ] {
        let id = format!("doc-{expected:?}");
        draft(&db, &id, |d| {
            d.pay_date = patch.0.map(String::from);
            d.paid_cents = patch.1;
            d.vendor_id = patch.2.map(String::from);
            d.item_key = patch.3.map(String::from);
        });

        assert_eq!(page(&db, &id).missing, expected, "{patch:?}");
    }
}

#[test]
fn a_typed_amount_is_the_paid_amount_without_lines_and_reads_in_the_number_format() {
    let (_dir, db) = setup();
    draft(&db, "doc-c", |d| d.paid_cents = Some(4_690));

    let dot = page(&db, "doc-c");
    let comma = view::document(&db, "doc-c", NumberFormat::CommaDecimal, ORDER).unwrap();

    assert_eq!(dot.paid.as_deref(), Some("€46.90"));
    assert_eq!(dot.paid_text.as_deref(), Some("46.90"));
    assert!(!dot.paid_from_lines);
    assert_eq!(comma.paid.as_deref(), Some("€46,90"));
    assert_eq!(comma.paid_text.as_deref(), Some("46,90"));
    assert!(dot.estimate.is_some());
}

#[test]
fn linked_lines_override_a_typed_amount_as_the_paid_amount() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    let doc = draft(&db, "doc-d", |d| d.paid_cents = Some(1));
    link(&db, "doc-d", "l-1");
    let lines = view::linked_lines(&db, "doc-d").unwrap();

    assert_eq!(view::paid_cents(&doc, &lines), Some(4_690));
    assert_eq!(view::paid_cents(&doc, &[]), Some(1));
    assert_eq!(
        view::paid_cents(
            &DocumentRow {
                paid_cents: None,
                ..doc
            },
            &[]
        ),
        None
    );
}

#[test]
fn an_invoice_in_a_foreign_currency_shows_its_code_and_plain_amount() {
    let (_dir, db) = setup();
    draft(&db, "doc-e", |d| {
        d.invoice_cents = Some(12_900);
        d.invoice_currency = Some("USD".into());
    });

    let page = page(&db, "doc-e");

    assert_eq!(page.invoice_amount.as_deref(), Some("129.00"));
    assert_eq!(page.invoice_currency, "USD");
}

#[test]
fn the_items_come_from_the_payment_years_rules_or_else_the_newest_open_year() {
    let (_dir, db) = setup();
    let mut rules = minimal_rules(2025);
    set(&mut rules, "/items/0/name", json!("Old Books"));
    insert(
        &db,
        taxyears::TABLE,
        &TaxYearRow::new(2025, "2025 v1".into(), rules_text(&rules), 1),
    );
    for (id, date) in [("in-2025", "2025-06-01"), ("in-2024", "2024-06-01")] {
        draft(&db, id, |d| d.pay_date = Some(date.into()));
    }

    assert_eq!(
        page(&db, "in-2025").items,
        vec![ItemOption {
            key: "books".into(),
            label: "KZ 720 · Old Books".into(),
            kz: "720".into(),
        }]
    );
    assert_eq!(page(&db, "in-2024").items, items_of_setup());
}

#[test]
fn the_vendor_choices_count_their_documents() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    insert(
        &db,
        store::VENDORS,
        &vendor_row("vendor-2", "Nordbook", None),
    );
    for id in ["one", "two"] {
        draft(&db, id, |d| d.vendor_id = Some("vendor-1".into()));
    }

    assert_eq!(
        page(&db, "one").vendors,
        vec![
            VendorOption {
                name: "Nordbook".into(),
                detail: "0 documents".into()
            },
            VendorOption {
                name: "Pixelforge Ltd.".into(),
                detail: "2 documents".into()
            },
        ]
    );
}

#[test]
fn a_document_page_travels_in_camel_case() {
    let (_dir, db) = setup();
    draft(&db, "doc-f", |_| {});

    assert_eq!(
        serde_json::to_value(page(&db, "doc-f")).unwrap(),
        json!({
            "id": "doc-f",
            "title": "doc-f",
            "state": "draft",
            "file": "docs/inbox/doc-f.pdf",
            "payDate": null,
            "year": null,
            "lines": [],
            "paid": null,
            "paidFromLines": false,
            "paidText": null,
            "vendor": null,
            "vendors": [],
            "items": [],
            "itemKey": null,
            "workShare": 100,
            "estimate": null,
            "note": "",
            "invoiceAmount": null,
            "invoiceCurrency": "EUR",
            "missing": ["payment date", "amount paid", "vendor", "item"],
            "finishedOn": null,
            "excludedOn": null,
            "excludeReason": null,
            "depreciation": null
        })
    );
}

#[test]
fn without_a_query_the_line_options_match_the_documents_invoice_facts() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    add_lines(
        &db,
        &[
            dollar_debit("l-fx", "2026-11-10", -11_871),
            debit("l-other", "2026-11-10", -500, "Lastschrift OFFICE SUPPLIES"),
        ],
    );
    draft(&db, "doc-a", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.invoice_cents = Some(12_900);
        d.invoice_currency = Some("USD".into());
        d.invoice_date = Some("2026-11-09".into());
    });

    let options = view::line_options(&db, "doc-a", "  ", FORMAT, ORDER).unwrap();

    assert_eq!(
        options,
        vec![LineChoice {
            id: "l-fx".into(),
            title: "PIXELFORGE LTD".into(),
            detail: "Private · Tue, 10 Nov · USD 129.00".into(),
            amount: "\u{2212}€118.71".into(),
        }]
    );
}

#[test]
fn without_an_invoice_the_line_options_use_the_paid_amount_and_payment_date() {
    let (_dir, db) = setup();
    add_lines(
        &db,
        &[
            debit("near", "2026-11-12", -12_000, "Lastschrift SHOP A"),
            debit("far", "2026-11-30", -12_000, "Lastschrift SHOP B"),
            debit("other", "2026-11-10", -11_000, "Lastschrift SHOP C"),
        ],
    );
    draft(&db, "doc-a", |d| {
        d.paid_cents = Some(12_000);
        d.pay_date = Some("2026-11-10".into());
    });

    let options = view::line_options(&db, "doc-a", "", FORMAT, ORDER).unwrap();

    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["near"]);
}

#[test]
fn a_query_searches_debit_lines_by_text_newest_first_without_linked_ones() {
    let (_dir, db) = setup();
    add_lines(
        &db,
        &[
            debit("old", "2026-11-01", -300, "Lastschrift CAFE ALPEN"),
            debit("new", "2026-11-05", -400, "Lastschrift CAFE ALPEN"),
            debit("credit", "2026-11-06", 500, "Gutschrift CAFE ALPEN"),
            debit("taken", "2026-11-07", -600, "Lastschrift CAFE ALPEN"),
            debit("miss", "2026-11-08", -700, "Lastschrift SHOP"),
        ],
    );
    draft(&db, "doc-other", |_| {});
    link(&db, "doc-other", "taken");
    draft(&db, "doc-a", |_| {});

    let options = view::line_options(&db, "doc-a", " cafe ", FORMAT, ORDER).unwrap();

    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["new", "old"]);
}

#[test]
fn a_search_shows_at_most_twenty_lines_and_reads_the_detail_lines_too() {
    let (_dir, db) = setup();
    let mut lines: Vec<TransactionRow> = (1..=25)
        .map(|day| {
            debit(
                &format!("shop-{day:02}"),
                &format!("2026-11-{day:02}"),
                -100,
                "Lastschrift SHOP",
            )
        })
        .collect();
    lines.push(TransactionRow {
        extra: r#"["Ref ABC123"]"#.into(),
        ..debit("ref", "2026-10-01", -100, "Lastschrift")
    });
    add_lines(&db, &lines);

    let shown = payments::search(&db, "shop").unwrap();
    let by_detail = payments::search(&db, "abc123").unwrap();

    assert_eq!(shown.len(), 20);
    assert_eq!(shown[0].id, "shop-25");
    assert_eq!(shown[19].id, "shop-06");
    assert_eq!(by_detail.len(), 1);
    assert_eq!(by_detail[0].id, "ref");
}

#[test]
fn a_line_is_shown_as_the_picker_shows_it_and_an_unknown_id_is_none() {
    let (_dir, db) = setup();
    two_october_lines(&db);

    assert_eq!(
        payments::choice_of(&db, "l-1", FORMAT, ORDER).unwrap(),
        Some(LineChoice {
            id: "l-1".into(),
            title: "Lastschrift SEITENWEISE".into(),
            detail: "Private · Wed, 14 Oct".into(),
            amount: "\u{2212}€46.90".into(),
        })
    );
    assert_eq!(
        payments::choice_of(&db, "nope", FORMAT, ORDER).unwrap(),
        None
    );
}

#[test]
fn account_names_are_listed_by_account_id() {
    let (_dir, db) = setup();

    assert_eq!(
        payments::account_names(&db).unwrap(),
        std::collections::HashMap::from([
            ("account-1".to_string(), "Private".to_string()),
            ("account-2".to_string(), "Household".to_string()),
            ("account-3".to_string(), "Business".to_string()),
        ])
    );
}

#[test]
fn only_a_draft_can_be_edited_and_the_refusal_says_what_to_do() {
    for (state, message) in [
        (
            DocState::Finished,
            "This document is finished; unlock it to change it",
        ),
        (
            DocState::Excluded,
            "This document is excluded; restore it to change it",
        ),
    ] {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| d.state = state);
        let before = count(&db, "journal");

        let result = apply(&db, "doc-a", DocumentEdit::Note { text: "x".into() });

        match result {
            Err(AppError::InvalidInput(text)) => assert_eq!(text, message),
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert_eq!(count(&db, "journal"), before, "{message}");
        assert_eq!(document(&db, "doc-a").note, "", "{message}");
    }
}

#[test]
fn the_payment_date_is_stored_as_iso_and_a_blank_clears_it() {
    for (typed, expected) in [
        (Some("2026-10-14"), Some("2026-10-14")),
        (Some("  2026-10-14 "), Some("2026-10-14")),
        (Some(""), None),
        (Some("   "), None),
        (None, None),
    ] {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| d.pay_date = Some("2026-01-01".into()));

        apply(
            &db,
            "doc-a",
            DocumentEdit::PayDate {
                value: typed.map(String::from),
            },
        )
        .unwrap();

        assert_eq!(
            document(&db, "doc-a").pay_date.as_deref(),
            expected,
            "{typed:?}"
        );
    }
}

#[test]
fn a_payment_date_that_is_no_date_is_refused() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    for typed in ["next tuesday", "2026-13-40"] {
        assert_eq!(
            rejected(apply(&db, "doc-a", pay_date_edit(typed))),
            ("document.payDate".into(), "This isn't a date".into()),
            "{typed}"
        );
    }
    assert_eq!(document(&db, "doc-a").pay_date, None);
}

#[test]
fn the_amount_paid_is_typed_without_sign_and_a_blank_clears_it() {
    for (typed, expected) in [
        ("46.90", Some(4_690)),
        ("-12.50", Some(1_250)),
        ("€ 12", Some(1_200)),
        ("", None),
    ] {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| d.paid_cents = Some(100));

        apply(
            &db,
            "doc-a",
            DocumentEdit::Paid {
                value: Some(typed.into()),
            },
        )
        .unwrap();

        assert_eq!(document(&db, "doc-a").paid_cents, expected, "{typed:?}");
    }
}

#[test]
fn the_amount_paid_is_refused_while_lines_are_linked_or_when_it_is_no_amount() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "linked", |_| {});
    link(&db, "linked", "l-1");
    draft(&db, "typed", |_| {});

    assert_eq!(
        rejected(apply(
            &db,
            "linked",
            DocumentEdit::Paid {
                value: Some("5".into())
            }
        )),
        (
            "document.paid".into(),
            "The linked bank line is the amount paid".into()
        )
    );
    assert_eq!(
        rejected(apply(
            &db,
            "typed",
            DocumentEdit::Paid {
                value: Some("abc".into())
            }
        )),
        ("document.paid".into(), "This isn't an amount".into())
    );
}

#[test]
fn a_new_vendor_name_creates_the_vendor_in_the_same_op_and_undo_removes_both() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    let written = apply(
        &db,
        "doc-a",
        DocumentEdit::Vendor {
            name: "  Nordbook ".into(),
        },
    )
    .unwrap()
    .unwrap();

    assert_eq!(written.label, "Edit doc-a");
    let vendors = db.read(store::vendors).unwrap();
    assert_eq!(
        vendors,
        vec![VendorRow {
            id: vendors[0].id.clone(),
            name: "Nordbook".into(),
            default_item_key: None,
            created_at: NOW,
        }]
    );
    assert_eq!(
        document(&db, "doc-a").vendor_id.as_deref(),
        Some(vendors[0].id.as_str())
    );

    db.undo().unwrap();

    assert_eq!(count(&db, store::VENDORS), 0);
    assert_eq!(document(&db, "doc-a").vendor_id, None);
}

#[test]
fn an_existing_vendor_is_reused_ignoring_case_and_the_edit_is_labelled_by_the_documents_title() {
    let (_dir, db) = setup();
    with_pixelforge_vendor(&db);
    draft(&db, "doc-a", |_| {});

    let first = apply(
        &db,
        "doc-a",
        DocumentEdit::Vendor {
            name: "PIXELFORGE LTD.".into(),
        },
    )
    .unwrap()
    .unwrap();
    let second = apply(
        &db,
        "doc-a",
        DocumentEdit::Vendor {
            name: "Pixelforge Ltd.".into(),
        },
    )
    .unwrap();

    assert_eq!(first.label, "Edit doc-a");
    assert_eq!(second, None);
    assert_eq!(count(&db, store::VENDORS), 1);
    assert_eq!(
        document(&db, "doc-a").vendor_id.as_deref(),
        Some("vendor-1")
    );
    let again = apply(&db, "doc-a", DocumentEdit::Note { text: "x".into() })
        .unwrap()
        .unwrap();
    assert_eq!(again.label, "Edit Pixelforge Ltd. invoice");
}

#[test]
fn a_blank_vendor_name_is_refused() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    assert_eq!(
        rejected(apply(
            &db,
            "doc-a",
            DocumentEdit::Vendor { name: "  ".into() }
        )),
        ("document.vendor".into(), "Name the vendor".into())
    );
}

#[test]
fn choosing_an_item_sets_its_default_work_share_and_a_blank_clears_the_item() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));
    let choose = |key: Option<&str>| DocumentEdit::Item {
        key: key.map(String::from),
    };

    apply(&db, "doc-a", choose(Some("phone"))).unwrap();
    let phone = document(&db, "doc-a");
    apply(&db, "doc-a", choose(Some("books"))).unwrap();
    let books = document(&db, "doc-a");
    apply(&db, "doc-a", choose(Some(""))).unwrap();
    let cleared = document(&db, "doc-a");

    assert_eq!(
        (phone.item_key.as_deref(), phone.work_share),
        (Some("phone"), 60)
    );
    assert_eq!(
        (books.item_key.as_deref(), books.work_share),
        (Some("books"), 100)
    );
    assert_eq!(cleared.item_key, None);
    assert_eq!(apply(&db, "doc-a", choose(None)).unwrap(), None);
}

#[test]
fn an_item_needs_a_payment_date_and_must_be_one_of_the_years_items() {
    let (_dir, db) = setup();
    draft(&db, "undated", |_| {});
    draft(&db, "dated", |d| d.pay_date = Some("2026-10-14".into()));
    let choose = |key: &str| DocumentEdit::Item {
        key: Some(key.into()),
    };

    assert_eq!(
        rejected(apply(&db, "undated", choose("books"))),
        ("document.item".into(), "Set the payment date first".into())
    );
    assert_eq!(
        rejected(apply(&db, "dated", choose("nope"))),
        (
            "document.item".into(),
            "Choose one of the year's items".into()
        )
    );
}

#[test]
fn the_work_share_is_a_percent_between_zero_and_one_hundred() {
    for (value, ok) in [
        (0, true),
        (60, true),
        (100, true),
        (-1, false),
        (101, false),
    ] {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| d.work_share = 50);

        let result = apply(&db, "doc-a", DocumentEdit::WorkShare { value });

        if ok {
            result.unwrap();
            assert_eq!(document(&db, "doc-a").work_share, value);
        } else {
            assert_eq!(
                rejected(result),
                (
                    "document.workShare".into(),
                    "A share between 0 and 100 %".into()
                ),
                "{value}"
            );
            assert_eq!(document(&db, "doc-a").work_share, 50);
        }
    }
}

#[test]
fn the_note_is_stored_as_typed() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    apply(
        &db,
        "doc-a",
        DocumentEdit::Note {
            text: " Seminar fee\nreceipt ".into(),
        },
    )
    .unwrap();

    assert_eq!(document(&db, "doc-a").note, " Seminar fee\nreceipt ");
}

#[test]
fn the_invoice_amount_and_currency_are_stored_together_in_upper_case() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    apply(
        &db,
        "doc-a",
        DocumentEdit::Invoice {
            amount: Some("-129.00".into()),
            currency: " usd ".into(),
        },
    )
    .unwrap();
    let typed = document(&db, "doc-a");
    apply(
        &db,
        "doc-a",
        DocumentEdit::Invoice {
            amount: Some(" ".into()),
            currency: "eur".into(),
        },
    )
    .unwrap();
    let cleared = document(&db, "doc-a");

    assert_eq!(
        (typed.invoice_cents, typed.invoice_currency.as_deref()),
        (Some(12_900), Some("USD"))
    );
    assert_eq!(
        (cleared.invoice_cents, cleared.invoice_currency),
        (None, None)
    );
}

#[test]
fn an_invoice_needs_a_three_letter_currency_and_a_real_amount() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    for (amount, currency, message) in [
        ("1", "US", "A three-letter currency code"),
        ("1", "EURO", "A three-letter currency code"),
        ("1", "U$D", "A three-letter currency code"),
        ("1", "12E", "A three-letter currency code"),
        ("abc", "USD", "This isn't an amount"),
    ] {
        assert_eq!(
            rejected(apply(
                &db,
                "doc-a",
                DocumentEdit::Invoice {
                    amount: Some(amount.into()),
                    currency: currency.into()
                }
            )),
            ("document.invoice".into(), message.into()),
            "{amount} {currency}"
        );
    }
}

#[test]
fn linking_a_line_adds_the_link_dates_an_undated_document_and_drops_a_typed_amount() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.paid_cents = Some(900));

    let written = apply(&db, "doc-a", DocumentEdit::Link { line: "l-1".into() })
        .unwrap()
        .unwrap();
    apply(&db, "doc-a", DocumentEdit::Link { line: "l-2".into() }).unwrap();

    assert_eq!(written.label, "Edit doc-a");
    let doc = document(&db, "doc-a");
    assert_eq!(doc.pay_date.as_deref(), Some("2026-10-14"));
    assert_eq!(doc.paid_cents, None);
    let mut linked: Vec<String> = db
        .read(store::links)
        .unwrap()
        .into_iter()
        .map(|l| l.transaction_id)
        .collect();
    linked.sort();
    assert_eq!(linked, ["l-1", "l-2"]);
    assert_eq!(page(&db, "doc-a").paid.as_deref(), Some("€56.90"));
}

#[test]
fn linking_keeps_an_existing_payment_date() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.pay_date = Some("2026-11-01".into()));

    apply(&db, "doc-a", DocumentEdit::Link { line: "l-1".into() }).unwrap();

    assert_eq!(
        document(&db, "doc-a").pay_date.as_deref(),
        Some("2026-11-01")
    );
}

#[test]
fn a_line_that_is_missing_or_already_linked_cannot_be_linked() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |_| {});
    link(&db, "doc-a", "l-1");

    assert!(matches!(
        apply(
            &db,
            "doc-a",
            DocumentEdit::Link {
                line: "gone".into()
            }
        ),
        Err(AppError::NotFound(_))
    ));
    assert_eq!(
        rejected(apply(
            &db,
            "doc-a",
            DocumentEdit::Link { line: "l-1".into() }
        )),
        (
            "document.lines".into(),
            "This line is linked already".into()
        )
    );
    assert_eq!(count(&db, store::LINKS), 1);
}

#[test]
fn unlinking_deletes_the_link_and_a_line_that_is_not_linked_is_not_found() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));
    link(&db, "doc-a", "l-1");

    assert!(matches!(
        apply(&db, "doc-a", DocumentEdit::Unlink { line: "l-2".into() }),
        Err(AppError::NotFound(_))
    ));
    let written = apply(&db, "doc-a", DocumentEdit::Unlink { line: "l-1".into() })
        .unwrap()
        .unwrap();

    assert_eq!(written.label, "Edit doc-a");
    assert_eq!(count(&db, store::LINKS), 0);
    assert_eq!(
        document(&db, "doc-a").pay_date.as_deref(),
        Some("2026-10-14")
    );
}

#[test]
fn an_edit_that_changes_nothing_writes_nothing() {
    let edits = [
        pay_date_edit("2026-10-15"),
        DocumentEdit::Paid {
            value: Some("46.90".into()),
        },
        DocumentEdit::Vendor {
            name: "Nordbook".into(),
        },
        DocumentEdit::Item {
            key: Some("phone".into()),
        },
        DocumentEdit::WorkShare { value: 70 },
        DocumentEdit::Note {
            text: "memo".into(),
        },
        DocumentEdit::Invoice {
            amount: Some("12.00".into()),
            currency: "usd".into(),
        },
        DocumentEdit::UsefulLife { years: Some(5) },
    ];
    for change in edits {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));
        assert!(apply(&db, "doc-a", change.clone()).unwrap().is_some());
        let before = count(&db, "journal");

        assert_eq!(
            apply(&db, "doc-a", change.clone()).unwrap(),
            None,
            "{change:?}"
        );
        assert_eq!(count(&db, "journal"), before, "{change:?}");
    }
}

#[test]
fn undoing_a_link_removes_it_and_restores_the_payment_date() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.paid_cents = Some(900));
    apply(&db, "doc-a", DocumentEdit::Link { line: "l-1".into() }).unwrap();

    db.undo().unwrap();

    assert_eq!(count(&db, store::LINKS), 0);
    let doc = document(&db, "doc-a");
    assert_eq!((doc.pay_date, doc.paid_cents), (None, Some(900)));
}

#[test]
fn document_edits_are_read_from_the_pages_tagged_json() {
    for (wire, expected) in [
        (
            json!({ "field": "payDate", "value": "2026-10-14" }),
            pay_date_edit("2026-10-14"),
        ),
        (
            json!({ "field": "paid", "value": null }),
            DocumentEdit::Paid { value: None },
        ),
        (
            json!({ "field": "vendor", "name": "Nordbook" }),
            DocumentEdit::Vendor {
                name: "Nordbook".into(),
            },
        ),
        (
            json!({ "field": "item", "key": null }),
            DocumentEdit::Item { key: None },
        ),
        (
            json!({ "field": "workShare", "value": 60 }),
            DocumentEdit::WorkShare { value: 60 },
        ),
        (
            json!({ "field": "note", "text": "memo" }),
            DocumentEdit::Note {
                text: "memo".into(),
            },
        ),
        (
            json!({ "field": "invoice", "amount": "12.00", "currency": "USD" }),
            DocumentEdit::Invoice {
                amount: Some("12.00".into()),
                currency: "USD".into(),
            },
        ),
        (
            json!({ "field": "usefulLife", "years": 5 }),
            DocumentEdit::UsefulLife { years: Some(5) },
        ),
        (
            json!({ "field": "usefulLife", "years": null }),
            DocumentEdit::UsefulLife { years: None },
        ),
        (
            json!({ "field": "link", "line": "l-1" }),
            DocumentEdit::Link { line: "l-1".into() },
        ),
        (
            json!({ "field": "unlink", "line": "l-1" }),
            DocumentEdit::Unlink { line: "l-1".into() },
        ),
    ] {
        assert_eq!(
            serde_json::from_value::<DocumentEdit>(wire.clone()).unwrap(),
            expected,
            "{wire}"
        );
    }
}

fn listed(db: &Db, query: &str) -> Vec<DocumentGroup> {
    list::documents(db, query, FORMAT, ORDER).unwrap()
}

fn listed_ids(groups: &[DocumentGroup]) -> Vec<&str> {
    groups
        .iter()
        .flat_map(|g| g.documents.iter().map(|d| d.id.as_str()))
        .collect()
}

fn row_view(id: &str, title: &str, sub: &str, state: DocState, anv: &str) -> DocumentRowView {
    DocumentRowView {
        id: id.into(),
        title: title.into(),
        sub: sub.into(),
        state,
        anv: anv.into(),
    }
}

fn with_listed_documents(db: &Db) {
    with_pixelforge_vendor(db);
    draft(db, "i", |_| {});
    draft(db, "a", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.original_name = "nov-scan.pdf".into();
        d.pay_date = Some("2026-10-14".into());
        d.paid_cents = Some(4_690);
        d.item_key = Some("phone".into());
        d.work_share = 60;
        d.note = "Seminar".into();
    });
    draft(db, "b", |d| {
        d.state = DocState::Finished;
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some("2026-11-10".into());
        d.paid_cents = Some(1_000);
        d.item_key = Some("books".into());
        d.invoice_number = Some("RE-77".into());
        d.invoice_cents = Some(12_900);
    });
    draft(db, "c", |d| {
        d.original_name = "Beleg-Maerz.pdf".into();
        d.pay_date = Some("2025-03-01".into());
        d.item_key = Some("books".into());
    });
    draft(db, "x", |d| {
        d.state = DocState::Excluded;
        d.pay_date = Some("2026-12-01".into());
        d.paid_cents = Some(500);
    });
}

#[test]
fn documents_are_grouped_inbox_first_then_years_newest_first_and_newest_payment_first() {
    let (_dir, db) = setup();
    with_listed_documents(&db);

    assert_eq!(
        listed(&db, ""),
        vec![
            DocumentGroup {
                title: "Inbox".into(),
                year: None,
                count: "1 document".into(),
                total: None,
                documents: vec![row_view("i", "i", "Not dated", DocState::Draft, "")],
            },
            DocumentGroup {
                title: "Tax year 2026".into(),
                year: Some(2026),
                count: "2 documents".into(),
                total: Some("€38.14 paid".into()),
                documents: vec![
                    row_view(
                        "b",
                        "Pixelforge Ltd. invoice",
                        "10 Nov · KZ 720 Books",
                        DocState::Finished,
                        "€10.00"
                    ),
                    row_view(
                        "a",
                        "Pixelforge Ltd. invoice",
                        "14 Oct · KZ 721 Phone",
                        DocState::Draft,
                        "€28.14"
                    ),
                ],
            },
            DocumentGroup {
                title: "Tax year 2025".into(),
                year: Some(2025),
                count: "1 document".into(),
                total: Some("€0.00 paid".into()),
                documents: vec![row_view(
                    "c",
                    "Beleg-Maerz",
                    "1 Mar · KZ 720 Books",
                    DocState::Draft,
                    ""
                )],
            },
        ]
    );
}

#[test]
fn the_sub_line_reads_the_date_in_the_chosen_order() {
    let (_dir, db) = setup();
    with_listed_documents(&db);

    let groups = list::documents(&db, "seminar", FORMAT, DateOrder::MonthFirst).unwrap();

    assert_eq!(groups[0].documents[0].sub, "Oct 14 · KZ 721 Phone");
}

#[test]
fn the_query_matches_title_file_name_note_invoice_number_and_amounts_ignoring_case() {
    let (_dir, db) = setup();
    with_listed_documents(&db);

    for (query, expected) in [
        ("PIXELFORGE", vec!["b", "a"]),
        ("NOV-SCAN", vec!["a"]),
        ("beleg", vec!["c"]),
        ("seMinar", vec!["a"]),
        ("re-77", vec!["b"]),
        ("129.00", vec!["b"]),
        ("46.90", vec!["a"]),
        ("  ", vec!["i", "b", "a", "c"]),
        ("zzz", vec![]),
    ] {
        assert_eq!(listed_ids(&listed(&db, query)), expected, "{query:?}");
    }
}

#[test]
fn nothing_found_gives_no_groups_and_amounts_are_searched_as_shown() {
    let (_dir, db) = setup();
    with_listed_documents(&db);

    assert_eq!(listed(&db, "zzz"), vec![]);
    let comma = list::documents(&db, "46,90", NumberFormat::CommaDecimal, ORDER).unwrap();
    assert_eq!(listed_ids(&comma), ["a"]);
    assert_eq!(comma[0].documents[0].anv, "€28,14");
}

#[test]
fn a_linked_line_is_the_amount_a_documents_row_counts() {
    let (_dir, db) = setup();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.paid_cents = Some(1));
    link(&db, "doc-a", "l-1");

    let groups = listed(&db, "");

    assert_eq!(groups[0].total, None);
    assert_eq!(
        groups[0].documents,
        vec![row_view(
            "doc-a",
            "doc-a",
            "Not dated",
            DocState::Draft,
            "€46.90"
        )]
    );
}

#[test]
fn the_document_groups_travel_in_camel_case() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |_| {});

    assert_eq!(
        serde_json::to_value(listed(&db, "")).unwrap(),
        json!([{
            "title": "Inbox",
            "year": null,
            "count": "1 document",
            "total": null,
            "documents": [{
                "id": "doc-a",
                "title": "doc-a",
                "sub": "Not dated",
                "state": "draft",
                "anv": ""
            }]
        }])
    );
}

fn with_vendors_and_documents(db: &Db) {
    for (id, name, default_item) in [
        ("v-pixel", "Pixelforge Ltd.", None),
        ("v-nord", "Nordbook", Some("books")),
        ("v-unused", "Unused Co", None),
        ("v-zeta", "Zeta", None),
    ] {
        insert(db, store::VENDORS, &vendor_row(id, name, default_item));
    }
    for (id, vendor, item, created_at) in [
        ("d1", "v-pixel", Some("phone"), 1),
        ("d2", "v-pixel", Some("books"), 2),
        ("d3", "v-pixel", None, 3),
        ("d4", "v-nord", Some("phone"), 4),
        ("d5", "v-zeta", Some("legacy"), 5),
    ] {
        draft(db, id, |d| {
            d.vendor_id = Some(vendor.into());
            d.item_key = item.map(String::from);
            d.created_at = created_at;
        });
    }
}

fn vendor_names(db: &Db) -> Vec<String> {
    db.read(store::vendors)
        .unwrap()
        .into_iter()
        .map(|v| v.name)
        .collect()
}

#[test]
fn the_vendor_list_shows_documents_and_the_default_or_last_used_item() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    assert_eq!(
        vendors::view(&db).unwrap(),
        VendorsView {
            vendors: vec![
                VendorView {
                    id: "v-nord".into(),
                    name: "Nordbook".into(),
                    detail: "1 document · KZ 720".into(),
                    default_item_key: Some("books".into()),
                    unused: false,
                },
                VendorView {
                    id: "v-pixel".into(),
                    name: "Pixelforge Ltd.".into(),
                    detail: "3 documents · KZ 720".into(),
                    default_item_key: None,
                    unused: false,
                },
                VendorView {
                    id: "v-unused".into(),
                    name: "Unused Co".into(),
                    detail: "0 documents".into(),
                    default_item_key: None,
                    unused: true,
                },
                VendorView {
                    id: "v-zeta".into(),
                    name: "Zeta".into(),
                    detail: "1 document · legacy".into(),
                    default_item_key: None,
                    unused: false,
                },
            ],
            items: items_of_setup(),
        }
    );
}

#[test]
fn the_vendor_view_travels_in_camel_case() {
    let (_dir, db) = setup();
    insert(
        &db,
        store::VENDORS,
        &vendor_row("v-1", "Nordbook", Some("books")),
    );

    assert_eq!(
        serde_json::to_value(vendors::view(&db).unwrap()).unwrap(),
        json!({
            "vendors": [{
                "id": "v-1",
                "name": "Nordbook",
                "detail": "0 documents · KZ 720",
                "defaultItemKey": "books",
                "unused": true
            }],
            "items": [
                { "key": "books", "label": "KZ 720 · Books", "kz": "720" },
                { "key": "phone", "label": "KZ 721 · Phone", "kz": "721" }
            ]
        })
    );
}

#[test]
fn renaming_a_vendor_trims_the_name_and_is_labelled_by_the_old_name() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    let written = vendors::rename(&db, "v-pixel", "  Pixelforge GmbH ", Source::Ui).unwrap();

    assert_eq!(written.label, "Rename Vendor Pixelforge Ltd.");
    assert_eq!(
        vendor_names(&db),
        ["Nordbook", "Pixelforge GmbH", "Unused Co", "Zeta"]
    );
    db.undo().unwrap();
    assert_eq!(
        vendor_names(&db),
        ["Nordbook", "Pixelforge Ltd.", "Unused Co", "Zeta"]
    );
}

#[test]
fn a_vendor_may_change_the_case_of_its_own_name() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    vendors::rename(&db, "v-nord", "NORDBOOK", Source::Ui).unwrap();

    assert!(vendor_names(&db).contains(&"NORDBOOK".to_string()));
}

#[test]
fn a_vendor_name_must_not_be_empty_or_taken_by_another_vendor() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    for (name, message) in [
        ("   ", "The name can't be empty"),
        (
            "nordbook",
            "Another vendor has this name; merge them instead",
        ),
    ] {
        match vendors::rename(&db, "v-pixel", name, Source::Ui) {
            Err(AppError::InvalidField {
                field,
                message: text,
            }) => {
                assert_eq!((field.as_str(), text.as_str()), ("vendor.name", message));
            }
            other => panic!("expected a refusal for {name:?}, got {other:?}"),
        }
    }
    assert!(matches!(
        vendors::rename(&db, "gone", "Any", Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn a_default_item_is_set_cleared_and_must_be_one_of_the_items() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);
    let default_of = |db: &Db, id: &str| {
        db.read(store::vendors)
            .unwrap()
            .into_iter()
            .find(|v| v.id == id)
            .unwrap()
            .default_item_key
    };

    let written =
        vendors::set_default_item(&db, "v-unused", Some("phone".into()), Source::Ui).unwrap();
    assert_eq!(written.label, "Set Default Item of Unused Co");
    assert_eq!(default_of(&db, "v-unused").as_deref(), Some("phone"));

    vendors::set_default_item(&db, "v-unused", None, Source::Ui).unwrap();
    assert_eq!(default_of(&db, "v-unused"), None);

    match vendors::set_default_item(&db, "v-unused", Some("nope".into()), Source::Ui) {
        Err(AppError::InvalidField { field, .. }) => assert_eq!(field, "vendor.item"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(default_of(&db, "v-unused"), None);
}

#[test]
fn merging_moves_the_documents_removes_the_vendor_and_undo_restores_both_in_one_step() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    let written = vendors::merge(&db, "v-pixel", "v-nord", Source::Ui).unwrap();

    assert_eq!(written.label, "Merge Pixelforge Ltd. into Nordbook");
    assert_eq!(vendor_names(&db), ["Nordbook", "Unused Co", "Zeta"]);
    for id in ["d1", "d2", "d3", "d4"] {
        assert_eq!(
            document(&db, id).vendor_id.as_deref(),
            Some("v-nord"),
            "{id}"
        );
    }
    assert_eq!(document(&db, "d5").vendor_id.as_deref(), Some("v-zeta"));

    db.undo().unwrap();

    assert_eq!(
        vendor_names(&db),
        ["Nordbook", "Pixelforge Ltd.", "Unused Co", "Zeta"]
    );
    for id in ["d1", "d2", "d3"] {
        assert_eq!(
            document(&db, id).vendor_id.as_deref(),
            Some("v-pixel"),
            "{id}"
        );
    }
}

#[test]
fn a_vendor_cannot_merge_into_itself_or_with_a_vendor_that_does_not_exist() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    match vendors::merge(&db, "v-pixel", "v-pixel", Source::Ui) {
        Err(AppError::InvalidInput(text)) => assert_eq!(text, "A vendor can't merge into itself"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(matches!(
        vendors::merge(&db, "v-pixel", "gone", Source::Ui),
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        vendors::merge(&db, "gone", "v-pixel", Source::Ui),
        Err(AppError::NotFound(_))
    ));
    assert_eq!(count(&db, store::VENDORS), 4);
}

#[test]
fn a_vendor_with_documents_cannot_be_deleted_and_an_unused_one_can() {
    let (_dir, db) = setup();
    with_vendors_and_documents(&db);

    match vendors::delete(&db, "v-pixel", Source::Ui) {
        Err(AppError::InvalidInput(text)) => assert_eq!(
            text,
            "Pixelforge Ltd. has documents; merge it into another vendor instead"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(count(&db, store::VENDORS), 4);

    let written = vendors::delete(&db, "v-unused", Source::Ui).unwrap();

    assert_eq!(written.label, "Delete Vendor Unused Co");
    assert_eq!(vendor_names(&db), ["Nordbook", "Pixelforge Ltd.", "Zeta"]);
    db.undo().unwrap();
    assert_eq!(count(&db, store::VENDORS), 4);
}

const WEEK: i64 = 7 * 24 * 60 * 60 * 1000;

fn noon(year: i32, month: u32, day: u32) -> i64 {
    use chrono::TimeZone;
    chrono::Utc
        .with_ymd_and_hms(year, month, day, 12, 0, 0)
        .unwrap()
        .timestamp_millis()
}

fn setup_with_vendor() -> (TempDir, Db) {
    let (dir, db) = setup();
    with_pixelforge_vendor(&db);
    (dir, db)
}

/// A draft with every required fact set.
fn complete_draft(db: &Db, id: &str) -> DocumentRow {
    draft(db, id, |d| {
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some("2026-10-14".into());
        d.paid_cents = Some(4_690);
        d.item_key = Some("books".into());
    })
}

fn excluded(db: &Db, id: &str, at: i64) -> DocumentRow {
    draft(db, id, |d| {
        d.state = DocState::Excluded;
        d.excluded_at = Some(at);
        d.exclude_reason = Some("Duplicate".into());
    })
}

fn excluded_with_file(db: &Db, dir: &Path, at: i64) -> DocumentRow {
    stored_draft(db, dir, "scan.pdf", "one", |d| {
        d.state = DocState::Excluded;
        d.excluded_at = Some(at);
        d.exclude_reason = Some("Duplicate".into());
        d.vendor_id = Some("vendor-1".into());
        d.created_at = noon(2026, 3, 2);
    })
}

fn refused(result: AppResult<Written>) -> String {
    match result {
        Err(AppError::InvalidInput(message)) => message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn erase(
    db: &Db,
    dir: &Path,
    id: &str,
    reason: &str,
    understood: bool,
    now: i64,
) -> AppResult<Written> {
    lifecycle::delete(db, dir, id, reason, understood, 7, now)
}

fn entries_of(db: &Db, id: &str) -> Vec<(HistoryKind, String)> {
    history::document_history(db, id, FORMAT, ORDER)
        .unwrap()
        .into_iter()
        .map(|e| (e.kind, e.text))
        .collect()
}

#[test]
fn finishing_a_complete_draft_locks_it_and_undo_makes_it_a_draft_again() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");

    let written = lifecycle::finish(&db, "doc-a", noon(2026, 11, 13), Source::Ui).unwrap();

    assert_eq!(written.label, "Finish Pixelforge Ltd. invoice");
    let doc = document(&db, "doc-a");
    assert_eq!(doc.state, DocState::Finished);
    assert_eq!(doc.finished_at, Some(noon(2026, 11, 13)));

    db.undo().unwrap();

    let doc = document(&db, "doc-a");
    assert_eq!(doc.state, DocState::Draft);
    assert_eq!(doc.finished_at, None);
}

#[test]
fn finishing_names_every_missing_fact_in_order_and_changes_nothing() {
    let (_dir, db) = setup_with_vendor();
    let with_vendor = draft(&db, "doc-a", |d| d.vendor_id = Some("vendor-1".into()));
    let empty = draft(&db, "doc-b", |_| {});

    assert_eq!(
        refused(lifecycle::finish(&db, "doc-a", 1, Source::Ui)),
        "Set the payment date, amount paid, item first"
    );
    assert_eq!(
        refused(lifecycle::finish(&db, "doc-b", 1, Source::Ui)),
        "Set the payment date, amount paid, vendor, item first"
    );
    assert_eq!(document(&db, "doc-a"), with_vendor);
    assert_eq!(document(&db, "doc-b"), empty);
}

#[test]
fn a_linked_line_counts_as_the_amount_paid_when_finishing() {
    let (_dir, db) = setup_with_vendor();
    two_october_lines(&db);
    let doc = draft(&db, "doc-a", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.pay_date = Some("2026-10-14".into());
        d.item_key = Some("books".into());
    });
    assert_eq!(lifecycle::missing(&db, &doc).unwrap(), vec!["amount paid"]);
    link(&db, "doc-a", "l-1");

    assert_eq!(lifecycle::missing(&db, &doc).unwrap(), Vec::<&str>::new());
    lifecycle::finish(&db, "doc-a", 1, Source::Ui).unwrap();
    assert_eq!(document(&db, "doc-a").state, DocState::Finished);
}

#[test]
fn only_a_draft_can_be_finished_and_only_an_existing_document() {
    let (_dir, db) = setup_with_vendor();
    for (id, state) in [("doc-f", DocState::Finished), ("doc-x", DocState::Excluded)] {
        draft(&db, id, |d| d.state = state);

        assert_eq!(
            refused(lifecycle::finish(&db, id, 1, Source::Ui)),
            "Only a draft can be finished",
            "{id}"
        );
    }
    assert!(matches!(
        lifecycle::finish(&db, "nope", 1, Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn unlocking_a_finished_document_makes_it_a_draft_and_undo_finishes_it_again() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");
    lifecycle::finish(&db, "doc-a", noon(2026, 11, 13), Source::Ui).unwrap();

    let written = lifecycle::unlock(&db, "doc-a", Source::Ui).unwrap();

    assert_eq!(written.label, "Unlock Pixelforge Ltd. invoice");
    let doc = document(&db, "doc-a");
    assert_eq!(doc.state, DocState::Draft);
    assert_eq!(doc.finished_at, None);

    db.undo().unwrap();

    let doc = document(&db, "doc-a");
    assert_eq!(doc.state, DocState::Finished);
    assert_eq!(doc.finished_at, Some(noon(2026, 11, 13)));
}

#[test]
fn only_a_finished_document_can_be_unlocked() {
    let (_dir, db) = setup_with_vendor();
    for (id, state) in [("doc-d", DocState::Draft), ("doc-x", DocState::Excluded)] {
        draft(&db, id, |d| d.state = state);

        assert_eq!(
            refused(lifecycle::unlock(&db, id, Source::Ui)),
            "Only a finished document can be unlocked",
            "{id}"
        );
    }
    assert!(matches!(
        lifecycle::unlock(&db, "nope", Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

fn close_2026(db: &Db) {
    let open = db
        .read(|c| taxyears::row(c, 2026))
        .unwrap()
        .expect("the setup starts 2026");
    let closed = TaxYearRow {
        state: taxyears::YearState::Closed,
        closed_at: Some(2),
        ..open.clone()
    };
    db.write(
        Op::new(
            "Close",
            vec![Change::update(taxyears::TABLE, &open, &closed).unwrap()],
        ),
        Source::Seed,
    )
    .unwrap();
}

const CLOSED_PAY_DATE: &str = "document.payDate";

#[test]
fn unlocking_a_finished_document_paid_in_a_closed_year_is_refused_until_it_is_reopened() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");
    lifecycle::finish(&db, "doc-a", noon(2026, 11, 13), Source::Ui).unwrap();
    close_2026(&db);

    assert_eq!(
        lifecycle::unlock(&db, "doc-a", Source::Ui).unwrap_err(),
        AppError::InvalidField {
            field: CLOSED_PAY_DATE.into(),
            message: "Tax year 2026 is closed; reopen it to change its documents".into(),
        }
    );
    assert_eq!(document(&db, "doc-a").state, DocState::Finished);
}

#[test]
fn a_draft_dated_in_a_closed_year_cannot_be_changed() {
    let (_dir, db) = setup();
    let doc = draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));
    close_2026(&db);

    let (field, _) = rejected(apply(&db, "doc-a", DocumentEdit::Note { text: "x".into() }));

    assert_eq!(field, CLOSED_PAY_DATE);
    assert_eq!(document(&db, "doc-a"), doc);
}

#[test]
fn a_draft_cannot_be_dated_into_a_closed_year() {
    let (_dir, db) = setup();
    let doc = draft(&db, "doc-a", |_| {});
    close_2026(&db);

    let (field, _) = rejected(apply(&db, "doc-a", pay_date_edit("2026-10-14")));

    assert_eq!(field, CLOSED_PAY_DATE);
    assert_eq!(document(&db, "doc-a"), doc);
}

#[test]
fn a_draft_cannot_be_moved_out_of_a_closed_year() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));
    close_2026(&db);

    let (field, _) = rejected(apply(&db, "doc-a", pay_date_edit("2027-01-05")));

    assert_eq!(field, CLOSED_PAY_DATE);
    assert_eq!(
        document(&db, "doc-a").pay_date.as_deref(),
        Some("2026-10-14")
    );
}

#[test]
fn drafts_of_other_years_stay_editable_while_one_year_is_closed() {
    let (_dir, db) = setup();
    draft(&db, "doc-a", |d| d.pay_date = Some("2027-01-05".into()));
    close_2026(&db);

    let written = apply(&db, "doc-a", DocumentEdit::Note { text: "x".into() }).unwrap();

    assert!(written.is_some());
    assert_eq!(document(&db, "doc-a").note, "x");
}

#[test]
fn adding_a_document_on_a_line_of_a_closed_year_is_refused_before_anything_is_written() {
    let (dir, db) = setup();
    add_lines(&db, &[dollar_debit("l-fx", "2026-11-10", -11_871)]);
    let pending = pixelforge_pending(&db);
    close_2026(&db);

    let result = commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", Some("phone"), Some("l-fx")),
    );

    assert_eq!(failed_field(result), CLOSED_PAY_DATE);
    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(count(&db, store::VENDORS), 0);
    assert_eq!(count(&db, store::LINKS), 0);
}

#[test]
fn adding_a_document_without_a_line_still_works_while_a_year_is_closed() {
    let (dir, db) = setup();
    let pending = pixelforge_pending(&db);
    close_2026(&db);

    commit(
        &db,
        dir.path(),
        pending,
        &input("Pixelforge Ltd.", None, None),
    )
    .unwrap();

    assert_eq!(count(&db, store::DOCUMENTS), 1);
}

#[test]
fn excluding_stores_the_trimmed_reason_and_time_and_undo_brings_the_document_back() {
    for (state, finished_at) in [
        (DocState::Draft, None),
        (DocState::Finished, Some(noon(2026, 11, 1))),
    ] {
        let (_dir, db) = setup_with_vendor();
        let before = draft(&db, "doc-b", |d| {
            d.vendor_id = Some("vendor-1".into());
            d.state = state;
            d.finished_at = finished_at;
        });

        let written = lifecycle::exclude(
            &db,
            "doc-b",
            "  Scanned twice ",
            noon(2026, 11, 2),
            Source::Ui,
        )
        .unwrap();

        assert_eq!(written.label, "Exclude Pixelforge Ltd. invoice");
        let doc = document(&db, "doc-b");
        assert_eq!(doc.state, DocState::Excluded);
        assert_eq!(doc.excluded_at, Some(noon(2026, 11, 2)));
        assert_eq!(doc.exclude_reason.as_deref(), Some("Scanned twice"));
        assert_eq!(doc.finished_at, None, "{state:?}");

        db.undo().unwrap();

        assert_eq!(document(&db, "doc-b"), before, "{state:?}");
    }
}

#[test]
fn excluding_needs_a_reason_and_an_included_document() {
    let (_dir, db) = setup_with_vendor();
    let before = complete_draft(&db, "doc-a");
    excluded(&db, "doc-x", 1);

    for reason in ["", "   "] {
        match lifecycle::exclude(&db, "doc-a", reason, 1, Source::Ui) {
            Err(AppError::InvalidField { field, .. }) => {
                assert_eq!(field, "document.reason", "{reason:?}");
            }
            other => panic!("expected a refusal for {reason:?}, got {other:?}"),
        }
    }
    assert_eq!(document(&db, "doc-a"), before);
    assert_eq!(
        refused(lifecycle::exclude(&db, "doc-x", "Again", 1, Source::Ui)),
        "This document is excluded already"
    );
    assert!(matches!(
        lifecycle::exclude(&db, "nope", "Reason", 1, Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn restoring_an_excluded_document_makes_it_a_draft_without_its_exclusion_and_undo_excludes_it_again()
 {
    let (_dir, db) = setup_with_vendor();
    let before = excluded(&db, "doc-a", noon(2026, 11, 2));

    let written = lifecycle::restore(&db, "doc-a", Source::Ui).unwrap();

    assert_eq!(written.label, "Restore doc-a");
    let doc = document(&db, "doc-a");
    assert_eq!(doc.state, DocState::Draft);
    assert_eq!(doc.excluded_at, None);
    assert_eq!(doc.exclude_reason, None);

    db.undo().unwrap();

    assert_eq!(document(&db, "doc-a"), before);
}

#[test]
fn only_an_excluded_document_can_be_restored() {
    let (_dir, db) = setup_with_vendor();
    for (id, state) in [("doc-d", DocState::Draft), ("doc-f", DocState::Finished)] {
        draft(&db, id, |d| d.state = state);

        assert_eq!(
            refused(lifecycle::restore(&db, id, Source::Ui)),
            "Only an excluded document can be restored",
            "{id}"
        );
    }
    assert!(matches!(
        lifecycle::restore(&db, "nope", Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn a_document_can_be_deleted_once_it_waited_the_archive_days_after_its_exclusion() {
    let doc = |excluded_at| DocumentRow {
        excluded_at,
        ..blank_document("doc-a")
    };

    assert_eq!(
        lifecycle::deletable_from(&doc(Some(1_000)), 7),
        Some(1_000 + WEEK)
    );
    assert_eq!(
        lifecycle::deletable_from(&doc(Some(1_000)), 1),
        Some(1_000 + WEEK / 7)
    );
    assert_eq!(lifecycle::deletable_from(&doc(None), 7), None);
}

#[test]
fn deleting_removes_the_file_links_and_document_and_keeps_a_log_entry() {
    let (dir, db) = setup_with_vendor();
    add_lines(
        &db,
        &[debit(
            "l-1",
            "2026-10-14",
            -4_690,
            "Lastschrift SEITENWEISE",
        )],
    );
    let excluded_at = noon(2026, 11, 2);
    let doc = excluded_with_file(&db, dir.path(), excluded_at);
    link(&db, "doc-one", "l-1");
    assert!(exists(dir.path(), &doc.file));

    let written = erase(
        &db,
        dir.path(),
        "doc-one",
        "  Scanned twice ",
        true,
        excluded_at + WEEK,
    )
    .unwrap();

    assert_eq!(written.label, "Delete Pixelforge Ltd. invoice");
    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(count(&db, store::LINKS), 0);
    assert_eq!(count(&db, statements::TRANSACTIONS), 1);
    assert!(!exists(dir.path(), &doc.file));
    let log = lifecycle::deletions(&db).unwrap();
    assert_eq!(
        log,
        vec![DeletionRow {
            id: log[0].id.clone(),
            document_id: "doc-one".into(),
            title: "Pixelforge Ltd. invoice".into(),
            original_name: "scan.pdf".into(),
            file: "docs/inbox/scan.pdf".into(),
            file_hash: doc.file_hash.clone(),
            created_at: noon(2026, 3, 2),
            excluded_at: Some(excluded_at),
            deleted_at: excluded_at + WEEK,
            reason: "Scanned twice".into(),
        }]
    );
}

#[test]
fn a_deletion_is_not_undoable() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    excluded_with_file(&db, dir.path(), excluded_at);
    erase(
        &db,
        dir.path(),
        "doc-one",
        "Scanned twice",
        true,
        excluded_at + WEEK,
    )
    .unwrap();

    assert_eq!(db.undo().unwrap(), None);

    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(lifecycle::deletions(&db).unwrap().len(), 1);
}

#[test]
fn a_document_whose_file_is_already_gone_can_still_be_deleted() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    let doc = excluded_with_file(&db, dir.path(), excluded_at);
    std::fs::remove_file(dir.path().join(&doc.file)).unwrap();

    erase(
        &db,
        dir.path(),
        "doc-one",
        "Scanned twice",
        true,
        excluded_at + WEEK,
    )
    .unwrap();

    assert_eq!(count(&db, store::DOCUMENTS), 0);
    assert_eq!(lifecycle::deletions(&db).unwrap().len(), 1);
}

#[test]
fn deleting_before_the_waiting_time_is_over_changes_nothing() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    let doc = excluded_with_file(&db, dir.path(), excluded_at);

    let message = refused(erase(
        &db,
        dir.path(),
        "doc-one",
        "Scanned twice",
        true,
        excluded_at + WEEK - 1,
    ));

    assert_eq!(
        message,
        "An excluded document can be deleted after 7 days in the Archive"
    );
    assert_eq!(document(&db, "doc-one"), doc);
    assert!(exists(dir.path(), &doc.file));
    assert_eq!(count(&db, lifecycle::DELETIONS), 0);
}

#[test]
fn deleting_needs_a_reason_and_the_confirmation_that_it_cannot_be_undone() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    let doc = excluded_with_file(&db, dir.path(), excluded_at);

    for (reason, understood, field) in [
        ("  ", true, "deletion.reason"),
        ("  ", false, "deletion.reason"),
        ("Scanned twice", false, "deletion.understood"),
    ] {
        let result = erase(
            &db,
            dir.path(),
            "doc-one",
            reason,
            understood,
            excluded_at + WEEK,
        );

        assert_eq!(failed_field(result), field, "{reason:?} {understood}");
    }
    assert_eq!(document(&db, "doc-one"), doc);
    assert!(exists(dir.path(), &doc.file));
    assert_eq!(count(&db, lifecycle::DELETIONS), 0);
}

#[test]
fn only_an_excluded_document_can_be_deleted() {
    let (dir, db) = setup_with_vendor();
    for (id, state) in [("doc-d", DocState::Draft), ("doc-f", DocState::Finished)] {
        draft(&db, id, |d| d.state = state);

        assert_eq!(
            refused(erase(&db, dir.path(), id, "Reason", true, i64::MAX)),
            "Only an excluded document can be deleted",
            "{id}"
        );
    }
    assert!(matches!(
        erase(&db, dir.path(), "nope", "Reason", true, i64::MAX),
        Err(AppError::NotFound(_))
    ));
    assert_eq!(count(&db, store::DOCUMENTS), 2);
}

#[test]
fn deletions_are_listed_newest_first() {
    let (dir, db) = setup_with_vendor();
    for (id, name, deleted_at) in [
        ("doc-1", "first.pdf", noon(2026, 11, 10)),
        ("doc-2", "second.pdf", noon(2026, 11, 12)),
        ("doc-3", "third.pdf", noon(2026, 11, 11)),
    ] {
        draft(&db, id, |d| {
            d.state = DocState::Excluded;
            d.excluded_at = Some(1);
            d.original_name = name.into();
        });
        erase(&db, dir.path(), id, "Reason", true, deleted_at).unwrap();
    }

    let titles: Vec<String> = lifecycle::deletions(&db)
        .unwrap()
        .into_iter()
        .map(|d| d.title)
        .collect();

    assert_eq!(titles, ["second", "third", "first"]);
}

#[test]
fn the_archive_lists_the_years_excluded_documents_newest_exclusion_first_with_their_waiting_notes()
{
    let (_dir, db) = setup_with_vendor();
    let now = noon(2026, 11, 10);
    for (id, pay_date, created_at, at) in [
        ("a", Some("2026-03-01"), 1, noon(2026, 11, 2)),
        ("b", Some("2026-04-01"), 1, noon(2026, 11, 9)),
        ("c", None, noon(2026, 5, 1), noon(2026, 11, 5)),
        ("d", Some("2025-12-31"), 1, noon(2026, 11, 6)),
        ("e", None, noon(2024, 6, 1), noon(2026, 11, 7)),
    ] {
        draft(&db, id, |doc| {
            doc.state = DocState::Excluded;
            doc.pay_date = pay_date.map(String::from);
            doc.created_at = created_at;
            doc.excluded_at = Some(at);
            doc.exclude_reason = Some("Duplicate".into());
        });
    }
    for (id, state) in [("f", DocState::Draft), ("g", DocState::Finished)] {
        draft(&db, id, |doc| {
            doc.state = state;
            doc.pay_date = Some("2026-06-01".into());
        });
    }
    let archived = |id: &str, excluded: &str, note: Option<&str>| ArchivedDocument {
        id: id.into(),
        title: id.into(),
        file: format!("docs/inbox/{id}.pdf"),
        reason: "Duplicate".into(),
        excluded: excluded.into(),
        deletable: note.is_none(),
        deletable_note: note.map(String::from),
    };

    assert_eq!(
        lifecycle::archive(&db, 2026, 7, now, ORDER).unwrap(),
        vec![
            archived(
                "b",
                "Excluded 9 Nov 2026",
                Some("Can be deleted from 16 Nov 2026")
            ),
            archived(
                "c",
                "Excluded 5 Nov 2026",
                Some("Can be deleted from 12 Nov 2026")
            ),
            archived("a", "Excluded 2 Nov 2026", None),
        ]
    );
    let ids_of_2024: Vec<String> = lifecycle::archive(&db, 2024, 7, now, ORDER)
        .unwrap()
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids_of_2024, ["e"]);
}

#[test]
fn the_archive_waiting_time_follows_the_archive_days_and_the_date_order() {
    let (_dir, db) = setup_with_vendor();
    draft(&db, "a", |d| {
        d.state = DocState::Excluded;
        d.pay_date = Some("2026-03-01".into());
        d.excluded_at = Some(noon(2026, 11, 2));
        d.exclude_reason = Some("Duplicate".into());
    });
    let now = noon(2026, 11, 3);

    let one_day = lifecycle::archive(&db, 2026, 1, now, ORDER).unwrap();
    let thirty_days = lifecycle::archive(&db, 2026, 30, now, DateOrder::MonthFirst).unwrap();

    assert!(one_day[0].deletable);
    assert_eq!(one_day[0].deletable_note, None);
    assert!(!thirty_days[0].deletable);
    assert_eq!(thirty_days[0].excluded, "Excluded Nov 2, 2026");
    assert_eq!(
        thirty_days[0].deletable_note.as_deref(),
        Some("Can be deleted from Dec 2, 2026")
    );
}

#[test]
fn the_deletion_log_shows_title_file_dates_and_reason_newest_first() {
    let (_dir, db) = setup();
    let row = |id: &str, title: &str, created: i64, deleted: i64, reason: &str| DeletionRow {
        id: id.into(),
        document_id: format!("doc-{id}"),
        title: title.into(),
        original_name: format!("{id}.pdf"),
        file: format!("docs/2026/{id}.pdf"),
        file_hash: file_hash(id.as_bytes()),
        created_at: created,
        excluded_at: None,
        deleted_at: deleted,
        reason: reason.into(),
    };
    insert(
        &db,
        lifecycle::DELETIONS,
        &row(
            "old",
            "scan-001",
            noon(2026, 1, 5),
            noon(2026, 11, 1),
            "Wrong person",
        ),
    );
    insert(
        &db,
        lifecycle::DELETIONS,
        &row(
            "new",
            "Pixelforge Ltd. invoice",
            noon(2026, 3, 2),
            noon(2026, 11, 13),
            "Scanned twice",
        ),
    );

    assert_eq!(
        lifecycle::deletion_log(&db, ORDER).unwrap(),
        vec![
            DeletionView {
                title: "Pixelforge Ltd. invoice".into(),
                file: "docs/2026/new.pdf".into(),
                dates: "Added 2 Mar 2026 · deleted 13 Nov 2026".into(),
                reason: "Scanned twice".into(),
            },
            DeletionView {
                title: "scan-001".into(),
                file: "docs/2026/old.pdf".into(),
                dates: "Added 5 Jan 2026 · deleted 1 Nov 2026".into(),
                reason: "Wrong person".into(),
            },
        ]
    );
    assert_eq!(
        lifecycle::deletion_log(&db, DateOrder::MonthFirst).unwrap()[0].dates,
        "Added Mar 2, 2026 · deleted Nov 13, 2026"
    );
}

#[test]
fn the_archive_and_deletion_log_travel_in_camel_case() {
    assert_eq!(
        serde_json::to_value(ArchivedDocument {
            id: "a".into(),
            title: "Pixelforge Ltd. invoice".into(),
            file: "docs/2026/a.pdf".into(),
            reason: "Duplicate".into(),
            excluded: "Excluded 2 Nov 2026".into(),
            deletable: false,
            deletable_note: Some("Can be deleted from 9 Nov 2026".into()),
        })
        .unwrap(),
        json!({
            "id": "a",
            "title": "Pixelforge Ltd. invoice",
            "file": "docs/2026/a.pdf",
            "reason": "Duplicate",
            "excluded": "Excluded 2 Nov 2026",
            "deletable": false,
            "deletableNote": "Can be deleted from 9 Nov 2026"
        })
    );
    assert_eq!(
        serde_json::to_value(DeletionView {
            title: "scan-001".into(),
            file: "docs/2026/a.pdf".into(),
            dates: "Added 2 Mar 2026 · deleted 13 Nov 2026".into(),
            reason: "Duplicate".into(),
        })
        .unwrap(),
        json!({
            "title": "scan-001",
            "file": "docs/2026/a.pdf",
            "dates": "Added 2 Mar 2026 · deleted 13 Nov 2026",
            "reason": "Duplicate"
        })
    );
}

#[test]
fn the_page_shows_when_a_document_was_finished_or_excluded_and_why() {
    for (state, finished_at, excluded_at, expected) in [
        (DocState::Draft, None, None, (None, None, None)),
        (
            DocState::Finished,
            Some(noon(2026, 4, 3)),
            None,
            (Some("3 Apr 2026"), None, None),
        ),
        (
            DocState::Excluded,
            None,
            Some(noon(2026, 11, 2)),
            (None, Some("2 Nov 2026"), Some("Duplicate")),
        ),
    ] {
        let (_dir, db) = setup();
        draft(&db, "doc-a", |d| {
            d.state = state;
            d.finished_at = finished_at;
            d.excluded_at = excluded_at;
            d.exclude_reason = excluded_at.map(|_| "Duplicate".to_string());
        });

        let page = page(&db, "doc-a");

        assert_eq!(
            (
                page.finished_on.as_deref(),
                page.excluded_on.as_deref(),
                page.exclude_reason.as_deref()
            ),
            expected,
            "{state:?}"
        );
    }
}

#[test]
fn document_steps_are_read_from_the_pages_tagged_json() {
    use super::commands::DocumentStep;

    for (wire, expected) in [
        (json!({ "kind": "finish" }), DocumentStep::Finish),
        (json!({ "kind": "unlock" }), DocumentStep::Unlock),
        (
            json!({ "kind": "exclude", "reason": "Scanned twice" }),
            DocumentStep::Exclude {
                reason: "Scanned twice".into(),
            },
        ),
        (json!({ "kind": "restore" }), DocumentStep::Restore),
    ] {
        assert_eq!(
            serde_json::from_value::<DocumentStep>(wire.clone()).unwrap(),
            expected,
            "{wire}"
        );
    }
}

#[test]
fn a_documents_history_tells_its_life_newest_first() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");
    lifecycle::finish(&db, "doc-a", 1, Source::Ui).unwrap();
    lifecycle::unlock(&db, "doc-a", Source::Ui).unwrap();
    lifecycle::exclude(&db, "doc-a", "Scanned twice", 2, Source::Ui).unwrap();
    lifecycle::restore(&db, "doc-a", Source::Ui).unwrap();

    assert_eq!(
        entries_of(&db, "doc-a"),
        vec![
            (HistoryKind::Restored, "Restored".to_string()),
            (HistoryKind::Excluded, "Excluded: Scanned twice".to_string()),
            (HistoryKind::Unlocked, "Unlocked".to_string()),
            (HistoryKind::Finished, "Finished".to_string()),
            (HistoryKind::Added, "Added".to_string()),
        ]
    );
}

#[test]
fn excluding_a_finished_document_reads_as_excluded_not_unlocked() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");
    lifecycle::finish(&db, "doc-a", 1, Source::Ui).unwrap();
    lifecycle::exclude(&db, "doc-a", "Duplicate", 2, Source::Ui).unwrap();

    let entries = entries_of(&db, "doc-a");

    assert_eq!(
        entries[0],
        (HistoryKind::Excluded, "Excluded: Duplicate".to_string())
    );
}

#[test]
fn each_changed_field_reads_as_label_old_to_new_with_names_dates_and_amounts_resolved() {
    let (_dir, db) = setup_with_vendor();
    draft(&db, "doc-a", |d| {
        d.vendor_id = Some("vendor-1".into());
        d.note = "Seminar".into();
    });
    let steps = vec![
        (
            pay_date_edit("2026-10-14"),
            vec!["Payment date: — → 14 Oct 2026"],
        ),
        (
            DocumentEdit::Paid {
                value: Some("46.90".into()),
            },
            vec!["Paid: — → 46.90"],
        ),
        (
            DocumentEdit::Vendor {
                name: "Nordbook".into(),
            },
            vec!["Vendor: Pixelforge Ltd. → Nordbook"],
        ),
        (
            DocumentEdit::Item {
                key: Some("books".into()),
            },
            vec!["Item: — → books"],
        ),
        (
            DocumentEdit::WorkShare { value: 50 },
            vec!["Work share: 100 % → 50 %"],
        ),
        (
            DocumentEdit::Note {
                text: "Workshop".into(),
            },
            vec!["Note: Seminar → Workshop"],
        ),
        (
            DocumentEdit::Invoice {
                amount: Some("129.00".into()),
                currency: "usd".into(),
            },
            vec!["Invoice amount: — → 129.00", "Invoice currency: — → USD"],
        ),
        (
            DocumentEdit::PayDate { value: None },
            vec!["Payment date: 14 Oct 2026 → —"],
        ),
    ];

    for (change, expected) in steps {
        let before = entries_of(&db, "doc-a").len();

        apply(&db, "doc-a", change).unwrap().unwrap();

        let entries = entries_of(&db, "doc-a");
        let added = &entries[..entries.len() - before];
        assert!(
            added.iter().all(|(kind, _)| *kind == HistoryKind::Changed),
            "{expected:?}"
        );
        let mut texts: Vec<&str> = added.iter().map(|(_, text)| text.as_str()).collect();
        texts.sort_unstable();
        assert_eq!(texts, expected, "{expected:?}");
    }
}

#[test]
fn an_invoice_date_change_reads_as_a_date() {
    let (_dir, db) = setup_with_vendor();
    let before = draft(&db, "doc-a", |_| {});
    let after = DocumentRow {
        invoice_date: Some("2026-11-09".into()),
        ..before.clone()
    };
    db.write(
        Op::new(
            "Edit",
            vec![Change::update(store::DOCUMENTS, &before, &after).unwrap()],
        ),
        Source::Ui,
    )
    .unwrap();

    assert_eq!(
        entries_of(&db, "doc-a")[0],
        (
            HistoryKind::Changed,
            "Invoice date: — → 9 Nov 2026".to_string()
        )
    );
}

#[test]
fn money_in_the_history_follows_the_number_format() {
    let (_dir, db) = setup_with_vendor();
    draft(&db, "doc-a", |_| {});
    apply(
        &db,
        "doc-a",
        DocumentEdit::Paid {
            value: Some("46.90".into()),
        },
    )
    .unwrap();

    let entries =
        history::document_history(&db, "doc-a", NumberFormat::CommaDecimal, ORDER).unwrap();

    assert_eq!(entries[0].text, "Paid: — → 46,90");
}

#[test]
fn linking_and_unlinking_a_line_name_the_line_and_its_amount() {
    let (_dir, db) = setup_with_vendor();
    two_october_lines(&db);
    draft(&db, "doc-a", |d| d.pay_date = Some("2026-10-14".into()));

    apply(&db, "doc-a", DocumentEdit::Link { line: "l-1".into() }).unwrap();
    apply(&db, "doc-a", DocumentEdit::Unlink { line: "l-1".into() }).unwrap();

    assert_eq!(
        entries_of(&db, "doc-a"),
        vec![
            (
                HistoryKind::Unlinked,
                "Unlinked Lastschrift SEITENWEISE · \u{2212}€46.90".to_string()
            ),
            (
                HistoryKind::Linked,
                "Linked Lastschrift SEITENWEISE · \u{2212}€46.90".to_string()
            ),
            (HistoryKind::Added, "Added".to_string()),
        ]
    );
}

#[test]
fn moving_a_documents_file_adds_no_history_entry() {
    let (dir, db) = setup_with_vendor();
    pixelforge_draft(&db, dir.path(), "scan.pdf", "one");
    files::relocate(&db, dir.path()).unwrap().unwrap();
    assert_eq!(document(&db, "doc-one").file, MOVED);

    assert_eq!(
        entries_of(&db, "doc-one"),
        vec![(HistoryKind::Added, "Added".to_string())]
    );
}

#[test]
fn a_deleted_documents_history_ends_with_its_deletion() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    excluded_with_file(&db, dir.path(), excluded_at);
    erase(
        &db,
        dir.path(),
        "doc-one",
        "Scanned twice",
        true,
        excluded_at + WEEK,
    )
    .unwrap();

    assert_eq!(
        entries_of(&db, "doc-one"),
        vec![
            (HistoryKind::Deleted, "Deleted".to_string()),
            (HistoryKind::Added, "Added".to_string()),
        ]
    );
}

#[test]
fn an_entry_names_its_document_and_when_it_happened_in_local_time() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");

    let entries = history::document_history(&db, "doc-a", FORMAT, ORDER).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.document, "Pixelforge Ltd. invoice");
    assert_eq!(entry.document_id, "doc-a");
    let (day, time) = entry.when.split_once(", ").unwrap();
    assert!(
        chrono::NaiveTime::parse_from_str(time, "%H:%M").is_ok(),
        "{}",
        entry.when
    );
    let parts: Vec<&str> = day.split(' ').collect();
    assert_eq!(parts.len(), 3, "{}", entry.when);
    assert!(parts[0].parse::<u32>().is_ok(), "{}", entry.when);
    assert_eq!(parts[1].len(), 3, "{}", entry.when);
    assert!(parts[2].parse::<i32>().is_ok(), "{}", entry.when);
}

#[test]
fn a_history_entry_travels_in_camel_case_with_a_snake_case_kind() {
    let (_dir, db) = setup_with_vendor();
    complete_draft(&db, "doc-a");
    let entry = history::document_history(&db, "doc-a", FORMAT, ORDER)
        .unwrap()
        .remove(0);

    let mut wire = serde_json::to_value(&entry).unwrap();

    assert!(wire["when"].is_string());
    wire.as_object_mut().unwrap().remove("when");
    assert_eq!(
        wire,
        json!({
            "kind": "added",
            "text": "Added",
            "document": "Pixelforge Ltd. invoice",
            "documentId": "doc-a"
        })
    );
}

#[test]
fn the_year_log_covers_documents_paid_that_year_and_those_deleted_from_its_folder() {
    let (dir, db) = setup_with_vendor();
    let excluded_at = noon(2026, 11, 2);
    draft(&db, "doc-a", |d| {
        d.pay_date = Some("2026-10-14".into());
        d.note = "Seminar".into();
    });
    apply(
        &db,
        "doc-a",
        DocumentEdit::Note {
            text: "Workshop".into(),
        },
    )
    .unwrap();
    draft(&db, "doc-b", |d| d.pay_date = Some("2025-05-01".into()));
    for (id, file, pay_date) in [
        (
            "doc-c",
            "docs/2026/2026-03-01-seitenweise.pdf",
            "2026-03-01",
        ),
        (
            "doc-d",
            "docs/2025/2025-03-01-seitenweise.pdf",
            "2025-03-01",
        ),
    ] {
        draft(&db, id, |d| {
            d.state = DocState::Excluded;
            d.excluded_at = Some(excluded_at);
            d.file = file.into();
            d.pay_date = Some(pay_date.into());
        });
        erase(
            &db,
            dir.path(),
            id,
            "Scanned twice",
            true,
            excluded_at + WEEK,
        )
        .unwrap();
    }

    let log = history::year_log(&db, 2026, "", FORMAT, ORDER).unwrap();

    let mut seen: Vec<(&str, &str, HistoryKind)> = log
        .iter()
        .map(|e| (e.document.as_str(), e.text.as_str(), e.kind))
        .collect();
    seen.sort_unstable_by_key(|(document, text, _)| (*document, *text));
    assert_eq!(
        seen,
        vec![
            ("doc-a", "Added", HistoryKind::Added),
            ("doc-a", "Note: Seminar → Workshop", HistoryKind::Changed),
            ("doc-c", "Added", HistoryKind::Added),
            ("doc-c", "Deleted: Scanned twice", HistoryKind::Deleted),
        ]
    );
}

#[test]
fn the_year_log_query_matches_the_entry_text_or_the_document_ignoring_case() {
    let (_dir, db) = setup_with_vendor();
    draft(&db, "doc-a", |d| {
        d.pay_date = Some("2026-10-14".into());
        d.note = "Seminar".into();
    });
    apply(
        &db,
        "doc-a",
        DocumentEdit::Note {
            text: "Workshop".into(),
        },
    )
    .unwrap();
    draft(&db, "doc-b", |d| d.pay_date = Some("2026-05-01".into()));

    let by_text = history::year_log(&db, 2026, " WORKSHOP ", FORMAT, ORDER).unwrap();
    let by_document = history::year_log(&db, 2026, "doc-b", FORMAT, ORDER).unwrap();
    let nothing = history::year_log(&db, 2026, "no such entry", FORMAT, ORDER).unwrap();

    let texts = |log: &[history::HistoryEntry]| -> Vec<(String, String)> {
        log.iter()
            .map(|e| (e.document.clone(), e.text.clone()))
            .collect()
    };
    assert_eq!(
        texts(&by_text),
        [("doc-a".to_string(), "Note: Seminar → Workshop".to_string())]
    );
    assert_eq!(
        texts(&by_document),
        [("doc-b".to_string(), "Added".to_string())]
    );
    assert!(nothing.is_empty());
}

#[test]
fn vendor_changes_are_read_from_the_pages_tagged_json() {
    use super::commands::VendorChange;

    for (wire, expected) in [
        (
            json!({ "kind": "rename", "name": "Nordbook" }),
            VendorChange::Rename {
                name: "Nordbook".into(),
            },
        ),
        (
            json!({ "kind": "defaultItem", "key": "books" }),
            VendorChange::DefaultItem {
                key: Some("books".into()),
            },
        ),
        (
            json!({ "kind": "defaultItem", "key": null }),
            VendorChange::DefaultItem { key: None },
        ),
        (
            json!({ "kind": "mergeInto", "into": "v-2" }),
            VendorChange::MergeInto { into: "v-2".into() },
        ),
        (json!({ "kind": "delete" }), VendorChange::Delete),
    ] {
        assert_eq!(
            serde_json::from_value::<VendorChange>(wire.clone()).unwrap(),
            expected,
            "{wire}"
        );
    }
}

/// An open 2026 year (GWG limit 1,000; computers over 3 years from 07-01 on half) and the years in `closed`.
fn depreciation_setup(closed: &[i32]) -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    for year in [2026, 2027, 2028, 2029] {
        let mut rules = minimal_rules(year);
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
        let closing = closed.contains(&year);
        insert(
            &db,
            taxyears::TABLE,
            &TaxYearRow {
                state: if closing {
                    taxyears::YearState::Closed
                } else {
                    taxyears::YearState::Open
                },
                closed_at: closing.then_some(2),
                ..TaxYearRow::new(year, format!("{year} v1"), rules_text(&rules), 1)
            },
        );
    }
    (dir, db)
}

fn computer_draft(db: &Db, date: &str, cents: i64, patch: impl FnOnce(&mut DocumentRow)) {
    draft(db, "pc", |d| {
        d.item_key = Some("computer".into());
        d.pay_date = Some(date.into());
        d.paid_cents = Some(cents);
        patch(d);
    });
}

fn slice_view(year: i32, amount: &str, note: Option<&str>) -> SliceView {
    SliceView {
        year,
        amount: amount.into(),
        note: note.map(String::from),
    }
}

#[test]
fn a_purchase_above_the_gwg_limit_shows_its_schedule_with_the_rule_and_the_reason() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 150_000, |_| {});

    assert_eq!(
        page(&db, "pc").depreciation,
        Some(DepreciationView {
            cost: "€1,500.00".into(),
            years: 3,
            rule: "3 years for computer in ruleset 2026 v1".into(),
            own: false,
            why: "€1,500.00 is above the GWG limit of €1,000.00, so it counts over 3 years.".into(),
            slices: vec![
                slice_view(2026, "€500.00", None),
                slice_view(2027, "€500.00", None),
                slice_view(2028, "€500.00", None),
            ],
        })
    );
}

#[test]
fn a_purchase_in_the_second_half_says_so_and_marks_its_half_year_slices() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-09-01", 150_000, |_| {});

    let view = page(&db, "pc").depreciation.unwrap();

    assert_eq!(
        view.why,
        "€1,500.00 is above the GWG limit of €1,000.00, so it counts over 3 years; bought on or after 07-01, the first and the last year count half."
    );
    assert_eq!(
        view.slices,
        [
            slice_view(2026, "€250.00", Some("half year")),
            slice_view(2027, "€500.00", None),
            slice_view(2028, "€500.00", None),
            slice_view(2029, "€250.00", Some("half year")),
        ]
    );
}

#[test]
fn slices_in_closed_tax_years_are_frozen_even_when_they_are_half_slices() {
    let (_dir, db) = depreciation_setup(&[2027, 2029]);
    computer_draft(&db, "2026-09-01", 150_000, |_| {});

    let view = page(&db, "pc").depreciation.unwrap();

    let notes: Vec<Option<&str>> = view.slices.iter().map(|s| s.note.as_deref()).collect();
    assert_eq!(
        notes,
        [Some("half year"), Some("frozen"), None, Some("frozen")]
    );
}

#[test]
fn an_own_useful_life_is_flagged_only_when_it_differs_from_the_rule() {
    for (own, years, flagged) in [(Some(5), 5, true), (Some(3), 3, false), (None, 3, false)] {
        let (_dir, db) = depreciation_setup(&[]);
        computer_draft(&db, "2026-03-10", 150_000, |d| d.useful_life = own);

        let view = page(&db, "pc").depreciation.unwrap();

        assert_eq!((view.years, view.own), (years, flagged), "{own:?}");
        assert_eq!(
            view.rule, "3 years for computer in ruleset 2026 v1",
            "{own:?}"
        );
        assert_eq!(view.slices.len(), years as usize, "{own:?}");
    }
}

#[test]
fn the_cost_is_the_work_share_of_what_was_paid() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 200_000, |d| d.work_share = 75);

    let view = page(&db, "pc").depreciation.unwrap();

    assert_eq!(view.cost, "€1,500.00");
    assert_eq!(view.slices[0].amount, "€500.00");
}

#[test]
fn a_document_that_counts_in_full_or_lacks_facts_has_no_schedule() {
    type Patch = fn(&mut DocumentRow);
    let cases: [(&str, Patch); 6] = [
        ("at the GWG limit", |d| d.paid_cents = Some(100_000)),
        ("below the GWG limit", |d| d.paid_cents = Some(500)),
        ("an item without a class", |d| {
            d.item_key = Some("books".into())
        }),
        ("no item", |d| d.item_key = None),
        ("no payment date", |d| d.pay_date = None),
        ("no amount", |d| d.paid_cents = None),
    ];
    for (name, patch) in cases {
        let (_dir, db) = depreciation_setup(&[]);
        computer_draft(&db, "2026-03-10", 150_000, patch);

        assert_eq!(page(&db, "pc").depreciation, None, "{name}");
    }
}

#[test]
fn a_depreciated_documents_estimate_counts_only_the_first_slice_with_a_year_one_formula() {
    for (date, years, cents, anv, formula) in [
        (
            "2026-03-10",
            Some(10),
            148_000,
            "€148.00",
            "year 1 of 10 · €1,480.00 in total",
        ),
        (
            "2026-09-01",
            None,
            150_000,
            "€250.00",
            "year 1 of 3 · €1,500.00 in total",
        ),
    ] {
        let (_dir, db) = depreciation_setup(&[]);
        computer_draft(&db, date, cents, |d| d.useful_life = years);

        let estimate = page(&db, "pc").estimate.unwrap();

        assert_eq!(estimate.anv, anv, "{date}");
        assert_eq!(estimate.formula, formula, "{date}");
    }
}

#[test]
fn a_depreciated_estimate_uses_the_work_share_cost_in_its_formula() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 200_000, |d| d.work_share = 75);

    let estimate = page(&db, "pc").estimate.unwrap();

    assert_eq!(estimate.anv, "€500.00");
    assert_eq!(estimate.formula, "year 1 of 3 · €1,500.00 in total");
}

#[test]
fn a_document_counted_in_full_keeps_its_share_formula_and_full_estimate() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 90_000, |d| d.work_share = 50);

    let estimate = page(&db, "pc").estimate.unwrap();

    assert_eq!(page(&db, "pc").depreciation, None);
    assert_eq!(estimate.anv, "€450.00");
    assert_eq!(estimate.formula, "€900.00 × 50 %");
}

#[test]
fn a_depreciation_view_travels_in_camel_case() {
    let (_dir, db) = depreciation_setup(&[2027]);
    computer_draft(&db, "2026-09-01", 150_000, |d| d.useful_life = Some(1));

    assert_eq!(
        serde_json::to_value(page(&db, "pc").depreciation).unwrap(),
        json!({
            "cost": "€1,500.00",
            "years": 1,
            "rule": "3 years for computer in ruleset 2026 v1",
            "own": true,
            "why": "€1,500.00 is above the GWG limit of €1,000.00, so it counts over 1 year; bought on or after 07-01, the first and the last year count half.",
            "slices": [
                { "year": 2026, "amount": "€750.00", "note": "half year" },
                { "year": 2027, "amount": "€750.00", "note": "frozen" }
            ]
        })
    );
}

#[test]
fn the_useful_life_is_stored_between_one_and_fifty_years_and_none_returns_to_the_rule() {
    for (years, stored) in [
        (Some(1), Some(1)),
        (Some(50), Some(50)),
        (Some(7), Some(7)),
        (None, None),
    ] {
        let (_dir, db) = depreciation_setup(&[]);
        computer_draft(&db, "2026-03-10", 150_000, |d| d.useful_life = Some(2));

        apply(&db, "pc", DocumentEdit::UsefulLife { years }).unwrap();

        assert_eq!(document(&db, "pc").useful_life, stored, "{years:?}");
    }
}

#[test]
fn a_useful_life_outside_one_to_fifty_years_is_refused_and_stores_nothing() {
    for years in [0, 51, 1_000] {
        let (_dir, db) = depreciation_setup(&[]);
        computer_draft(&db, "2026-03-10", 150_000, |_| {});

        assert_eq!(
            rejected(apply(
                &db,
                "pc",
                DocumentEdit::UsefulLife { years: Some(years) }
            )),
            (
                "document.usefulLife".into(),
                "Between 1 and 50 years".into()
            ),
            "{years}"
        );
        assert_eq!(document(&db, "pc").useful_life, None, "{years}");
    }
}

#[test]
fn undoing_a_useful_life_edit_restores_the_rules_years() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 150_000, |_| {});
    apply(&db, "pc", DocumentEdit::UsefulLife { years: Some(5) }).unwrap();
    assert_eq!(page(&db, "pc").depreciation.unwrap().years, 5);

    db.undo().unwrap();

    assert_eq!(document(&db, "pc").useful_life, None);
    assert_eq!(page(&db, "pc").depreciation.unwrap().years, 3);
}

#[test]
fn a_finished_document_refuses_a_useful_life_change() {
    let (_dir, db) = depreciation_setup(&[]);
    computer_draft(&db, "2026-03-10", 150_000, |d| d.state = DocState::Finished);

    let result = apply(&db, "pc", DocumentEdit::UsefulLife { years: Some(5) });

    assert!(
        matches!(result, Err(AppError::InvalidInput(_))),
        "{result:?}"
    );
    assert_eq!(document(&db, "pc").useful_life, None);
}
