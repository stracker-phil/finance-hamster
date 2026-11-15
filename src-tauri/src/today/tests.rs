//! The Today page against a temp database with the demo accounts, a 2026 tax year and statements up to date.

use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::db::Source;
use crate::documents::store::DocState;
use crate::rules::store::{Action, DECISIONS, DecisionRow};
use crate::statements::import_tests::open_with_accounts;
use crate::statements::store::{STATEMENTS, StatementRow, TRANSACTIONS, TransactionRow};
use crate::subscriptions::store::SubscriptionRow;
use crate::test_support::rules::{minimal_rules, minimal_text};
use crate::test_support::subscriptions::{day, insert_rows, plan_row, subscription_row, today};
use crate::year::tests::{add_document, debit, insert, open_year_with, rules, ruleset};

const NOW: i64 = 1_700_000_000;
const HOME_DAYS: &str = "Telearbeitstage (Homeoffice days)";
const PAUSCHALE: &str = "Telearbeitspauschale (from the employer)";
const ACCOUNTS: [&str; 3] = ["account-1", "account-2", "account-3"];
const BEHIND_AFTER_DAYS: u32 = 7;

fn view_in(db: &Db, number: NumberFormat, order: DateOrder) -> TodayView {
    today_view(db, today(), number, order, BEHIND_AFTER_DAYS).unwrap()
}

fn view_behind_after(db: &Db, days: u32) -> TodayView {
    today_view(
        db,
        today(),
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
        days,
    )
    .unwrap()
}

fn view(db: &Db) -> TodayView {
    view_in(db, NumberFormat::DotDecimal, DateOrder::DayFirst)
}

fn keys(view: &TodayView) -> Vec<&str> {
    view.next
        .iter()
        .chain(&view.also)
        .map(|s| s.key.as_str())
        .collect()
}

fn statement_ending(db: &Db, account_id: &str, to: &str) {
    insert(
        db,
        STATEMENTS,
        &StatementRow {
            id: format!("statement-{account_id}"),
            account_id: account_id.into(),
            number: 11,
            year: 2026,
            from_date: "2026-01-01".into(),
            to_date: to.into(),
            old_balance: 0,
            credits: 0,
            debits: 0,
            new_balance: 0,
            layout: "sparkasse".into(),
            file_hash: format!("hash-{account_id}"),
            file: format!("statements/2026/{account_id}.pdf"),
            pages: 1,
            imported_at: 1,
        },
    );
}

/// A second statement of `account_id`, Nr. 9, which leaves Nr. 10 missing before its Nr. 11.
fn earlier_statement(db: &Db, account_id: &str) {
    insert(
        db,
        STATEMENTS,
        &StatementRow {
            id: format!("statement-{account_id}-earlier"),
            account_id: account_id.into(),
            number: 9,
            year: 2026,
            from_date: "2025-12-01".into(),
            to_date: "2025-12-31".into(),
            old_balance: 0,
            credits: 0,
            debits: 0,
            new_balance: 0,
            layout: "sparkasse".into(),
            file_hash: format!("hash-{account_id}-earlier"),
            file: format!("statements/2026/{account_id}-earlier.pdf"),
            pages: 1,
            imported_at: 1,
        },
    );
}

/// A line of `account_id`'s statement, still waiting for a decision.
fn add_line(db: &Db, n: u32, account_id: &str, date: &str, text: &str) {
    insert(
        db,
        TRANSACTIONS,
        &TransactionRow {
            statement_id: format!("statement-{account_id}"),
            account_id: account_id.into(),
            row: n as _,
            ..debit(&format!("line-{n}"), date, -1_000, text)
        },
    );
}

fn decide(db: &Db, n: u32, kind: Action, item: Option<&str>) {
    insert(
        db,
        DECISIONS,
        &DecisionRow {
            id: format!("line-{n}"),
            kind,
            item_key: item.map(String::from),
            subscription_id: None,
            document_id: None,
            rule_id: None,
            decided_at: 1,
        },
    );
}

