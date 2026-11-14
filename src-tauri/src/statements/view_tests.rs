use chrono::{Datelike, NaiveDate};

use super::import;
use super::import_tests::{october, open_with_accounts};
use super::view::{AccountCoverage, AccountState, Cover, Coverage, coverage};
use crate::date::DateOrder;
use crate::db::{Db, Source};
use crate::rules::store::Action;
use crate::rules::{RuleForm, save};
use crate::test_support::demo::seed_if_empty;
use crate::test_support::statements::{Statement, StatementLine, demo_month};
use crate::test_support::temp_config_dir;

pub(super) fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn today() -> NaiveDate {
    day(2026, 11, 13)
}

pub(super) fn seeded() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    seed_if_empty(&db, dir.path(), today()).unwrap();
    (dir, db)
}

/// A rule that ignores every line of `account-1`, so its lines arrive decided.
fn ignore_everything_of_private(db: &Db) {
    let form = RuleForm {
        contains: vec![],
        amount: None,
        tolerance: None,
        account_id: Some("account-1".into()),
        own_counterparty: false,
        action: Action::Ignore,
        item_key: None,
    };
    save(db, None, &form, 0, Source::Ui).unwrap();
}

pub(super) fn import_statement(db: &Db, dir: &std::path::Path, statement: &Statement) {
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

/// `ending`, with one more booking on `to`, so the statement's last booking is that day.
fn ending_on_a_booking(number: u32, month: u32, to: NaiveDate, old_balance: i64) -> Statement {
    let mut statement = ending(number, month, to, old_balance);
    statement.lines.push(StatementLine {
        text: "Entgelt".into(),
        extra: Vec::new(),
        value_date: to,
        cents: -250,
    });
    statement
}

fn overview(db: &Db) -> Coverage {
    coverage(db, today(), "", DateOrder::DayFirst).unwrap()
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
            Missing, Part, Missing, Full, Full, Full, Full, Full, Full, Full, Full, Part
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
            "January 2026 · Nr. 001/2026, Nr. 002/2026",
            "October 2026 · Nr. 010/2026, Nr. 011/2026",
            "November 2026 · partly, Nr. 011/2026",
        ]
    );
    assert_eq!(household.months[2].tip, "February 2026 · no statement");
}

