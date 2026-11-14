use std::collections::HashMap;

use chrono::NaiveDate;
use serde_json::json;

use super::Book;
use super::detail::{ChargeView, PlanView, TimelineMonth, detail};
use super::edit::{self, PlanForm, SubscriptionForm};
use super::matcher::{Matchers, charged_in, plan_on};
use super::schedule::Running;
use super::store::{self, Interval, PlanRow, Reminder, SubscriptionRow};
use super::view::{
    EndedRowView, Format, MatchCount, SubscriptionGroup, SubscriptionRowView, letter, match_count,
    prefill, subscriptions as page,
};
use crate::accounts;
use crate::date::DateOrder;
use crate::db::{Db, Source, Written};
use crate::error::{AppError, AppResult};
use crate::review;
use crate::rules::store::{self as rules_store, Action, DecisionRow};
use crate::rules::{self, RuleForm};
use crate::settings::NumberFormat;
use crate::statements::store::{self as statements, TransactionRow};
use crate::test_support::statements::{Statement, demo_month};
use crate::test_support::subscriptions::{
    NOW, add, day, form, imported, insert_rows, plan_row, subscription_row, today,
};

fn fmt() -> Format {
    Format {
        number: NumberFormat::DotDecimal,
        order: DateOrder::DayFirst,
    }
}

fn month(number: u32) -> Statement {
    demo_month(number, 2026, number, 141_230)
}

fn streamflix() -> SubscriptionForm {
    form("Streamflix", "STREAMFLIX", "12.99")
}

fn line(id: &str, date: &str, cents: i64, text: &str) -> TransactionRow {
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

fn foreign(mut line: TransactionRow, cents: i64, currency: &str) -> TransactionRow {
    line.original_cents = Some(cents);
    line.original_currency = Some(currency.into());
    line
}

fn plan_with(plan: PlanRow, interval: Interval, currency: &str) -> PlanRow {
    PlanRow {
        interval,
        currency: currency.into(),
        ..plan
    }
}

fn matchers(sub: SubscriptionRow, plans: Vec<PlanRow>) -> Matchers {
    Matchers::new(vec![sub], plans)
}

fn decision_of(matchers: &Matchers, t: &TransactionRow) -> Option<DecisionRow> {
    matchers.decide(t, 7)
}

fn all_lines(db: &Db) -> Vec<TransactionRow> {
    db.read(statements::all_transactions).unwrap()
}

fn decisions(db: &Db) -> HashMap<String, DecisionRow> {
    db.read(rules_store::decisions).unwrap()
}

fn rows(db: &Db) -> (Vec<SubscriptionRow>, Vec<PlanRow>) {
    (
        db.read(store::subscriptions).unwrap(),
        db.read(store::plans).unwrap(),
    )
}

/// The value dates of the lines decided as charges of the subscription, oldest first.
fn linked_dates(db: &Db, id: &str) -> Vec<String> {
    let decided = decisions(db);
    let mut dates: Vec<String> = all_lines(db)
        .into_iter()
        .filter(|l| {
            decided
                .get(&l.id)
                .is_some_and(|d| d.subscription_id.as_deref() == Some(id))
        })
        .map(|l| l.value_date)
        .collect();
    dates.sort();
    dates
}

fn line_dated(db: &Db, date: &str, needle: &str) -> TransactionRow {
    all_lines(db)
        .into_iter()
        .find(|l| l.value_date == date && format!("{} {}", l.text, l.extra).contains(needle))
        .unwrap_or_else(|| panic!("no line {needle} on {date}"))
}

fn field_of(result: AppResult<Written>) -> String {
    match result {
        Err(AppError::InvalidField { field, .. }) => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn book(db: &Db) -> Book {
    db.read(Book::load).unwrap()
}

fn plan_form(name: &str, amount: &str, start: &str) -> PlanForm {
    PlanForm {
        name: name.into(),
        amount: amount.into(),
        currency: "EUR".into(),
        interval: Interval::Monthly,
        start: start.into(),
    }
}

fn detail_of(db: &Db, id: &str) -> super::detail::SubscriptionDetail {
    let accounts = db.read(accounts::rows).unwrap();
    let items = rules::items(db).unwrap();
    detail(&book(db), id, &accounts, &items, today(), &fmt()).unwrap()
}

// store

#[test]
fn a_plan_covers_the_days_from_its_start_to_its_end_and_an_open_plan_has_no_end() {
    let closed = plan_row("p", "s", 1_000, "2026-03-01", Some("2026-05-31"));
    let open = plan_row("p", "s", 1_000, "2026-03-01", None);
    for (plan, day, expected) in [
        (&closed, "2026-02-28", false),
        (&closed, "2026-03-01", true),
        (&closed, "2026-05-31", true),
        (&closed, "2026-06-01", false),
        (&open, "2026-02-28", false),
        (&open, "2026-03-01", true),
        (&open, "2099-01-01", true),
    ] {
        assert_eq!(plan.covers(day), expected, "{day}");
    }
}

#[test]
fn intervals_and_reminders_travel_as_snake_case_words_and_read_in_words() {
    for (interval, word, months, label) in [
        (Interval::Monthly, "monthly", 1, "Monthly"),
        (Interval::Quarterly, "quarterly", 3, "Quarterly"),
        (Interval::Yearly, "yearly", 12, "Yearly"),
    ] {
        assert_eq!(serde_json::to_value(interval).unwrap(), word);
        assert_eq!((interval.months(), interval.label()), (months, label));
    }
    for (reminder, word, label) in [
        (Reminder::Off, "off", "Off"),
        (Reminder::OneDay, "one_day", "1 day before"),
        (Reminder::ThreeDays, "three_days", "3 days before"),
        (Reminder::OneWeek, "one_week", "1 week before"),
    ] {
        assert_eq!(serde_json::to_value(reminder).unwrap(), word);
        assert_eq!(reminder.label(), label);
    }
}

#[test]
fn subscriptions_are_read_by_name_ignoring_case_and_plans_by_start() {
    let (_dir, db) = imported(&[]);
    insert_rows(
        &db,
        &subscription_row("s-b", "beta", "BETA"),
        &[
            plan_row("p-2", "s-b", 200, "2026-06-01", None),
            plan_row("p-1", "s-b", 100, "2026-01-01", Some("2026-05-31")),
        ],
    );
    insert_rows(&db, &subscription_row("s-a", "Alpha", "ALPHA"), &[]);

    let (subscriptions, plans) = rows(&db);

    let names: Vec<&str> = subscriptions.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "beta"]);
    let ids: Vec<&str> = plans.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["p-1", "p-2"]);
}

// matcher

#[test]
fn a_line_is_a_charge_when_its_text_contains_the_subscriptions_text_ignoring_case() {
    let sub = SubscriptionRow {
        match_text: " streamflix ".into(),
        ..subscription_row("sub-1", "Streamflix", "")
    };
    let m = matchers(sub, vec![plan_row("p", "sub-1", 1_299, "2026-01-01", None)]);

    let decision = decision_of(
        &m,
        &line("l1", "2026-10-07", -1_299, "E-COMM STREAMFLIX WIEN"),
    );

    assert_eq!(
        decision,
        Some(DecisionRow {
            id: "l1".into(),
            kind: Action::Subscription,
            item_key: None,
            subscription_id: Some("sub-1".into()),
            document_id: None,
            rule_id: Some("sub-1".into()),
            decided_at: 7,
        })
    );
    assert_eq!(
        decision_of(&m, &line("l2", "2026-10-07", -1_299, "E-COMM NETFLIX")),
        None
    );
}