/// An ANV payment on books, which needs its invoice.
fn missing_invoice(db: &Db, n: u32, date: &str, text: &str) {
    add_line(db, n, "account-1", date, text);
    decide(db, n, Action::Anv, Some("books"));
}

/// Every account's newest statement ends 3 days ago; the year has no questions and nothing to attach.
fn quiet() -> (TempDir, Db) {
    quiet_ending(["2026-11-10"; 3])
}

/// Like `quiet`, with each account's newest statement ending on its own day.
fn quiet_ending(ends: [&str; 3]) -> (TempDir, Db) {
    let (dir, db) = open_year_with(&minimal_rules(2026));
    for (account, end) in ACCOUNTS.into_iter().zip(ends) {
        statement_ending(&db, account, end);
    }
    (dir, db)
}

/// Review, attach, import, questions and draft all waiting: Private is behind, Business has a new line.
fn busy() -> (TempDir, Db) {
    let (dir, db) = open_year_with(&rules());
    statement_ending(&db, "account-1", "2026-10-20");
    statement_ending(&db, "account-2", "2026-11-10");
    statement_ending(&db, "account-3", "2026-11-10");
    add_line(&db, 1, "account-3", "2026-11-09", "Lastschrift NEW");
    missing_invoice(&db, 2, "2026-10-05", "Lastschrift SEITENWEISE");
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
    });
    (dir, db)
}

fn add_sub(db: &Db, n: u32, name: &str, start: &str, patch: impl FnOnce(&mut SubscriptionRow)) {
    let id = format!("sub-{n}");
    let mut sub = subscription_row(&id, name, &name.to_uppercase());
    patch(&mut sub);
    insert_rows(
        db,
        &sub,
        &[plan_row(&format!("plan-{n}"), &id, 1_299, start, None)],
    );
}

fn upcoming_names(view: &TodayView) -> Vec<&str> {
    view.upcoming.iter().map(|c| c.name.as_str()).collect()
}

#[test]
fn nothing_open_leaves_no_next_step_and_no_other_steps() {
    let (_dir, db) = quiet();

    let view = view(&db);

    assert_eq!(view.next, None);
    assert!(view.also.is_empty());
    assert!(view.upcoming.is_empty());
}

#[test]
fn the_steps_come_in_order_review_attach_import_questions_draft() {
    let (_dir, db) = busy();

    let view = view(&db);

    assert_eq!(view.next.as_ref().map(|s| s.key.as_str()), Some("review"));
    assert_eq!(
        view.also.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
        ["attach", "import", "questions", "draft"]
    );
}

#[test]
fn the_headline_moves_on_to_the_next_open_step_once_review_is_done() {
    let (_dir, db) = busy();
    decide(&db, 1, Action::Ignore, None);

    let view = view(&db);

    assert_eq!(keys(&view), ["attach", "import", "questions", "draft"]);
    assert_eq!(view.next.map(|s| s.key), Some("attach".to_string()));
}

#[test]
fn a_single_line_asks_for_one_minute_of_review() {
    let (_dir, db) = quiet();
    add_line(&db, 1, "account-1", "2026-11-09", "Lastschrift A");

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "review".into(),
            title: "1 line to review".into(),
            detail: "about 1 minute".into(),
            line: "From Private. Rules decided the rest.".into(),
            button: "Start Review".into(),
            target: StepTarget::Review,
        }
    );
}

#[test]
fn review_time_is_the_waiting_count_over_six_rounded_up() {
    for (count, title, detail) in [
        (6, "6 lines to review", "about 1 minute"),
        (7, "7 lines to review", "about 2 minutes"),
        (12, "12 lines to review", "about 2 minutes"),
        (13, "13 lines to review", "about 3 minutes"),
    ] {
        let (_dir, db) = quiet();
        for n in 1..=count {
            add_line(&db, n, "account-1", "2026-11-09", "Lastschrift A");
        }

        let step = view(&db).next.unwrap();

        assert_eq!(
            (step.title.as_str(), step.detail.as_str()),
            (title, detail),
            "{count}"
        );
    }
}

