use chrono::{Datelike, NaiveDate};

use super::import;
use super::import_tests::{october, open_with_accounts};
use super::view::{
    AccountCoverage, AccountState, Cover, Coverage, LineView, account_lines, coverage,
};
use crate::date::DateOrder;
use crate::db::{Db, Source};
use crate::error::AppError;
use crate::settings::NumberFormat;
use crate::test_support::demo::seed_if_empty;
use crate::test_support::statements::{Statement, StatementLine, demo_month};
use crate::test_support::temp_config_dir;

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn today() -> NaiveDate {
    day(2026, 11, 13)
}

fn seeded() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    seed_if_empty(&db, dir.path(), today()).unwrap();
    (dir, db)
}

fn import_statement(db: &Db, dir: &std::path::Path, statement: &Statement) {
    let pending = import::prepare(db, dir, statement.pdf()).unwrap();
    import::commit(db, dir, pending, 0, Source::Import).unwrap();
}

/// A month's statement ending on `to`, without the lines booked after it.
fn ending(number: u32, month: u32, to: NaiveDate, old_balance: i64) -> Statement {
    let mut statement = demo_month(number, 2026, month, old_balance);
    statement.to = to;
    statement.lines.retain(|l| l.value_date <= to);
    statement
}

fn overview(db: &Db) -> Coverage {
    coverage(db, today(), DateOrder::DayFirst).unwrap()
}

fn account<'a>(coverage: &'a Coverage, name: &str) -> &'a AccountCoverage {
    coverage
        .accounts
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("no account {name}"))
}

fn covers(account: &AccountCoverage) -> Vec<Cover> {
    account.months.iter().map(|m| m.cover).collect()
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

#[test]
fn the_month_heads_run_twelve_months_up_to_this_one_with_january_marked() {
    let (_dir, db) = seeded();

    let heads = overview(&db).months;

    let labels: Vec<&str> = heads.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Dec", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov"
        ]
    );
    let januaries: Vec<usize> = heads
        .iter()
        .enumerate()
        .filter(|(_, m)| m.january)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(januaries, vec![1]);
}

#[test]
fn every_account_has_a_cover_for_every_month_shown() {
    let (_dir, db) = seeded();

    let overview = overview(&db);

    let names: Vec<&str> = overview.accounts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Private", "Household", "Business"]);
    for account in &overview.accounts {
        assert_eq!(account.months.len(), 12, "{}", account.name);
    }
}

#[test]
fn months_are_full_part_or_missing_by_the_days_the_statements_cover() {
    let (_dir, db) = seeded();
    let overview = overview(&db);
    use Cover::{Full, Missing, Part};

    assert_eq!(
        covers(account(&overview, "Private")),
        [
            Missing, Full, Full, Full, Full, Full, Full, Full, Full, Full, Full, Part
        ]
    );
    assert_eq!(
        covers(account(&overview, "Household")),
        [
            Missing, Full, Missing, Full, Full, Full, Full, Full, Full, Full, Full, Part
        ]
    );
    assert_eq!(
        covers(account(&overview, "Business")),
        [
            Missing, Missing, Missing, Missing, Missing, Missing, Missing, Missing, Missing, Full,
            Full, Part
        ]
    );
}

#[test]
fn month_tips_name_the_statements_that_cover_them() {
    let (_dir, db) = seeded();
    let overview = overview(&db);
    let private = account(&overview, "Private");
    let household = account(&overview, "Household");

    let tips = |a: &AccountCoverage, months: &[usize]| -> Vec<String> {
        months.iter().map(|i| a.months[*i].tip.clone()).collect()
    };

    assert_eq!(
        tips(private, &[0, 1, 10, 11]),
        [
            "December 2025 · no statement",
            "January 2026 · Nr. 001/2026",
            "October 2026 · Nr. 010/2026",
            "November 2026 · partly, Nr. 011/2026",
        ]
    );
    assert_eq!(household.months[2].tip, "February 2026 · no statement");
}

#[test]
fn the_current_month_counts_only_up_to_today() {
    let (dir, db) = open_with_accounts();
    import_statement(&db, dir.path(), &ending(11, 11, today(), 141_230));

    let overview = overview(&db);
    let private = account(&overview, "Private");

    assert_eq!(private.months[11].cover, Cover::Full);
    assert_eq!(private.months[11].tip, "November 2026 · Nr. 011/2026");
    assert_eq!(private.last, "Nr. 011 · until 13 Nov");
    assert_eq!(private.state, AccountState::Ok);
}

#[test]
fn a_statement_that_covers_some_days_of_a_past_month_makes_it_partly_covered() {
    let (dir, db) = open_with_accounts();
    import_statement(&db, dir.path(), &october());

    let overview = overview(&db);
    let private = account(&overview, "Private");

    assert_eq!(private.months[10].cover, Cover::Part);
    assert_eq!(
        private.months[10].tip,
        "October 2026 · partly, Nr. 010/2026"
    );
}

#[test]
fn the_last_statement_is_named_with_its_number_and_end_day() {
    let (_dir, db) = seeded();
    let overview = overview(&db);

    assert_eq!(account(&overview, "Private").last, "Nr. 011 · until 10 Nov");
    assert_eq!(
        account(&overview, "Business").last,
        "Nr. 011 · until 10 Nov"
    );
}