#[test]
fn only_outgoing_lines_are_charges() {
    let m = matchers(
        subscription_row("sub-1", "Streamflix", "STREAMFLIX"),
        vec![plan_row("p", "sub-1", 1_299, "2026-01-01", None)],
    );

    assert_eq!(
        decision_of(&m, &line("l1", "2026-10-07", 1_299, "STREAMFLIX REFUND")),
        None
    );
    assert_eq!(
        decision_of(&m, &line("l2", "2026-10-07", 0, "STREAMFLIX")),
        None
    );
}

#[test]
fn an_empty_text_matches_nothing() {
    let m = matchers(
        subscription_row("sub-1", "Anything", "   "),
        vec![plan_row("p", "sub-1", 1_299, "2026-01-01", None)],
    );

    assert_eq!(
        decision_of(&m, &line("l1", "2026-10-07", -1_299, "ANYTHING")),
        None
    );
}

#[test]
fn a_subscription_charges_up_to_and_including_its_last_day() {
    let sub = SubscriptionRow {
        ended_on: Some("2026-09-30".into()),
        ..subscription_row("sub-1", "Streamflix", "STREAMFLIX")
    };
    let m = matchers(sub, vec![plan_row("p", "sub-1", 1_299, "2026-01-01", None)]);

    for (date, expected) in [("2026-09-30", true), ("2026-10-01", false)] {
        let found = decision_of(&m, &line("l1", date, -1_299, "STREAMFLIX")).is_some();
        assert_eq!(found, expected, "{date}");
    }
}

#[test]
fn the_tolerance_is_how_far_the_amount_may_differ_from_the_plan_in_either_direction() {
    for (tolerance, cents, expected) in [
        (Some(0), 1_299, true),
        (Some(0), 1_300, false),
        (Some(100), 1_399, true),
        (Some(100), 1_400, false),
        (Some(100), 1_199, true),
        (Some(100), 1_198, false),
        (None, 99_999, true),
        (None, 1, true),
    ] {
        let sub = SubscriptionRow {
            tolerance_cents: tolerance,
            ..subscription_row("sub-1", "Streamflix", "STREAMFLIX")
        };
        let m = matchers(sub, vec![plan_row("p", "sub-1", 1_299, "2026-01-01", None)]);

        let found = decision_of(&m, &line("l1", "2026-10-07", -cents, "STREAMFLIX")).is_some();

        assert_eq!(found, expected, "{tolerance:?} {cents}");
    }
}

#[test]
fn a_charge_belongs_to_the_plan_covering_its_day_else_the_last_one_started_before_else_the_first() {
    let first = plan_row("first", "s", 1_000, "2026-01-01", Some("2026-05-31"));
    let second = plan_row("second", "s", 1_200, "2026-07-01", None);
    let plans = [&first, &second];

    for (date, expected) in [
        ("2025-12-31", "first"),
        ("2026-01-01", "first"),
        ("2026-05-31", "first"),
        ("2026-06-15", "first"),
        ("2026-07-01", "second"),
        ("2027-01-01", "second"),
    ] {
        assert_eq!(
            plan_on(&plans, date).map(|p| p.id.as_str()),
            Some(expected),
            "{date}"
        );
    }
    assert_eq!(plan_on(&[], "2026-01-01"), None);
}

#[test]
fn the_amount_is_compared_with_the_plan_in_effect_on_the_line_date() {
    let m = matchers(
        subscription_row("sub-1", "Streamflix", "STREAMFLIX"),
        vec![
            plan_row("old", "sub-1", 1_000, "2026-01-01", Some("2026-05-31")),
            plan_row("new", "sub-1", 1_200, "2026-06-01", None),
        ],
    );

    for (date, cents, expected) in [
        ("2026-03-07", 1_000, true),
        ("2026-03-07", 1_200, false),
        ("2026-07-07", 1_200, true),
        ("2026-07-07", 1_000, false),
    ] {
        let found = decision_of(&m, &line("l1", date, -cents, "STREAMFLIX")).is_some();
        assert_eq!(found, expected, "{date} {cents}");
    }
}

#[test]
fn a_foreign_charge_is_compared_by_its_original_amount_in_the_plans_currency() {
    let usd = plan_with(
        plan_row("p", "sub-1", 2_400, "2026-01-01", None),
        Interval::Monthly,
        "USD",
    );
    let m = matchers(subscription_row("sub-1", "Lumen AI", "LUMEN AI"), vec![usd]);
    let charge = |original: Option<(i64, &str)>| {
        let base = line("l1", "2026-10-10", -2_248, "E-COMM LUMEN AI SAN FRANCISCO");
        match original {
            Some((cents, currency)) => foreign(base, cents, currency),
            None => base,
        }
    };

    for (original, expected) in [
        (Some((2_400, "USD")), true),
        (Some((2_500, "USD")), false),
        (Some((2_400, "GBP")), false),
        (None, false),
    ] {
        assert_eq!(
            decision_of(&m, &charge(original)).is_some(),
            expected,
            "{original:?}"
        );
    }
}

#[test]
fn a_charge_paid_in_dollars_does_not_fit_a_plan_in_euro() {
    let eur = plan_row("p", "sub-1", 1_299, "2026-01-01", None);
    let m = matchers(
        subscription_row("sub-1", "Streamflix", "STREAMFLIX"),
        vec![eur],
    );

    let found = decision_of(
        &m,
        &foreign(line("l1", "2026-10-07", -1_299, "STREAMFLIX"), 1_299, "USD"),
    );

    assert_eq!(found, None);
}

#[test]
fn the_charged_amount_has_no_sign_and_is_in_the_plans_currency_or_nothing() {
    let eur = line("l1", "2026-10-07", -1_299, "X");
    let usd = foreign(line("l2", "2026-10-07", -2_248, "X"), -2_400, "USD");
    for (t, currency, expected) in [
        (&eur, "EUR", Some(1_299)),
        (&eur, "USD", None),
        (&usd, "USD", Some(2_400)),
        (&usd, "EUR", None),
        (&usd, "GBP", None),
    ] {
        assert_eq!(charged_in(t, currency), expected, "{currency}");
    }
}

#[test]
fn a_line_with_the_text_at_another_amount_is_a_possible_plan_change() {
    let sub = subscription_row("sub-1", "Streamflix", "STREAMFLIX");
    let plan = plan_row("p", "sub-1", 1_299, "2026-01-01", None);
    let m = matchers(sub.clone(), vec![plan.clone()]);

    assert_eq!(
        m.plan_changed(&line("l1", "2026-10-07", -1_599, "STREAMFLIX")),
        Some((&sub, &plan))
    );
    for (t, why) in [
        (
            line("l2", "2026-10-07", -1_299, "STREAMFLIX"),
            "amount fits",
        ),
        (line("l3", "2026-10-07", -1_599, "NETFLIX"), "text differs"),
        (line("l4", "2026-10-07", 1_599, "STREAMFLIX"), "incoming"),
    ] {
        assert_eq!(m.plan_changed(&t), None, "{why}");
    }
}