#[test]
fn review_names_only_the_accounts_with_waiting_lines_and_counts_undecided_ones() {
    let (_dir, db) = quiet();
    add_line(&db, 1, "account-1", "2026-11-09", "Lastschrift A");
    add_line(&db, 2, "account-2", "2026-11-09", "Lastschrift B");
    add_line(&db, 3, "account-3", "2026-11-09", "Lastschrift C");
    decide(&db, 2, Action::Ignore, None);

    let step = view(&db).next.unwrap();

    assert_eq!(step.title, "2 lines to review");
    assert_eq!(
        step.line,
        "From Private and Business. Rules decided the rest."
    );
}

#[test]
fn attach_counts_the_missing_invoices_and_names_the_first_two_by_title() {
    let (_dir, db) = quiet();
    missing_invoice(&db, 1, "2026-10-05", "Lastschrift A");
    missing_invoice(&db, 2, "2026-10-06", "Lastschrift A");
    missing_invoice(&db, 3, "2026-10-07", "Lastschrift B");
    missing_invoice(&db, 4, "2026-10-08", "Lastschrift C");

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "attach".into(),
            title: "4 invoices to attach".into(),
            detail: "Lastschrift A and Lastschrift B".into(),
            line: "They count once their PDF is attached.".into(),
            button: "Show in Tax Year 2026".into(),
            target: StepTarget::Year {
                year: 2026,
                attention: Some(AttentionKind::MissingInvoice),
            },
        }
    );
}

#[test]
fn one_missing_invoice_reads_in_the_singular() {
    let (_dir, db) = quiet();
    missing_invoice(&db, 1, "2026-10-05", "Lastschrift SEITENWEISE");

    let step = view(&db).next.unwrap();

    assert_eq!(step.title, "1 invoice to attach");
    assert_eq!(step.detail, "Lastschrift SEITENWEISE");
}

#[test]
fn import_names_the_first_account_that_is_behind_and_how_long() {
    let (_dir, db) = open_year_with(&minimal_rules(2026));
    statement_ending(&db, "account-1", "2026-10-20");
    statement_ending(&db, "account-2", "2026-10-01");
    statement_ending(&db, "account-3", "2026-11-10");

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "import".into(),
            title: "Private statement is 24 days behind".into(),
            detail: "last one ends 20 Oct".into(),
            line: "Its last statement ends on 20 Oct.".into(),
            button: "Import Statement…".into(),
            target: StepTarget::Import,
        }
    );
}

#[test]
fn an_account_without_statements_is_behind_too() {
    let (_dir, db) = open_year_with(&minimal_rules(2026));
    statement_ending(&db, "account-1", "2026-11-10");
    statement_ending(&db, "account-2", "2026-11-10");

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "import".into(),
            title: "Business has no statement yet".into(),
            detail: "import its first one".into(),
            line: "Import its first statement to see its lines.".into(),
            button: "Import Statement…".into(),
            target: StepTarget::Import,
        }
    );
}

#[test]
fn an_account_within_a_week_of_today_is_not_behind() {
    let (_dir, db) = quiet_ending(["2026-11-06", "2026-11-10", "2026-11-10"]);

    assert_eq!(view(&db).next, None);
}

#[test]
fn questions_lists_the_unanswered_ones_of_the_newest_open_year() {
    let (_dir, db) = open_year_with(&rules());
    for account in ACCOUNTS {
        statement_ending(&db, account, "2026-11-10");
    }

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "questions".into(),
            title: "2 questions for Tax year 2026".into(),
            detail: format!("{HOME_DAYS} · {PAUSCHALE}"),
            line: "Numbers from your payslip (Lohnzettel). Hamster works out what they change."
                .into(),
            button: "Answer in Tax Year 2026".into(),
            target: StepTarget::Year {
                year: 2026,
                attention: None,
            },
        }
    );
}

