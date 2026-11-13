use super::import_tests::{october, open_with_accounts, open_with_accounts_and_year};
use super::lines::{AccountLines, LineStatus, LineView, account_lines};
use super::view_tests::{day, import_statement, seeded};
use crate::date::DateOrder;
use crate::db::{Db, Source};
use crate::error::AppError;
use crate::review;
use crate::rules::store::Action;
use crate::rules::{RuleForm, ensure_builtins, save};
use crate::settings::NumberFormat;
use crate::test_support::statements::StatementLine;
use crate::test_support::subscriptions::{add as add_subscription, form as subscription_form};

const NOW: i64 = 1_700_000_000;

fn lines_of(db: &Db, account_id: &str) -> AccountLines {
    account_lines(
        db,
        account_id,
        "",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap()
}

fn lines_matching(db: &Db, account_id: &str, query: &str) -> AccountLines {
    account_lines(
        db,
        account_id,
        query,
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap()
}

fn shown(line: &LineView) -> (&str, &str, Option<&str>, &str, bool, bool) {
    (
        line.date.as_str(),
        line.title.as_str(),
        line.original.as_deref(),
        line.amount.as_str(),
        line.credit,
        line.transfer,
    )
}

fn rule(tokens: &[&str], action: Action, item_key: Option<&str>) -> RuleForm {
    RuleForm {
        contains: tokens.iter().map(|t| (*t).to_string()).collect(),
        amount: None,
        tolerance: None,
        account_id: None,
        own_counterparty: false,
        action,
        item_key: item_key.map(Into::into),
    }
}

fn line_titled<'a>(page: &'a AccountLines, title: &str) -> &'a LineView {
    page.groups
        .iter()
        .flat_map(|g| &g.lines)
        .find(|l| l.title == title)
        .unwrap_or_else(|| panic!("no line {title}"))
}

fn status_of(page: &AccountLines, title: &str) -> (LineStatus, Option<String>) {
    let line = line_titled(page, title);
    (line.status, line.note.clone())
}

/// October imported with an own-account transfer, two rules and the built-in rules.
fn decided_october() -> (tempfile::TempDir, Db) {
    let (dir, db) = open_with_accounts_and_year();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    save(
        &db,
        None,
        &rule(&["Nordwind"], Action::Ignore, None),
        NOW,
        Source::Ui,
    )
    .unwrap();
    save(
        &db,
        None,
        &rule(&["Seitenweise"], Action::Anv, Some("books")),
        NOW,
        Source::Ui,
    )
    .unwrap();
    let mut statement = october();
    statement.lines.insert(
        1,
        StatementLine {
            text: "Dauerauftrag".into(),
            extra: vec![
                "HOUSEHOLD".into(),
                "AT94 1999 9000 0000 0002".into(),
                "HAUSHALT".into(),
            ],
            value_date: day(2026, 10, 2),
            cents: -50_000,
        },
    );
    import_statement(&db, dir.path(), &statement);
    (dir, db)
}

fn decide_by_hand(db: &Db, title: &str, action: Action) {
    let id = line_titled(&lines_of(db, "account-1"), title).id.clone();
    review::decide(db, &id, action, None, None, false, NOW, Source::Ui).unwrap();
}

#[test]
fn the_account_page_groups_lines_by_statement_newest_first() {
    let (_dir, db) = seeded();

    let page = lines_of(&db, "demo-account-1");

    assert_eq!(page.id, "demo-account-1");
    assert_eq!(page.name, "Private");
    assert_eq!(page.iban_masked, "AT•• •••• 0001");
    let heads: Vec<(&str, &str, &str)> = page
        .groups
        .iter()
        .map(|g| (g.label.as_str(), g.range.as_str(), g.count.as_str()))
        .collect();
    assert_eq!(heads.len(), 11);
    assert_eq!(heads[0], ("Nr. 011/2026", "1 Nov – 10 Nov", "6 lines"));
    assert_eq!(heads[1], ("Nr. 010/2026", "1 Oct – 31 Oct", "8 lines"));
    assert_eq!(heads[10], ("Nr. 001/2026", "1 Jan – 31 Jan", "8 lines"));
}