#[test]
fn a_line_is_linked_to_the_first_subscription_it_matches() {
    let a = subscription_row("sub-a", "A", "STREAM");
    let b = subscription_row("sub-b", "B", "STREAMFLIX");
    let plans = vec![
        plan_row("pa", "sub-a", 1_299, "2026-01-01", None),
        plan_row("pb", "sub-b", 1_299, "2026-01-01", None),
    ];
    let m = Matchers::new(vec![a, b], plans);

    let decision = m
        .decide(&line("l1", "2026-10-07", -1_299, "STREAMFLIX"), 0)
        .unwrap();

    assert_eq!(decision.subscription_id.as_deref(), Some("sub-a"));
}

// schedule

fn next_charge(
    sub: &SubscriptionRow,
    plans: &[PlanRow],
    charges: &[TransactionRow],
) -> Option<NaiveDate> {
    Running {
        sub,
        plans: plans.iter().collect(),
        charges: charges.iter().collect(),
    }
    .next_charge(today())
}

#[test]
fn the_next_charge_is_the_last_charge_plus_the_interval() {
    let sub = subscription_row("s", "S", "S");
    let monthly = plan_row("p", "s", 1_000, "2026-01-01", None);
    let quarterly = plan_with(monthly.clone(), Interval::Quarterly, "EUR");
    let yearly = plan_with(monthly.clone(), Interval::Yearly, "EUR");
    let charges = [
        line("l1", "2026-09-07", -1_000, "S"),
        line("l2", "2026-10-07", -1_000, "S"),
    ];

    for (plan, expected) in [
        (monthly, day(2026, 11, 7)),
        (quarterly, day(2027, 1, 7)),
        (yearly, day(2027, 10, 7)),
    ] {
        assert_eq!(
            next_charge(&sub, std::slice::from_ref(&plan), &charges),
            Some(expected),
            "{:?}",
            plan.interval
        );
    }
}

#[test]
fn a_charge_at_the_end_of_a_long_month_is_followed_by_the_end_of_the_short_one() {
    let sub = subscription_row("s", "S", "S");
    let plan = plan_row("p", "s", 1_000, "2026-01-01", None);

    let next = next_charge(&sub, &[plan], &[line("l1", "2026-01-31", -1_000, "S")]);

    assert_eq!(next, Some(day(2026, 2, 28)));
}

#[test]
fn without_a_charge_the_next_one_is_the_plans_start() {
    let sub = subscription_row("s", "S", "S");

    let next = next_charge(&sub, &[plan_row("p", "s", 1_000, "2026-12-01", None)], &[]);

    assert_eq!(next, Some(day(2026, 12, 1)));
}

#[test]
fn a_plan_that_starts_after_the_expected_day_charges_first_on_its_start() {
    let sub = subscription_row("s", "S", "S");
    let plan = plan_row("p", "s", 1_000, "2026-12-01", None);

    let next = next_charge(&sub, &[plan], &[line("l1", "2026-10-07", -1_000, "S")]);

    assert_eq!(next, Some(day(2026, 12, 1)));
}

#[test]
fn an_ended_subscription_or_one_without_a_plan_has_no_next_charge() {
    let ended = SubscriptionRow {
        ended_on: Some("2026-09-30".into()),
        ..subscription_row("s", "S", "S")
    };
    let plan = plan_row("p", "s", 1_000, "2026-01-01", None);

    assert_eq!(next_charge(&ended, std::slice::from_ref(&plan), &[]), None);
    assert_eq!(
        next_charge(&subscription_row("s", "S", "S"), &[], &[]),
        None
    );
}

#[test]
fn the_current_plan_is_the_one_in_effect_today_and_before_the_first_it_is_the_first() {
    let sub = subscription_row("s", "S", "S");
    let first = plan_row("first", "s", 1_000, "2026-01-01", Some("2026-05-31"));
    let second = plan_row("second", "s", 1_200, "2026-06-01", None);
    let running = Running {
        sub: &sub,
        plans: vec![&first, &second],
        charges: vec![],
    };

    for (on, expected) in [
        (day(2025, 12, 31), "first"),
        (day(2026, 3, 1), "first"),
        (day(2026, 11, 13), "second"),
    ] {
        assert_eq!(
            running.current(on).map(|p| p.id.as_str()),
            Some(expected),
            "{on}"
        );
    }
}

#[test]
fn the_monthly_cost_divides_the_plans_euro_amount_by_its_interval() {
    let sub = subscription_row("s", "S", "S");
    for (cents, interval, expected) in [
        (1_299, Interval::Monthly, 1_299),
        (2_997, Interval::Quarterly, 999),
        (12_000, Interval::Yearly, 1_000),
    ] {
        let plan = plan_with(
            plan_row("p", "s", cents, "2026-01-01", None),
            interval,
            "EUR",
        );
        let running = Running {
            sub: &sub,
            plans: vec![&plan],
            charges: vec![],
        };
        assert_eq!(
            running.monthly_cents(today()),
            Some(expected),
            "{interval:?}"
        );
        assert_eq!(running.charge_cents(today()), Some(cents), "{interval:?}");
    }
}

#[test]
fn a_plan_in_another_currency_costs_what_its_last_charge_cost_in_euro() {
    let sub = subscription_row("s", "S", "S");
    let usd = plan_with(
        plan_row("p", "s", 2_400, "2026-01-01", None),
        Interval::Monthly,
        "USD",
    );
    let charge = line("l1", "2026-10-10", -2_248, "S");
    let with_charge = Running {
        sub: &sub,
        plans: vec![&usd],
        charges: vec![&charge],
    };
    let without_charge = Running {
        sub: &sub,
        plans: vec![&usd],
        charges: vec![],
    };

    assert_eq!(with_charge.monthly_cents(today()), Some(2_248));
    assert_eq!(with_charge.charge_cents(today()), Some(2_248));
    assert_eq!(without_charge.monthly_cents(today()), None);
    assert_eq!(without_charge.charge_cents(today()), None);
}

// edit

#[test]
fn creating_a_subscription_stores_it_with_its_first_plan_and_names_the_change() {
    let (_dir, db) = imported(&[]);
    let mut new = streamflix();
    new.name = " Streamflix ".into();
    new.tone = 7;
    new.category = " Media ".into();
    new.currency = "usd".into();
    new.match_text = " STREAMFLIX ".into();
    new.tolerance = Some("1,50".into());
    new.active = false;
    new.ended_on = None;

    let written = edit::create(&db, &new, today(), NOW, Source::Ui).unwrap();

    assert_eq!(written.label, "New Subscription: Streamflix");
    let (subscriptions, plans) = rows(&db);
    let id = subscriptions[0].id.clone();
    assert_eq!(
        subscriptions,
        [SubscriptionRow {
            id: id.clone(),
            name: "Streamflix".into(),
            tone: 1,
            category: "Media".into(),
            ended_on: Some("2026-11-13".into()),
            reminder: Reminder::Off,
            item_key: None,
            match_text: "STREAMFLIX".into(),
            tolerance_cents: Some(150),
            created_at: NOW,
        }]
    );
    assert_eq!(
        plans,
        [PlanRow {
            id: plans[0].id.clone(),
            subscription_id: id,
            name: "Standard".into(),
            cents: 1_299,
            currency: "USD".into(),
            interval: Interval::Monthly,
            start: "2026-01-01".into(),
            end: None,
            created_at: NOW,
        }]
    );
}