#[test]
fn an_answered_question_drops_out_of_the_questions_step() {
    let (_dir, db) = open_year_with(&rules());
    for account in ACCOUNTS {
        statement_ending(&db, account, "2026-11-10");
    }
    crate::year::answers::answer(&db, &ruleset(), 2026, "pauschale", "1200", NOW, Source::Ui)
        .unwrap();

    let step = view(&db).next.unwrap();

    assert_eq!(step.title, "1 question for Tax year 2026");
    assert_eq!(step.detail, HOME_DAYS);
}

#[test]
fn the_questions_step_is_offered_on_any_date_while_a_question_is_unanswered() {
    for (on, end) in [
        (day(2026, 10, 31), "2026-10-30"),
        (day(2026, 11, 15), "2026-11-10"),
        (day(2026, 12, 31), "2026-12-30"),
        (day(2027, 1, 9), "2027-01-05"),
    ] {
        let (_dir, db) = open_year_with(&rules());
        for account in ACCOUNTS {
            statement_ending(&db, account, end);
        }

        let view = today_view(
            &db,
            on,
            NumberFormat::DotDecimal,
            DateOrder::DayFirst,
            BEHIND_AFTER_DAYS,
        )
        .unwrap();

        assert_eq!(keys(&view), ["questions"], "{on}");
    }
}

#[test]
fn the_questions_step_comes_before_a_draft() {
    let (_dir, db) = open_year_with(&rules());
    for account in ACCOUNTS {
        statement_ending(&db, account, "2026-11-10");
    }
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
    });

    assert_eq!(keys(&view(&db)), ["questions", "draft"]);
}

#[test]
fn the_questions_target_is_tagged_with_its_year() {
    let (_dir, db) = open_year_with(&rules());
    for account in ACCOUNTS {
        statement_ending(&db, account, "2026-11-10");
    }

    let value = serde_json::to_value(view(&db)).unwrap();

    assert_eq!(
        value["next"]["target"],
        json!({ "kind": "year", "year": 2026, "attention": null })
    );
}

#[test]
fn a_draft_opens_its_document_in_the_year() {
    let (_dir, db) = quiet();
    add_document(&db, "doc-d1", |d| {
        d.state = DocState::Draft;
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
    });

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "draft".into(),
            title: "1 draft to finish".into(),
            detail: "doc-d1".into(),
            line: "Finish it to lock its values for the tax year.".into(),
            button: "Open Draft".into(),
            target: StepTarget::Document {
                id: "doc-d1".into(),
                year: 2026
            },
        }
    );
}

#[test]
fn steps_read_in_german_with_singular_and_plural() {
    use crate::i18n::with_lang;
    use crate::settings::Language;
    let (_dir, db) = quiet();
    add_line(&db, 1, "account-1", "2026-11-09", "Lastschrift A");
    let review = with_lang(Language::De, || view(&db).next.unwrap());
    decide(&db, 1, Action::Ignore, None);
    for id in ["doc-d1", "doc-d2"] {
        add_document(&db, id, |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
            d.pay_date = Some("2026-03-03".into());
        });
    }

    let drafts = with_lang(Language::De, || view(&db).next.unwrap());

    assert_eq!(
        review,
        Step {
            key: "review".into(),
            title: "1 Buchung zu prüfen".into(),
            detail: "etwa 1 Minute".into(),
            line: "Von Private. Den Rest haben Regeln entschieden.".into(),
            button: "Prüfen starten".into(),
            target: StepTarget::Review,
        }
    );
    assert_eq!(
        drafts,
        Step {
            key: "draft".into(),
            title: "2 Entwürfe fertigzumachen".into(),
            detail: "doc-d1 und doc-d2".into(),
            line: "Schließe sie ab, um ihre Werte für das Steuerjahr festzuschreiben.".into(),
            button: "Entwürfe zeigen".into(),
            target: StepTarget::Year {
                year: 2026,
                attention: Some(AttentionKind::Draft)
            },
        }
    );
}