#[test]
fn an_account_is_up_to_date_within_seven_days_of_its_last_statements_end() {
    for (days_ago, state, label) in [
        (0, AccountState::Ok, "Up to date"),
        (7, AccountState::Ok, "Up to date"),
        (8, AccountState::Behind, "8 days behind"),
        (30, AccountState::Behind, "30 days behind"),
    ] {
        let (dir, db) = open_with_accounts();
        let end = today() - chrono::Days::new(days_ago);
        import_statement(&db, dir.path(), &ending(10, end.month(), end, 141_230));

        let overview = overview(&db);
        let private = account(&overview, "Private");

        assert_eq!(private.state, state, "{days_ago} days");
        assert_eq!(private.state_label, label, "{days_ago} days");
    }
}

#[test]
fn the_seeded_demo_accounts_are_up_to_date_three_days_after_their_last_statement() {
    let (_dir, db) = seeded();

    for account in &overview(&db).accounts {
        assert_eq!(account.state, AccountState::Ok, "{}", account.name);
        assert_eq!(account.state_label, "Up to date", "{}", account.name);
    }
}

#[test]
fn an_account_without_statements_is_behind_with_every_month_missing() {
    let (_dir, db) = open_with_accounts();

    let overview = overview(&db);

    assert_eq!(overview.accounts.len(), 3);
    for account in &overview.accounts {
        assert_eq!(account.last, "No statement yet", "{}", account.name);
        assert_eq!(account.state, AccountState::Behind, "{}", account.name);
        assert_eq!(account.state_label, "No statements", "{}", account.name);
        assert_eq!(account.note, None, "{}", account.name);
        assert_eq!(
            covers(account),
            vec![Cover::Missing; 12],
            "{}",
            account.name
        );
        assert_eq!(account.months[11].tip, "November 2026 · no statement");
    }
}

#[test]
fn a_skipped_statement_number_is_a_note_naming_the_missing_month() {
    let (_dir, db) = seeded();
    let overview = overview(&db);

    assert_eq!(
        account(&overview, "Household").note.as_deref(),
        Some("February is missing: statement Nr. 002/2026 was never imported.")
    );
    assert_eq!(account(&overview, "Private").note, None);
    assert_eq!(account(&overview, "Business").note, None);
}

#[test]
fn balances_that_do_not_connect_are_a_note() {
    let (dir, db) = open_with_accounts();
    import_statement(&db, dir.path(), &demo_month(9, 2026, 9, 141_230));
    import_statement(&db, dir.path(), &demo_month(10, 2026, 10, 100_000));

    let overview = overview(&db);

    assert_eq!(
        account(&overview, "Private").note.as_deref(),
        Some("The balances of Nr. 009/2026 and Nr. 010/2026 don't connect.")
    );
}

#[test]
fn the_month_heads_cross_the_year_boundary_for_an_early_date() {
    let (_dir, db) = open_with_accounts();

    let early = coverage(&db, day(2026, 2, 3), DateOrder::DayFirst).unwrap();

    let labels: Vec<&str> = early.months.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb"
        ]
    );
    let januaries: Vec<usize> = early
        .months
        .iter()
        .enumerate()
        .filter(|(_, m)| m.january)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(januaries, vec![10]);
}

#[test]
fn the_last_day_follows_the_date_order() {
    let (_dir, db) = seeded();

    let month_first = coverage(&db, today(), DateOrder::MonthFirst).unwrap();

    assert_eq!(
        account(&month_first, "Private").last,
        "Nr. 011 · until Nov 10"
    );
}

#[test]
fn the_account_page_groups_lines_by_statement_newest_first() {
    let (_dir, db) = seeded();

    let page = account_lines(
        &db,
        "demo-account-1",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap();

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

    let page = account_lines(
        &db,
        "demo-account-2",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap();

    let labels: Vec<&str> = page.groups.iter().map(|g| g.label.as_str()).collect();
    assert_eq!(labels.len(), 10);
    assert!(!labels.contains(&"Nr. 002/2026"));
    assert_eq!(labels[0], "Nr. 011/2026");
    assert_eq!(labels[9], "Nr. 001/2026");
}

#[test]
fn lines_show_their_day_title_foreign_amount_signed_amount_and_flags() {
    let (_dir, db) = seeded();

    let page = account_lines(
        &db,
        "demo-account-1",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap();

    let october = &page.groups[1];
    let lines: Vec<_> = october.lines.iter().map(shown).collect();
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

    let page = account_lines(
        &db,
        "account-1",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap();

    assert_eq!(page.groups.len(), 1);
    assert_eq!(page.groups[0].count, "1 line");
    assert_eq!(
        shown(&page.groups[0].lines[0]),
        ("28 Oct", "Entgelt", None, "−€2.50", false, false)
    );
}

#[test]
fn an_account_without_statements_has_no_groups() {
    let (_dir, db) = open_with_accounts();

    let page = account_lines(
        &db,
        "account-3",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap();

    assert_eq!(page.name, "Business");
    assert!(page.groups.is_empty());
}

#[test]
fn lines_of_an_unknown_account_are_not_found() {
    let (_dir, db) = open_with_accounts();

    let err = account_lines(
        &db,
        "no-such-account",
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
    .unwrap_err();

    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}