#[test]
fn a_negative_tone_wraps_into_the_six_tones_and_an_empty_tolerance_means_exact() {
    let (_dir, db) = imported(&[]);
    let mut new = streamflix();
    new.tone = -1;
    new.tolerance = Some("  ".into());

    edit::create(&db, &new, today(), NOW, Source::Ui).unwrap();

    let sub = rows(&db).0.remove(0);
    assert_eq!((sub.tone, sub.tolerance_cents), (5, Some(0)));
}

#[test]
fn a_form_without_a_tolerance_matches_any_amount() {
    let (_dir, db) = imported(&[]);
    let mut new = streamflix();
    new.tolerance = None;

    edit::create(&db, &new, today(), NOW, Source::Ui).unwrap();

    assert_eq!(rows(&db).0[0].tolerance_cents, None);
}

#[test]
fn an_item_of_the_year_is_accepted() {
    let (_dir, db) = imported(&[]);
    let mut new = streamflix();
    new.item_key = Some("books".into());

    edit::create(&db, &new, today(), NOW, Source::Ui).unwrap();

    assert_eq!(rows(&db).0[0].item_key.as_deref(), Some("books"));
}

#[test]
fn an_invalid_form_names_the_field_and_writes_nothing() {
    let (_dir, db) = imported(&[]);
    let base = streamflix();
    let cases = [
        (
            "subscription.name",
            SubscriptionForm {
                name: "  ".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.match",
            SubscriptionForm {
                match_text: " ".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.item",
            SubscriptionForm {
                item_key: Some("no-such-item".into()),
                ..base.clone()
            },
        ),
        (
            "subscription.amount",
            SubscriptionForm {
                amount: "abc".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.amount",
            SubscriptionForm {
                amount: "0".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.currency",
            SubscriptionForm {
                currency: "EURO".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.currency",
            SubscriptionForm {
                currency: "E1R".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.start",
            SubscriptionForm {
                start: "soon".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.endedOn",
            SubscriptionForm {
                active: false,
                ended_on: Some("someday".into()),
                ..base.clone()
            },
        ),
        (
            "subscription.tolerance",
            SubscriptionForm {
                tolerance: Some("lots".into()),
                ..base.clone()
            },
        ),
    ];

    for (field, invalid) in cases {
        let result = edit::create(&db, &invalid, today(), NOW, Source::Ui);
        assert_eq!(field_of(result), field, "{invalid:?}");
    }

    assert_eq!(rows(&db), (vec![], vec![]));
}

#[test]
fn creating_a_subscription_links_the_lines_it_matches_in_the_same_op() {
    let (_dir, db) = imported(&[month(9), month(10)]);

    let id = add(&db, &streamflix());

    assert_eq!(linked_dates(&db, &id), ["2026-09-07", "2026-10-07"]);
    let decision = decisions(&db)[&line_dated(&db, "2026-10-07", "STREAMFLIX").id].clone();
    assert_eq!(
        (
            decision.kind,
            decision.item_key.clone(),
            decision.rule_id.clone(),
            decision.by_hand()
        ),
        (Action::Subscription, None, Some(id), false)
    );
    db.undo().unwrap().unwrap();
    assert_eq!(decisions(&db), HashMap::new());
    assert_eq!(rows(&db), (vec![], vec![]));
}

#[test]
fn a_foreign_plan_links_charges_by_their_original_amount() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let mut lumen = form("Lumen AI", "LUMEN AI", "24.00");
    lumen.currency = "USD".into();

    let id = add(&db, &lumen);

    assert_eq!(linked_dates(&db, &id), ["2026-09-10", "2026-10-10"]);
}

#[test]
fn lines_at_another_amount_stay_unlinked_unless_any_amount_is_allowed() {
    let (_dir, db) = imported(&[month(10)]);
    let exact = add(&db, &form("Exact", "STREAMFLIX", "13.00"));
    let mut any = form("Any", "STREAMFLIX", "13.00");
    any.tolerance = None;

    assert_eq!(linked_dates(&db, &exact), Vec::<String>::new());
    let any_id = add(&db, &any);

    assert_eq!(linked_dates(&db, &any_id), ["2026-10-07"]);
}

#[test]
fn a_subscription_never_links_a_credit() {
    let (_dir, db) = imported(&[month(10)]);
    let mut salary = form("Salary", "NORDWIND", "2850.00");
    salary.tolerance = None;

    let id = add(&db, &salary);

    assert_eq!(linked_dates(&db, &id), Vec::<String>::new());
}

#[test]
fn a_rule_decides_a_line_before_a_subscription_does() {
    let (_dir, db) = imported(&[month(10)]);
    rules::save(
        &db,
        None,
        &RuleForm {
            contains: vec!["STREAMFLIX".into()],
            amount: None,
            tolerance: None,
            account_id: None,
            own_counterparty: false,
            action: Action::Ignore,
            item_key: None,
        },
        NOW,
        Source::Ui,
    )
    .unwrap();

    let id = add(&db, &streamflix());

    assert_eq!(linked_dates(&db, &id), Vec::<String>::new());
    let decision = &decisions(&db)[&line_dated(&db, "2026-10-07", "STREAMFLIX").id];
    assert_eq!(decision.kind, Action::Ignore);
}

#[test]
fn a_hand_decision_is_not_taken_over_by_a_new_subscription() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let october = line_dated(&db, "2026-10-07", "STREAMFLIX");
    review::decide(
        &db,
        &october.id,
        Action::Ignore,
        None,
        None,
        false,
        NOW,
        Source::Ui,
    )
    .unwrap();

    let id = add(&db, &streamflix());

    assert_eq!(linked_dates(&db, &id), ["2026-09-07"]);
    assert_eq!(decisions(&db)[&october.id].kind, Action::Ignore);
}

#[test]
fn editing_changes_the_subscription_and_its_current_plan_in_place_and_names_the_change() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());
    let mut changed = streamflix();
    changed.name = "Streamflix Plus".into();
    changed.amount = "15.99".into();
    changed.interval = Interval::Quarterly;
    changed.reminder = Reminder::OneWeek;
    changed.start = "2030-01-01".into();

    let written = edit::update(&db, &id, &changed, today(), NOW + 1, Source::Ui).unwrap();

    assert_eq!(written.label, "Edit Subscription: Streamflix Plus");
    let (subscriptions, plans) = rows(&db);
    assert_eq!(
        (
            subscriptions[0].name.as_str(),
            subscriptions[0].reminder,
            subscriptions[0].created_at
        ),
        ("Streamflix Plus", Reminder::OneWeek, NOW)
    );
    assert_eq!(plans.len(), 1);
    assert_eq!(
        (plans[0].cents, plans[0].interval, plans[0].start.as_str()),
        (1_599, Interval::Quarterly, "2026-01-01")
    );
    assert_eq!(linked_dates(&db, &id), Vec::<String>::new());
}

#[test]
fn editing_the_text_and_price_moves_the_links_to_the_lines_now_matched() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());
    let mut lumen = form("Streamflix", "LUMEN AI", "24.00");
    lumen.currency = "USD".into();

    edit::update(&db, &id, &lumen, today(), NOW, Source::Ui).unwrap();

    assert_eq!(linked_dates(&db, &id), ["2026-09-10", "2026-10-10"]);
    db.undo().unwrap().unwrap();
    assert_eq!(linked_dates(&db, &id), ["2026-09-07", "2026-10-07"]);
}

#[test]
fn editing_an_unknown_subscription_is_not_found_and_an_invalid_edit_changes_nothing() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());
    let before = rows(&db);

    let missing = edit::update(
        &db,
        "no-such-subscription",
        &streamflix(),
        today(),
        NOW,
        Source::Ui,
    );
    let mut broken = streamflix();
    broken.name = "".into();
    let invalid = edit::update(&db, &id, &broken, today(), NOW, Source::Ui);

    assert!(matches!(missing, Err(AppError::NotFound(_))), "{missing:?}");
    assert_eq!(field_of(invalid), "subscription.name");
    assert_eq!(rows(&db), before);
}