#[test]
fn several_drafts_are_counted_and_shown_together_in_the_year() {
    let (_dir, db) = quiet();
    for id in ["doc-d1", "doc-d2"] {
        add_document(&db, id, |d| {
            d.state = DocState::Draft;
            d.item_key = Some("books".into());
            d.pay_date = Some("2026-03-03".into());
        });
    }

    let step = view(&db).next.unwrap();

    assert_eq!(step.title, "2 drafts to finish");
    assert_eq!(
        step.line,
        "Finish them to lock their values for the tax year."
    );
    assert_eq!(step.button, "Show Drafts");
    assert_eq!(
        step.target,
        StepTarget::Year {
            year: 2026,
            attention: Some(AttentionKind::Draft)
        }
    );
}

#[test]
fn a_gap_in_the_chain_asks_to_import_the_missing_statement_when_nothing_is_behind() {
    let (_dir, db) = quiet();
    earlier_statement(&db, "account-1");

    let step = view(&db).next.unwrap();

    assert_eq!(
        step,
        Step {
            key: "gap".into(),
            title: "A Private statement is missing".into(),
            detail: "import it to close the gap".into(),
            line: "Statement Nr. 010/2026 was never imported: no lines between 31 Dec and 1 Jan."
                .into(),
            button: "Import Statement…".into(),
            target: StepTarget::Import,
        }
    );
}

#[test]
fn an_account_behind_comes_before_a_gap_in_the_import_step() {
    let (_dir, db) = quiet_ending(["2026-11-10", "2026-11-10", "2026-10-20"]);
    earlier_statement(&db, "account-1");

    let step = view(&db).next.unwrap();

    assert_eq!(step.key, "import");
    assert_eq!(step.title, "Business statement is 24 days behind");
}

#[test]
fn a_gap_in_the_chain_is_not_a_step_once_the_reminder_days_have_passed() {
    let (_dir, db) = quiet();
    earlier_statement(&db, "account-1");

    let step = view_behind_after(&db, 2).next.unwrap();

    assert_eq!(step.key, "import");
    assert_eq!(step.title, "Private statement is 3 days behind");
}

#[test]
fn an_account_is_behind_only_after_the_given_reminder_days() {
    let (_dir, db) = quiet_ending(["2026-10-20", "2026-11-10", "2026-11-10"]);

    assert_eq!(view_behind_after(&db, 30).next, None);
    assert_eq!(
        view_behind_after(&db, 23).next.map(|s| s.title),
        Some("Private statement is 24 days behind".to_string())
    );
}

#[test]
fn a_finished_document_is_not_a_draft_to_finish() {
    let (_dir, db) = quiet();
    add_document(&db, "doc-f1", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
        d.paid_cents = Some(5_000);
    });

    assert_eq!(view(&db).next, None);
}

#[test]
fn year_steps_belong_to_the_newest_open_year_not_a_newer_closed_one() {
    let (_dir, db) = quiet();
    insert(
        &db,
        taxyears::TABLE,
        &taxyears::TaxYearRow {
            state: YearState::Closed,
            closed_at: Some(2),
            ..taxyears::TaxYearRow::new(2027, "2027 v1".into(), minimal_text(2027), 1)
        },
    );
    missing_invoice(&db, 1, "2026-10-05", "Lastschrift A");

    let view = view(&db);

    assert_eq!(
        view.next.map(|s| s.target),
        Some(StepTarget::Year {
            year: 2026,
            attention: Some(AttentionKind::MissingInvoice)
        })
    );
    assert_eq!(view.year.map(|y| y.year), Some(2026));
}

#[test]
fn without_an_open_year_there_is_no_year_summary_and_no_year_steps() {
    let (_dir, db) = open_with_accounts();
    insert(
        &db,
        taxyears::TABLE,
        &taxyears::TaxYearRow {
            state: YearState::Closed,
            closed_at: Some(2),
            ..taxyears::TaxYearRow::new(2025, "2025 v1".into(), minimal_text(2025), 1)
        },
    );

    let view = view(&db);

    assert_eq!(view.year, None);
    assert_eq!(keys(&view), ["import"]);
}