#[test]
fn a_household_without_february_has_no_group_for_it() {
    let (_dir, db) = seeded();

    let page = lines_of(&db, "demo-account-2");

    let labels: Vec<&str> = page.groups.iter().map(|g| g.label.as_str()).collect();
    assert_eq!(labels.len(), 10);
    assert!(!labels.contains(&"Nr. 002/2026"));
    assert_eq!(labels[0], "Nr. 011/2026");
    assert_eq!(labels[9], "Nr. 001/2026");
}

#[test]
fn lines_show_their_day_title_foreign_amount_signed_amount_and_flags() {
    let (_dir, db) = seeded();

    let page = lines_of(&db, "demo-account-1");

    let lines: Vec<_> = page.groups[1].lines.iter().map(shown).collect();
    assert_eq!(
        lines,
        [
            (
                "1 Oct",
                "NORDWIND SOFTWARE GMBH",
                None,
                "+€2,850.00",
                true,
                false
            ),
            ("2 Oct", "HOUSEHOLD", None, "−€500.00", false, true),
            (
                "3 Oct",
                "HAUSVERWALTUNG MUSTER",
                None,
                "−€980.00",
                false,
                false
            ),
            (
                "5 Oct",
                "TANKSTELLE RHEIN LUSTENAU",
                None,
                "−€58.30",
                false,
                false
            ),
            ("7 Oct", "STREAMFLIX WIEN", None, "−€12.99", false, false),
            (
                "10 Oct",
                "LUMEN AI SAN FRANCISCO",
                Some("USD 24.00"),
                "−€22.48",
                false,
                false
            ),
            (
                "14 Oct",
                "BUCHHANDLUNG SEITENWEISE",
                None,
                "−€46.90",
                false,
                false
            ),
            (
                "21 Oct",
                "CAFE LINDE DORNBIRN",
                None,
                "−€6.21",
                false,
                false
            ),
        ]
    );
}

#[test]
fn amounts_and_days_follow_the_number_format_and_date_order() {
    let (_dir, db) = seeded();

    let page = account_lines(
        &db,
        "demo-account-1",
        "",
        NumberFormat::CommaDecimal,
        DateOrder::MonthFirst,
    )
    .unwrap();

    let first = &page.groups[1].lines[0];
    assert_eq!(
        (first.date.as_str(), first.amount.as_str()),
        ("Oct 1", "+€2.850,00")
    );
    assert_eq!(page.groups[1].range, "Oct 1 – Oct 31");
    let foreign = &page.groups[1].lines[5];
    assert_eq!(foreign.original.as_deref(), Some("USD 24,00"));
}

#[test]
fn a_line_without_merchant_or_counterparty_shows_its_booking_text_and_a_single_line_is_singular() {
    let (dir, db) = open_with_accounts();
    let mut statement = october();
    statement.lines = vec![StatementLine {
        text: "Entgelt".into(),
        extra: vec![],
        value_date: day(2026, 10, 28),
        cents: -250,
    }];
    import_statement(&db, dir.path(), &statement);

    let page = lines_of(&db, "account-1");

    assert_eq!(page.groups.len(), 1);
    assert_eq!(page.groups[0].count, "1 line");
    assert_eq!(
        shown(&page.groups[0].lines[0]),
        ("28 Oct", "Entgelt", None, "−€2.50", false, false)
    );
}

#[test]
fn a_query_keeps_only_the_lines_containing_it_ignoring_case() {
    let (_dir, db) = decided_october();

    let page = lines_matching(&db, "account-1", "  seitenWEISE ");

    assert_eq!(page.groups.len(), 1);
    assert_eq!(page.groups[0].count, "1 line");
    let titles: Vec<&str> = page.groups[0]
        .lines
        .iter()
        .map(|l| l.title.as_str())
        .collect();
    assert_eq!(titles, ["BUCHHANDLUNG SEITENWEISE"]);
}