#[test]
fn a_plan_change_ends_the_current_plan_the_day_before_and_relinks_the_charges() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());

    let written = edit::change_plan(
        &db,
        &id,
        &plan_form(" Premium ", "15.99", "2026-10-01"),
        NOW + 1,
        Source::Ui,
    )
    .unwrap();

    assert_eq!(written.label, "Change Plan: Streamflix");
    let plans = rows(&db).1;
    assert_eq!(
        plans
            .iter()
            .map(|p| (p.name.as_str(), p.cents, p.start.as_str(), p.end.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("Standard", 1_299, "2026-01-01", Some("2026-09-30")),
            ("Premium", 1_599, "2026-10-01", None),
        ]
    );
    assert_eq!(linked_dates(&db, &id), ["2026-09-07"]);
}

#[test]
fn a_plan_change_to_the_price_the_lines_show_links_them_again() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &form("Streamflix", "STREAMFLIX", "9.99"));
    assert_eq!(linked_dates(&db, &id), Vec::<String>::new());

    edit::change_plan(
        &db,
        &id,
        &plan_form("Standard", "12.99", "2026-09-01"),
        NOW,
        Source::Ui,
    )
    .unwrap();

    assert_eq!(linked_dates(&db, &id), ["2026-09-07", "2026-10-07"]);
}

#[test]
fn a_plan_change_needs_a_start_after_the_current_plans_start() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());
    let before = rows(&db);

    for start in ["2026-01-01", "2025-12-01", "next week"] {
        let result = edit::change_plan(
            &db,
            &id,
            &plan_form("Premium", "15.99", start),
            NOW,
            Source::Ui,
        );
        assert_eq!(field_of(result), "subscription.start", "{start}");
    }

    assert_eq!(rows(&db), before);
}

#[test]
fn a_plan_change_needs_a_name_an_amount_and_a_currency() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());
    let base = plan_form("Premium", "15.99", "2026-10-01");
    let cases = [
        (
            "subscription.planName",
            PlanForm {
                name: " ".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.amount",
            PlanForm {
                amount: "0".into(),
                ..base.clone()
            },
        ),
        (
            "subscription.currency",
            PlanForm {
                currency: "".into(),
                ..base.clone()
            },
        ),
    ];

    for (field, invalid) in cases {
        let result = edit::change_plan(&db, &id, &invalid, NOW, Source::Ui);
        assert_eq!(field_of(result), field, "{invalid:?}");
    }

    assert_eq!(rows(&db).1.len(), 1);
}

#[test]
fn a_plan_change_of_an_unknown_subscription_is_not_found() {
    let (_dir, db) = imported(&[]);

    let result = edit::change_plan(
        &db,
        "no-such-subscription",
        &plan_form("Premium", "15.99", "2026-10-01"),
        NOW,
        Source::Ui,
    );

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

#[test]
fn ending_a_subscription_unlinks_the_charges_after_its_last_day_and_resuming_links_them_again() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());

    let ended = edit::set_ended(&db, &id, Some("2026-09-30"), NOW, Source::Ui).unwrap();

    assert_eq!(ended.label, "End Subscription: Streamflix");
    assert_eq!(rows(&db).0[0].ended_on.as_deref(), Some("2026-09-30"));
    assert_eq!(linked_dates(&db, &id), ["2026-09-07"]);

    let resumed = edit::set_ended(&db, &id, None, NOW, Source::Ui).unwrap();

    assert_eq!(resumed.label, "Resume Subscription: Streamflix");
    assert_eq!(rows(&db).0[0].ended_on, None);
    assert_eq!(linked_dates(&db, &id), ["2026-09-07", "2026-10-07"]);
}

#[test]
fn ending_needs_a_real_date_and_an_existing_subscription() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());

    let bad = edit::set_ended(&db, &id, Some("yesterday"), NOW, Source::Ui);
    let missing = edit::set_ended(&db, "no-such-subscription", None, NOW, Source::Ui);

    assert_eq!(field_of(bad), "subscription.endedOn");
    assert!(matches!(missing, Err(AppError::NotFound(_))), "{missing:?}");
    assert_eq!(rows(&db).0[0].ended_on, None);
}

#[test]
fn deleting_removes_the_subscription_its_plans_and_its_links_and_names_the_change() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());
    edit::change_plan(
        &db,
        &id,
        &plan_form("Premium", "15.99", "2026-10-01"),
        NOW,
        Source::Ui,
    )
    .unwrap();

    let written = edit::delete(&db, &id, NOW, Source::Ui).unwrap();

    assert_eq!(written.label, "Delete Subscription: Streamflix");
    assert_eq!(rows(&db), (vec![], vec![]));
    assert_eq!(decisions(&db), HashMap::new());
}

#[test]
fn deleting_also_removes_hand_decisions_that_link_it_and_keeps_other_hand_decisions() {
    let (_dir, db) = imported(&[month(10)]);
    let id = add(&db, &form("Hand Club", "NO SUCH TEXT", "1.00"));
    let streaming = line_dated(&db, "2026-10-07", "STREAMFLIX");
    let cafe = line_dated(&db, "2026-10-21", "CAFE");
    review::decide(
        &db,
        &streaming.id,
        Action::Subscription,
        None,
        Some(id.clone()),
        false,
        NOW,
        Source::Ui,
    )
    .unwrap();
    review::decide(
        &db,
        &cafe.id,
        Action::Ignore,
        None,
        None,
        false,
        NOW,
        Source::Ui,
    )
    .unwrap();

    edit::delete(&db, &id, NOW, Source::Ui).unwrap();

    let remaining: Vec<String> = decisions(&db).into_keys().collect();
    assert_eq!(remaining, [cafe.id]);
}

#[test]
fn undoing_a_delete_restores_the_rows_and_the_links() {
    let (_dir, db) = imported(&[month(9), month(10)]);
    let id = add(&db, &streamflix());
    let rows_before = rows(&db);
    let links_before = decisions(&db);

    edit::delete(&db, &id, NOW, Source::Ui).unwrap();
    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.label, "Delete Subscription: Streamflix");
    assert_eq!(rows(&db), rows_before);
    assert_eq!(decisions(&db), links_before);
}