#[test]
fn the_current_month_counts_only_up_to_today() {
    let (dir, db) = open_with_accounts();
    ignore_everything_of_private(&db);
    import_statement(
        &db,
        dir.path(),
        &ending_on_a_booking(11, 11, today(), 141_230),
    );

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
fn a_statement_directly_after_another_covers_the_days_between_their_bookings_but_not_across_a_numbering_gap()
 {
    for (october_number, cover, tip) in [
        (
            10,
            Cover::Full,
            "September 2026 · Nr. 009/2026, Nr. 010/2026",
        ),
        (11, Cover::Part, "September 2026 · partly, Nr. 009/2026"),
    ] {
        let (dir, db) = open_with_accounts();
        let september = demo_month(9, 2026, 9, 141_230);
        import_statement(&db, dir.path(), &september);
        let october = demo_month(october_number, 2026, 10, september.new_balance());
        import_statement(&db, dir.path(), &october);

        let overview = overview(&db);
        let september_cover = &account(&overview, "Private").months[9];

        assert_eq!(september_cover.cover, cover, "Nr. {october_number}");
        assert_eq!(september_cover.tip, tip, "Nr. {october_number}");
    }
}

#[test]
fn the_first_statement_of_a_new_year_covers_the_days_since_the_last_of_the_year_before() {
    let (dir, db) = open_with_accounts();
    let december = demo_month(12, 2025, 12, 141_230);
    import_statement(&db, dir.path(), &december);
    import_statement(
        &db,
        dir.path(),
        &demo_month(1, 2026, 1, december.new_balance()),
    );

    let overview = overview(&db);
    let december_cover = &account(&overview, "Private").months[0];

    assert_eq!(december_cover.cover, Cover::Full);
    assert_eq!(
        december_cover.tip,
        "December 2025 · Nr. 012/2025, Nr. 001/2026"
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
        ignore_everything_of_private(&db);
        let end = today() - chrono::Days::new(days_ago);
        import_statement(
            &db,
            dir.path(),
            &ending_on_a_booking(10, end.month(), end, 141_230),
        );

        let overview = overview(&db);
        let private = account(&overview, "Private");

        assert_eq!(private.state, state, "{days_ago} days");
        assert_eq!(private.state_label, label, "{days_ago} days");
    }
}

#[test]
fn the_seeded_demo_accounts_are_current_but_have_lines_to_review() {
    let (_dir, db) = seeded();
    let overview = overview(&db);

    for (name, waiting) in [("Private", 21), ("Household", 9), ("Business", 2)] {
        let account = account(&overview, name);
        assert_eq!(account.state, AccountState::Review, "{name}");
        assert_eq!(account.to_review, waiting, "{name}");
        assert_eq!(
            account.state_label,
            format!("{waiting} to review"),
            "{name}"
        );
    }
}

#[test]
fn an_up_to_date_account_with_undecided_lines_is_in_review() {
    let (dir, db) = open_with_accounts();
    import_statement(&db, dir.path(), &ending(11, 11, today(), 141_230));

    let overview = overview(&db);
    let private = account(&overview, "Private");

    assert_eq!(private.state, AccountState::Review);
    assert_eq!(private.to_review, 5);
    assert_eq!(private.state_label, "5 to review");
}

#[test]
fn being_behind_wins_over_lines_to_review() {
    let (dir, db) = open_with_accounts();
    let end = today() - chrono::Days::new(30);
    import_statement(&db, dir.path(), &ending(10, end.month(), end, 141_230));

    let overview = overview(&db);
    let private = account(&overview, "Private");

    assert!(private.to_review > 0);
    assert_eq!(private.state, AccountState::Behind);
    assert_eq!(private.state_label, "30 days behind");
}

#[test]
fn an_account_without_undecided_lines_has_nothing_to_review() {
    let (dir, db) = open_with_accounts();
    ignore_everything_of_private(&db);
    import_statement(&db, dir.path(), &ending(11, 11, today(), 141_230));

    assert_eq!(account(&overview(&db), "Private").to_review, 0);
}

#[test]
fn account_states_travel_as_snake_case_words() {
    for (state, word) in [
        (AccountState::Ok, "ok"),
        (AccountState::Behind, "behind"),
        (AccountState::Review, "review"),
    ] {
        assert_eq!(serde_json::to_value(state).unwrap(), word, "{word}");
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
fn a_skipped_statement_number_is_a_note_naming_the_uncovered_days() {
    let (_dir, db) = seeded();
    let overview = overview(&db);

    assert_eq!(
        account(&overview, "Household").note.as_deref(),
        Some("Statement Nr. 002/2026 was never imported: no lines between 21 Jan and 1 Mar.")
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

    let early = coverage(&db, day(2026, 2, 3), "", DateOrder::DayFirst).unwrap();

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
fn a_query_keeps_the_accounts_whose_name_contains_it_ignoring_case() {
    let (_dir, db) = seeded();

    for (query, expected) in [
        ("", vec!["Private", "Household", "Business"]),
        ("hOUSE", vec!["Household"]),
        ("  i", vec!["Private", "Business"]),
        ("savings", vec![]),
    ] {
        let filtered = coverage(&db, today(), query, DateOrder::DayFirst).unwrap();

        let names: Vec<&str> = filtered.accounts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, expected, "{query:?}");
        assert_eq!(filtered.months.len(), 12, "{query:?}");
    }
}

#[test]
fn the_last_day_follows_the_date_order() {
    let (_dir, db) = seeded();

    let month_first = coverage(&db, today(), "", DateOrder::MonthFirst).unwrap();

    assert_eq!(
        account(&month_first, "Private").last,
        "Nr. 011 · until Nov 10"
    );
}