#[test]
fn a_query_matches_the_booking_text_of_a_line_shown_under_its_counterparty() {
    let (_dir, db) = decided_october();

    let page = lines_matching(&db, "account-1", "dauerauftrag");

    let titles: Vec<&str> = page
        .groups
        .iter()
        .flat_map(|g| &g.lines)
        .map(|l| l.title.as_str())
        .collect();
    assert_eq!(titles, ["HOUSEHOLD", "HAUSVERWALTUNG MUSTER"]);
}

#[test]
fn statements_without_a_matching_line_are_left_out_while_a_query_is_set() {
    let (_dir, db) = decided_october();

    let page = lines_matching(&db, "account-1", "no such merchant");

    assert!(page.groups.is_empty());
    assert_eq!(lines_of(&db, "account-1").groups.len(), 1);
}

#[test]
fn an_account_without_statements_has_no_groups() {
    let (_dir, db) = open_with_accounts();

    let page = lines_of(&db, "account-3");

    assert_eq!(page.name, "Business");
    assert!(page.groups.is_empty());
}

#[test]
fn lines_of_an_unknown_account_are_not_found() {
    let (_dir, db) = open_with_accounts();

    let err = account_lines(
        &db,
        "no-such-account",
        "",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}

#[test]
fn a_line_without_a_decision_waits_for_review() {
    let (_dir, db) = decided_october();

    let page = lines_of(&db, "account-1");

    assert_eq!(
        status_of(&page, "LUMEN AI SAN FRANCISCO"),
        (LineStatus::Review, Some("Waiting for review".into()))
    );
}

#[test]
fn an_item_decision_shows_the_kz_of_its_item() {
    let (_dir, db) = decided_october();

    let page = lines_of(&db, "account-1");

    assert_eq!(
        status_of(&page, "BUCHHANDLUNG SEITENWEISE"),
        (LineStatus::Anv, Some("KZ 720".into()))
    );
}

#[test]
fn an_ignored_transfer_between_own_accounts_is_a_transfer_without_a_note() {
    let (_dir, db) = decided_october();

    let page = lines_of(&db, "account-1");

    assert_eq!(status_of(&page, "HOUSEHOLD"), (LineStatus::Transfer, None));
}

#[test]
fn an_ignore_by_rule_says_so_and_an_ignore_by_hand_does_not() {
    let (_dir, db) = decided_october();
    decide_by_hand(&db, "STREAMFLIX WIEN", Action::Ignore);

    let page = lines_of(&db, "account-1");

    assert_eq!(
        status_of(&page, "NORDWIND SOFTWARE GMBH"),
        (LineStatus::Ignored, Some("Ignored by rule".into()))
    );
    assert_eq!(
        status_of(&page, "STREAMFLIX WIEN"),
        (LineStatus::Ignored, Some("Ignored".into()))
    );
}

#[test]
fn a_subscription_decision_is_a_subscription_without_a_note() {
    let (_dir, db) = decided_october();
    let subscription_id = add_subscription(
        &db,
        &subscription_form("Streamflix", "NO SUCH TEXT", "12.99"),
    );
    let line_id = line_titled(&lines_of(&db, "account-1"), "STREAMFLIX WIEN")
        .id
        .clone();
    review::decide(
        &db,
        &line_id,
        Action::Subscription,
        None,
        Some(subscription_id),
        false,
        NOW,
        Source::Ui,
    )
    .unwrap();

    let page = lines_of(&db, "account-1");

    assert_eq!(
        status_of(&page, "STREAMFLIX WIEN"),
        (LineStatus::Subscription, None)
    );
}

#[test]
fn line_statuses_travel_as_snake_case_words() {
    for (status, word) in [
        (LineStatus::Review, "review"),
        (LineStatus::Anv, "anv"),
        (LineStatus::Subscription, "subscription"),
        (LineStatus::Ignored, "ignored"),
        (LineStatus::Transfer, "transfer"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), word, "{word}");
    }
}