#[test]
fn deleting_an_unknown_subscription_is_not_found() {
    let (_dir, db) = imported(&[]);

    let result = edit::delete(&db, "no-such-subscription", NOW, Source::Ui);

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

// view

#[test]
fn letters_are_the_first_letter_or_digit_in_upper_case() {
    for (name, expected) in [
        ("Streamflix", "S"),
        ("lumen AI", "L"),
        ("  7 Days", "7"),
        ("- élan", "É"),
        ("", ""),
        ("--", ""),
    ] {
        assert_eq!(letter(name), expected, "{name:?}");
    }
}

#[test]
fn the_next_charge_reads_as_today_tomorrow_a_day_or_a_day_with_the_year_beyond_a_month() {
    let today = day(2026, 11, 13);
    for (days, expected) in [
        (0, "Today"),
        (1, "Tomorrow"),
        (10, "23 Nov"),
        (31, "14 Dec"),
        (32, "15 Dec 2026"),
    ] {
        let next = today + chrono::Days::new(days);
        assert_eq!(fmt().next(next, today), expected, "{days} days");
    }
    let month_first = Format {
        number: NumberFormat::DotDecimal,
        order: DateOrder::MonthFirst,
    };
    assert_eq!(month_first.next(day(2026, 11, 23), today), "Nov 23");
}

fn running_row(id: &str, name: &str, next: &str, amount: &str) -> SubscriptionRowView {
    SubscriptionRowView {
        id: id.into(),
        name: name.into(),
        letter: letter(name),
        tone: 1,
        anv: false,
        interval: "Monthly".into(),
        change: None,
        next: next.into(),
        amount: amount.into(),
    }
}

fn sub_with_plan(db: &Db, id: &str, name: &str, cents: i64, start: &str) {
    insert_rows(
        db,
        &subscription_row(id, name, &name.to_uppercase()),
        &[plan_row(&format!("plan-{id}"), id, cents, start, None)],
    );
}

fn view(db: &Db, query: &str) -> super::view::SubscriptionsView {
    page(&book(db), vec![], query, today(), &fmt())
}

#[test]
fn active_subscriptions_are_grouped_by_next_charge_with_overdue_ones_first_and_ended_ones_apart() {
    let (_dir, db) = imported(&[]);
    sub_with_plan(&db, "sub-overdue", "Overdue Club", 500, "2026-11-01");
    sub_with_plan(&db, "sub-gym", "Gym", 2_000, "2026-11-16");
    sub_with_plan(&db, "sub-tomorrow", "Tomorrow Club", 300, "2026-11-14");
    sub_with_plan(&db, "sub-cloud", "Cloud", 1_000, "2026-12-01");
    sub_with_plan(&db, "sub-later", "Later Club", 5_000, "2027-01-15");
    insert_rows(
        &db,
        &SubscriptionRow {
            ended_on: Some("2026-08-31".into()),
            ..subscription_row("sub-old", "Old Club", "OLD CLUB")
        },
        &[plan_row("plan-old", "sub-old", 900, "2026-01-01", None)],
    );

    let view = view(&db, "");

    assert_eq!(view.summary, "5 active · €88.00 per month");
    assert_eq!(
        view.groups,
        [
            SubscriptionGroup {
                label: "Next 7 days".into(),
                total: "€28.00".into(),
                rows: vec![
                    running_row("sub-overdue", "Overdue Club", "1 Nov", "€5.00"),
                    running_row("sub-tomorrow", "Tomorrow Club", "Tomorrow", "€3.00"),
                    running_row("sub-gym", "Gym", "16 Nov", "€20.00"),
                ],
            },
            SubscriptionGroup {
                label: "Next 30 days".into(),
                total: "€10.00".into(),
                rows: vec![running_row("sub-cloud", "Cloud", "1 Dec", "€10.00")],
            },
            SubscriptionGroup {
                label: "Later".into(),
                total: "€50.00".into(),
                rows: vec![running_row(
                    "sub-later",
                    "Later Club",
                    "15 Jan 2027",
                    "€50.00"
                )],
            },
        ]
    );
    assert_eq!(
        view.ended,
        [EndedRowView {
            id: "sub-old".into(),
            name: "Old Club".into(),
            letter: "O".into(),
            tone: 1,
            interval: "Monthly".into(),
            ended: "Ended 31 Aug 2026".into(),
        }]
    );
    assert_eq!(
        (view.empty, view.categories.as_slice()),
        (false, ["Media".to_string()].as_slice())
    );
}

#[test]
fn an_ended_subscription_names_its_last_charge() {
    let (_dir, db) = imported(&[month(8)]);
    let id = add(&db, &streamflix());
    edit::set_ended(&db, &id, Some("2026-08-31"), NOW, Source::Ui).unwrap();

    let view = view(&db, "");

    assert_eq!(
        view.ended[0].ended,
        "Ended 31 Aug 2026 · last charge 7 Aug 2026"
    );
    assert_eq!(view.summary, "0 active · €0.00 per month");
    assert_eq!(view.groups, vec![]);
}

#[test]
fn the_next_charge_follows_the_last_linked_charge() {
    let (_dir, db) = imported(&[month(10), month(11)]);
    add(&db, &streamflix());

    let view = view(&db, "");

    assert_eq!(view.groups[0].label, "Next 30 days");
    assert_eq!(view.groups[0].rows[0].next, "7 Dec");
}

#[test]
fn the_search_keeps_subscriptions_whose_name_or_category_contains_it_but_the_summary_counts_all() {
    let (_dir, db) = imported(&[]);
    sub_with_plan(&db, "sub-gym", "Gym", 2_000, "2026-11-16");
    insert_rows(
        &db,
        &SubscriptionRow {
            category: "Health".into(),
            ..subscription_row("sub-yoga", "Yoga", "YOGA")
        },
        &[plan_row("plan-yoga", "sub-yoga", 3_000, "2026-11-20", None)],
    );
    sub_with_plan(&db, "sub-cloud", "Cloud", 1_000, "2026-12-01");

    for (query, expected) in [
        ("  GYM ", vec!["Gym"]),
        ("health", vec!["Yoga"]),
        ("o", vec!["Cloud", "Yoga"]),
        ("nothing", vec![]),
    ] {
        let shown = view(&db, query);
        let mut names: Vec<&str> = shown
            .groups
            .iter()
            .flat_map(|g| g.rows.iter().map(|r| r.name.as_str()))
            .collect();
        names.sort_unstable();
        assert_eq!(names, expected, "{query:?}");
        assert_eq!(shown.summary, "3 active · €60.00 per month", "{query:?}");
    }
}

#[test]
fn without_any_subscription_the_page_is_empty() {
    let (_dir, db) = imported(&[]);

    let view = view(&db, "");

    assert!(view.empty);
    assert_eq!(view.summary, "0 active · €0.00 per month");
    assert_eq!(
        (view.groups, view.ended, view.categories),
        (vec![], vec![], vec![])
    );
}

#[test]
fn the_page_passes_the_years_items_through() {
    let (_dir, db) = imported(&[]);
    let items = rules::items(&db).unwrap();

    let view = page(&book(&db), items.clone(), "", today(), &fmt());

    assert_eq!(view.items, items);
}

#[test]
fn a_recent_price_change_shows_as_a_chip_with_its_direction_and_month() {
    for (cents, start, currency, expected) in [
        (1_200, "2026-10-01", "EUR", Some("+€2.00 since Oct")),
        (800, "2026-10-01", "EUR", Some("\u{2212}€2.00 since Oct")),
        (1_000, "2026-10-01", "EUR", None),
        (1_200, "2026-07-01", "EUR", None),
        (1_200, "2026-12-01", "EUR", None),
        (1_200, "2026-10-01", "USD", None),
    ] {
        let (_dir, db) = imported(&[]);
        insert_rows(
            &db,
            &subscription_row("sub-1", "Streamflix", "STREAMFLIX"),
            &[
                plan_row("plan-1", "sub-1", 1_000, "2026-01-01", Some("2026-09-30")),
                plan_with(
                    plan_row("plan-2", "sub-1", cents, start, None),
                    Interval::Monthly,
                    currency,
                ),
            ],
        );

        let view = view(&db, "");

        let change = view.groups[0].rows[0].change.as_deref();
        assert_eq!(change, expected, "{cents} {start} {currency}");
    }
}

#[test]
fn a_plan_in_another_currency_shows_its_price_in_the_interval_text() {
    let (_dir, db) = imported(&[month(10)]);
    let mut lumen = form("Lumen AI", "LUMEN AI", "24.00");
    lumen.currency = "USD".into();
    lumen.item_key = Some("books".into());
    add(&db, &lumen);

    let view = view(&db, "");

    let row = &view.groups[0].rows[0];
    assert_eq!(
        (row.interval.as_str(), row.anv, row.amount.as_str()),
        ("Monthly · USD 24.00", true, "€22.48")
    );
}

#[test]
fn the_sheet_counts_the_charges_a_form_would_match() {
    let (_dir, db) = imported(&[month(9), month(10), month(11)]);
    let lines = all_lines(&db);
    let mut lumen = form("Lumen AI", "LUMEN AI", "24.00");
    lumen.currency = "USD".into();
    let mut any = streamflix();
    any.amount = "99.00".into();
    any.tolerance = None;

    for (draft, count, text) in [
        (streamflix(), 3, "Matches 3 charges, the last on 7 Nov 2026"),
        (lumen, 3, "Matches 3 charges, the last on 10 Nov 2026"),
        (any, 3, "Matches 3 charges, the last on 7 Nov 2026"),
        (
            form("Streamflix", "STREAMFLIX", "13.00"),
            0,
            "No charge in the statements matches yet",
        ),
        (
            form("Nothing", "NO SUCH TEXT", "1.00"),
            0,
            "No charge in the statements matches yet",
        ),
    ] {
        assert_eq!(
            match_count(&lines, &draft, &fmt()),
            MatchCount {
                text: text.into(),
                count
            },
            "{}",
            draft.name
        );
    }
}

#[test]
fn a_single_matching_charge_is_named_with_its_day() {
    let (_dir, db) = imported(&[month(10)]);

    let found = match_count(&all_lines(&db), &streamflix(), &fmt());

    assert_eq!(
        found,
        MatchCount {
            text: "Matches 1 charge, on 7 Oct 2026".into(),
            count: 1
        }
    );
}

#[test]
fn a_new_sheet_starts_from_a_bank_line() {
    let (_dir, db) = imported(&[month(10)]);
    let streaming = line_dated(&db, "2026-10-07", "STREAMFLIX");
    let lumen = line_dated(&db, "2026-10-10", "LUMEN AI");

    let from_streaming = db
        .read(|c| prefill(c, &streaming.id, None, &fmt()))
        .unwrap();
    let from_lumen = db.read(|c| prefill(c, &lumen.id, None, &fmt())).unwrap();

    assert_eq!(
        from_streaming,
        SubscriptionForm {
            name: "STREAMFLIX WIEN 1010".into(),
            tone: 2,
            category: String::new(),
            active: true,
            ended_on: None,
            amount: "12.99".into(),
            currency: "EUR".into(),
            interval: Interval::Monthly,
            start: "2026-10-07".into(),
            reminder: Reminder::Off,
            item_key: None,
            match_text: "STREAMFLIX".into(),
            tolerance: Some("0.00".into()),
        }
    );
    assert_eq!(
        (
            from_lumen.amount.as_str(),
            from_lumen.currency.as_str(),
            from_lumen.match_text.as_str()
        ),
        ("24.00", "USD", "LUMEN")
    );
}

#[test]
fn a_new_sheet_types_amounts_the_way_the_number_format_reads() {
    let (_dir, db) = imported(&[month(10)]);
    let streaming = line_dated(&db, "2026-10-07", "STREAMFLIX");
    let comma = Format {
        number: NumberFormat::CommaDecimal,
        order: DateOrder::DayFirst,
    };

    let form = db
        .read(|c| prefill(c, &streaming.id, None, &comma))
        .unwrap();

    assert_eq!(
        (form.amount.as_str(), form.tolerance.as_deref()),
        ("12,99", Some("0,00"))
    );
}

#[test]
fn a_new_sheet_for_an_unknown_line_is_not_found() {
    let (_dir, db) = imported(&[]);

    let result = db.read(|c| prefill(c, "no-such-line", None, &fmt()));

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

#[test]
fn a_new_sheet_matches_on_the_text_selected_in_the_line() {
    let (_dir, db) = imported(&[month(10)]);
    let streaming = line_dated(&db, "2026-10-07", "STREAMFLIX");

    let form = db
        .read(|c| prefill(c, &streaming.id, Some("  wien   1010 "), &fmt()))
        .unwrap();

    assert_eq!(form.match_text, "WIEN 1010");
}

#[test]
fn a_new_sheet_refuses_a_selection_that_is_not_in_the_line() {
    let (_dir, db) = imported(&[month(10)]);
    let streaming = line_dated(&db, "2026-10-07", "STREAMFLIX");

    for term in ["LUMEN", "WI"] {
        let result = db.read(|c| prefill(c, &streaming.id, Some(term), &fmt()));

        assert!(
            matches!(&result, Err(AppError::InvalidField { field, .. }) if field == "review.term"),
            "{term}: {result:?}"
        );
    }
}

// detail

fn months_with(plan: impl Fn(&str) -> (Option<u32>, String)) -> Vec<TimelineMonth> {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .iter()
    .map(|name| {
        let (index, tip) = plan(name);
        TimelineMonth {
            label: name[..1].into(),
            plan: index,
            tip,
        }
    })
    .collect()
}

#[test]
fn the_detail_shows_amount_next_charge_timeline_plans_rule_charges_and_the_edit_form() {
    let (_dir, db) = imported(&[month(9), month(10), month(11)]);
    let id = add(&db, &streamflix());

    let detail = detail_of(&db, &id);

    let sep = line_dated(&db, "2026-09-07", "STREAMFLIX");
    let oct = line_dated(&db, "2026-10-07", "STREAMFLIX");
    let nov = line_dated(&db, "2026-11-07", "STREAMFLIX");
    assert_eq!(
        (
            detail.name.as_str(),
            detail.letter.as_str(),
            detail.tone,
            detail.interval.as_str(),
            detail.amount.as_str(),
            detail.next.as_deref(),
            detail.ended.as_deref(),
            detail.year,
        ),
        (
            "Streamflix",
            "S",
            1,
            "Monthly",
            "€12.99 / month",
            Some("Mon, 7 Dec · in 24 days"),
            None,
            2026
        )
    );
    assert_eq!(
        detail.timeline,
        months_with(|name| (Some(0), format!("{name}: Standard")))
    );
    assert_eq!(
        detail.plans,
        [PlanView {
            name: "Standard".into(),
            price: "€12.99 monthly".into(),
            when: "since 1 Jan 2026".into(),
        }]
    );
    assert_eq!(
        (
            detail.anv.as_str(),
            detail.reminder.as_str(),
            detail.rule.as_str()
        ),
        ("No", "Off", "text contains STREAMFLIX · exact amount")
    );
    let charge = |id: String, date: &str| ChargeView {
        id,
        date: date.into(),
        account: "Private".into(),
        amount: "€12.99".into(),
    };
    assert_eq!(
        detail.charges,
        [
            charge(nov.id, "Sat, 7 Nov"),
            charge(oct.id, "Wed, 7 Oct"),
            charge(sep.id, "Mon, 7 Sep"),
        ]
    );
    assert_eq!(
        detail.current_plan,
        "Standard · €12.99 monthly · since 1 Jan 2026"
    );
    assert_eq!(
        serde_json::to_value(&detail.form).unwrap(),
        json!({
            "name": "Streamflix",
            "tone": 1,
            "category": "Media",
            "active": true,
            "endedOn": null,
            "amount": "12.99",
            "currency": "EUR",
            "interval": "monthly",
            "start": "2026-01-01",
            "reminder": "off",
            "itemKey": null,
            "matchText": "STREAMFLIX",
            "tolerance": "0.00",
        })
    );
}

#[test]
fn the_detail_travels_with_camel_case_names() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());

    let json = serde_json::to_value(detail_of(&db, &id)).unwrap();

    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "amount",
            "anv",
            "charges",
            "currentPlan",
            "ended",
            "form",
            "id",
            "interval",
            "letter",
            "name",
            "next",
            "plans",
            "reminder",
            "rule",
            "timeline",
            "tone",
            "year"
        ]
    );
}