#[test]
fn the_year_summary_shows_the_total_and_progress_of_the_year_page() {
    let (_dir, db) = open_year_with(&rules());
    add_document(&db, "doc-f1", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
        d.paid_cents = Some(5_000);
    });
    let page =
        year_page::page(&db, 2026, "", NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap();

    let year = view(&db).year;

    assert_eq!(
        year,
        Some(YearSummary {
            year: 2026,
            total: "€50.00".into(),
            progress: page.progress,
            progress_percent: page.progress_percent,
        })
    );
}

#[test]
fn the_year_total_follows_the_number_format() {
    let (_dir, db) = open_year_with(&rules());
    add_document(&db, "doc-f1", |d| {
        d.item_key = Some("books".into());
        d.pay_date = Some("2026-03-03".into());
        d.paid_cents = Some(123_456);
    });

    let dot = view_in(&db, NumberFormat::DotDecimal, DateOrder::DayFirst).year;
    let comma = view_in(&db, NumberFormat::CommaDecimal, DateOrder::DayFirst).year;

    assert_eq!(dot.map(|y| y.total), Some("€1,234.56".to_string()));
    assert_eq!(comma.map(|y| y.total), Some("€1.234,56".to_string()));
}

#[test]
fn an_upcoming_charge_carries_its_name_letter_tone_amount_and_when() {
    let (_dir, db) = quiet();
    add_sub(&db, 1, "Streamflix", "2026-11-14", |_| {});

    let upcoming = view(&db).upcoming;

    assert_eq!(
        upcoming,
        [UpcomingCharge {
            id: "sub-1".into(),
            name: "Streamflix".into(),
            letter: "S".into(),
            tone: 1,
            anv: false,
            when: "tomorrow".into(),
            soon: true,
            amount: "€12.99".into(),
        }]
    );
}

#[test]
fn a_subscription_with_an_item_is_marked_anv() {
    let (_dir, db) = quiet();
    add_sub(&db, 1, "Lumen AI", "2026-11-20", |s| {
        s.item_key = Some("books".into());
    });

    assert!(view(&db).upcoming[0].anv);
}

#[test]
fn the_charge_day_reads_as_today_tomorrow_in_days_or_a_full_date() {
    for (start, when, soon) in [
        ("2026-11-01", "today", true),
        ("2026-11-13", "today", true),
        ("2026-11-14", "tomorrow", true),
        ("2026-11-16", "in 3 days", true),
        ("2026-11-17", "in 4 days", false),
        ("2026-12-14", "in 31 days", false),
        ("2026-12-15", "15 Dec 2026", false),
    ] {
        let (_dir, db) = quiet();
        add_sub(&db, 1, "Streamflix", start, |_| {});

        let charge = view(&db).upcoming.remove(0);

        assert_eq!((charge.when.as_str(), charge.soon), (when, soon), "{start}");
    }
}

#[test]
fn the_charge_day_and_amount_follow_the_date_order_and_number_format() {
    let (_dir, db) = quiet();
    add_sub(&db, 1, "Streamflix", "2027-02-09", |_| {});

    let charge = view_in(&db, NumberFormat::CommaDecimal, DateOrder::MonthFirst)
        .upcoming
        .remove(0);

    assert_eq!(charge.when, "Feb 9, 2027");
    assert_eq!(charge.amount, "€12,99");
}

#[test]
fn only_the_five_nearest_charges_show_ordered_by_day_then_name() {
    let (_dir, db) = quiet();
    for (n, name, start) in [
        (1, "Zed", "2026-11-14"),
        (2, "Bravo", "2026-11-15"),
        (3, "Alpha", "2026-11-15"),
        (4, "Echo", "2026-11-16"),
        (5, "Delta", "2026-11-17"),
        (6, "Foxtrot", "2026-11-18"),
        (7, "Charlie", "2026-11-19"),
    ] {
        add_sub(&db, n, name, start, |_| {});
    }

    let view = view(&db);

    assert_eq!(
        upcoming_names(&view),
        ["Zed", "Alpha", "Bravo", "Echo", "Delta"]
    );
}