#[test]
fn a_subscription_for_anv_in_dollars_shows_the_item_euro_cost_reminder_and_tolerance() {
    let (_dir, db) = imported(&[month(10)]);
    let mut lumen = form("Lumen AI", "LUMEN AI", "24.00");
    lumen.currency = "USD".into();
    lumen.item_key = Some("books".into());
    lumen.reminder = Reminder::ThreeDays;
    lumen.tolerance = Some("1.00".into());
    let id = add(&db, &lumen);
    let item = rules::items(&db).unwrap().remove(0);

    let detail = detail_of(&db, &id);

    assert_eq!(
        (
            detail.interval.as_str(),
            detail.amount.as_str(),
            detail.next.as_deref(),
            detail.anv.as_str(),
            detail.reminder.as_str(),
            detail.rule.as_str(),
        ),
        (
            "Monthly · USD 24.00",
            "€22.48 / month",
            Some("Tue, 10 Nov · 3 days ago"),
            format!("Yes · {}", item.label).as_str(),
            "3 days before",
            "text contains LUMEN AI · ± USD 1.00",
        )
    );
    assert_eq!(detail.plans[0].price, "USD 24.00 monthly");
}

#[test]
fn the_rule_text_names_the_tolerance() {
    let (_dir, db) = imported(&[]);
    for (tolerance, expected) in [
        (None, "text contains STREAMFLIX · any amount"),
        (Some("0"), "text contains STREAMFLIX · exact amount"),
        (Some("1.50"), "text contains STREAMFLIX · ± €1.50"),
    ] {
        let mut new = streamflix();
        new.name = format!("Streamflix {tolerance:?}");
        new.tolerance = tolerance.map(Into::into);
        let id = add(&db, &new);

        assert_eq!(detail_of(&db, &id).rule, expected, "{tolerance:?}");
    }
}

#[test]
fn the_amount_names_the_interval() {
    let (_dir, db) = imported(&[]);
    for (interval, expected) in [
        (Interval::Monthly, "€12.99 / month"),
        (Interval::Quarterly, "€12.99 / quarter"),
        (Interval::Yearly, "€12.99 / year"),
    ] {
        let mut new = streamflix();
        new.name = format!("Streamflix {interval:?}");
        new.interval = interval;
        let id = add(&db, &new);

        assert_eq!(detail_of(&db, &id).amount, expected, "{interval:?}");
    }
}

#[test]
fn an_ended_subscription_has_no_next_charge_and_no_plan_in_the_months_after_its_end() {
    let (_dir, db) = imported(&[]);
    let id = add(&db, &streamflix());
    edit::set_ended(&db, &id, Some("2026-09-30"), NOW, Source::Ui).unwrap();

    let detail = detail_of(&db, &id);

    assert_eq!(detail.ended.as_deref(), Some("Ended 30 Sep 2026"));
    assert_eq!(detail.next, None);
    assert_eq!(
        detail.timeline[8],
        TimelineMonth {
            label: "S".into(),
            plan: Some(0),
            tip: "September: Standard".into()
        }
    );
    assert_eq!(
        detail.timeline[9],
        TimelineMonth {
            label: "O".into(),
            plan: None,
            tip: "October: no plan".into()
        }
    );
}

#[test]
fn plans_list_their_periods_and_the_timeline_follows_them() {
    let (_dir, db) = imported(&[]);
    let mut late = streamflix();
    late.start = "2026-03-01".into();
    let id = add(&db, &late);
    edit::change_plan(
        &db,
        &id,
        &plan_form("Premium", "15.99", "2026-10-01"),
        NOW,
        Source::Ui,
    )
    .unwrap();

    let detail = detail_of(&db, &id);

    assert_eq!(
        detail.plans,
        [
            PlanView {
                name: "Standard".into(),
                price: "€12.99 monthly".into(),
                when: "1 Mar – 30 Sep 2026".into(),
            },
            PlanView {
                name: "Premium".into(),
                price: "€15.99 monthly".into(),
                when: "since 1 Oct 2026".into(),
            },
        ]
    );
    assert_eq!(
        detail.timeline,
        months_with(|name| match name {
            "January" | "February" => (None, format!("{name}: no plan")),
            "March" | "April" | "May" | "June" | "July" | "August" | "September" => {
                (Some(0), format!("{name}: Standard"))
            }
            _ => (Some(1), format!("{name}: Premium")),
        })
    );
    assert_eq!(
        detail.current_plan,
        "Premium · €15.99 monthly · since 1 Oct 2026"
    );
    assert_eq!(
        (detail.form.amount.as_str(), detail.form.start.as_str()),
        ("15.99", "2026-10-01")
    );
}

#[test]
fn the_detail_of_an_unknown_subscription_is_not_found() {
    let (_dir, db) = imported(&[]);
    let items = rules::items(&db).unwrap();

    let result = detail(
        &book(&db),
        "no-such-subscription",
        &[],
        &items,
        today(),
        &fmt(),
    );

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}