#[test]
fn an_ended_subscription_is_not_upcoming() {
    let (_dir, db) = quiet();
    add_sub(&db, 1, "Tonfeld Audiobooks", "2026-11-20", |s| {
        s.ended_on = Some("2026-06-30".into());
    });
    add_sub(&db, 2, "Streamflix", "2026-11-21", |_| {});

    let view = view(&db);

    assert_eq!(upcoming_names(&view), ["Streamflix"]);
}

#[test]
fn accounts_that_end_on_the_same_day_share_one_statements_part() {
    let (_dir, db) = quiet_ending(["2026-11-10", "2026-11-01", "2026-11-10"]);

    let view = view(&db);

    assert_eq!(
        view.statements,
        [
            StatementReach {
                text: "Private and Business until 10 Nov".into(),
                behind: false
            },
            StatementReach {
                text: "Household until 1 Nov".into(),
                behind: true
            },
        ]
    );
}

#[test]
fn statements_parts_run_from_the_newest_end_with_the_ones_behind_last() {
    let (_dir, db) = open_year_with(&minimal_rules(2026));
    statement_ending(&db, "account-1", "2026-11-08");
    statement_ending(&db, "account-2", "2026-11-10");
    statement_ending(&db, "account-3", "2026-10-20");

    let view = view(&db);

    assert_eq!(
        view.statements,
        [
            StatementReach {
                text: "Household until 10 Nov".into(),
                behind: false
            },
            StatementReach {
                text: "Private until 8 Nov".into(),
                behind: false
            },
            StatementReach {
                text: "Business until 20 Oct".into(),
                behind: true
            },
        ]
    );
}

#[test]
fn an_account_without_statements_reads_none_yet_and_counts_as_behind() {
    let (_dir, db) = open_year_with(&minimal_rules(2026));
    statement_ending(&db, "account-1", "2026-11-10");
    statement_ending(&db, "account-2", "2026-11-10");

    let view = view(&db);

    assert_eq!(
        view.statements,
        [
            StatementReach {
                text: "Private and Household until 10 Nov".into(),
                behind: false
            },
            StatementReach {
                text: "Business: none yet".into(),
                behind: true
            },
        ]
    );
}

#[test]
fn statement_ends_follow_the_date_order() {
    let (_dir, db) = quiet();

    let view = view_in(&db, NumberFormat::DotDecimal, DateOrder::MonthFirst);

    assert_eq!(
        view.statements,
        [StatementReach {
            text: "Private, Household and Business until Nov 10".into(),
            behind: false
        }]
    );
}

#[test]
fn the_date_is_the_long_weekday_and_day() {
    let (_dir, db) = quiet();

    assert_eq!(view(&db).date, "Friday, 13 November");
    assert_eq!(
        view_in(&db, NumberFormat::DotDecimal, DateOrder::MonthFirst).date,
        "Friday, November 13"
    );
}

#[test]
fn another_day_changes_the_date_and_what_is_behind() {
    let (_dir, db) = quiet();

    let view = today_view(
        &db,
        day(2026, 11, 20),
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
        BEHIND_AFTER_DAYS,
    )
    .unwrap();

    assert_eq!(view.date, "Friday, 20 November");
    assert_eq!(view.next.map(|s| s.key), Some("import".to_string()));
}

#[test]
fn the_view_serializes_with_camel_case_fields_and_tagged_targets() {
    let (_dir, db) = busy();
    let view = view(&db);

    let value = serde_json::to_value(&view).unwrap();

    let mut keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        ["also", "date", "next", "statements", "upcoming", "year"]
    );
    assert_eq!(
        value["year"]["progressPercent"],
        json!(view.year.unwrap().progress_percent)
    );
    assert_eq!(value["next"]["target"], json!({ "kind": "review" }));
    assert_eq!(
        value["also"][0]["target"],
        json!({ "kind": "year", "year": 2026, "attention": "missing_invoice" })
    );
    assert_eq!(
        value["also"][2]["target"],
        json!({ "kind": "year", "year": 2026, "attention": null })
    );
    assert_eq!(value["also"][1]["target"], json!({ "kind": "import" }));
    assert_eq!(
        value["also"][3]["target"],
        json!({ "kind": "document", "id": "doc-d1", "year": 2026 })
    );
}
