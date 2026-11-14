//! Reminders as macOS notifications (ADR 0042): which ones are due today, each told once. The running Dock app
//! asks at launch and every half hour; `sent.rs` remembers what was told.

pub mod sent;

use chrono::{Datelike, Days, NaiveDate};

use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::notify::Notification;
use crate::rules::store as decisions;
use crate::settings::Settings;
use crate::statements::store as statements;
use crate::statements::view as statement_view;
use crate::subscriptions::Book;
use crate::subscriptions::matcher::{self, Matchers};
use crate::subscriptions::store::Reminder;
use crate::subscriptions::view::Format;
use crate::taxyears::{self, YearState};

/// A notification with the key that keeps it from being told twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    pub key: String,
    pub notification: Notification,
}

fn due(key: String, title: String, body: String) -> Due {
    Due {
        key,
        notification: Notification { title, body },
    }
}

fn days_before(reminder: Reminder) -> Option<u64> {
    match reminder {
        Reminder::Off => None,
        Reminder::OneDay => Some(1),
        Reminder::ThreeDays => Some(3),
        Reminder::OneWeek => Some(7),
    }
}

fn renewals(db: &Db, today: NaiveDate, format: &Format) -> AppResult<Vec<Due>> {
    let book = db.read(Book::load)?;
    let mut out = Vec::new();
    for sub in &book.subscriptions {
        let running = book.running(sub);
        let (Some(days), Some(next)) = (days_before(sub.reminder), running.next_charge(today))
        else {
            continue;
        };
        let Some(from) = next.checked_sub_days(Days::new(days)) else {
            continue;
        };
        if !(from..=next).contains(&today) {
            continue;
        }
        let amount = running
            .charge_cents(today)
            .map(|c| format!("{} on ", format.eur(c)))
            .unwrap_or_default();
        out.push(due(
            format!("renewal:{}:{}", sub.id, date::iso(next)),
            format!("{} renews soon", sub.name),
            format!("{amount}{}", date::weekday_short(next, format.order)),
        ));
    }
    Ok(out)
}

fn price_changes(db: &Db) -> AppResult<Vec<Due>> {
    let matchers = db.read(Matchers::load)?;
    let decided = db.read(decisions::decisions)?;
    let lines = db.read(statements::all_transactions)?;
    let asked = matchers.assign(&lines, &matcher::taken(&decided, false), |t| {
        !decided.contains_key(&t.id)
    });
    Ok(lines
        .iter()
        .filter_map(|t| {
            let (sub, _) = asked.asks_about(&t.id)?;
            Some(due(
                format!("plan:{}", t.id),
                format!("{} charged a different amount", sub.name),
                "Review the line: a new price, a one-off, or not this subscription.".into(),
            ))
        })
        .collect())
}

fn last_statement_end(db: &Db, account_id: &str) -> AppResult<Option<NaiveDate>> {
    Ok(db
        .read(|c| statements::statements_of(c, account_id))?
        .iter()
        .filter_map(|s| date::parse(&s.to_date))
        .max())
}

fn statements_due(
    db: &Db,
    settings: &Settings,
    today: NaiveDate,
    order: DateOrder,
) -> AppResult<Vec<Due>> {
    let mut out = Vec::new();
    for account in
        statement_view::coverage(db, today, "", order, settings.import_after_days)?.accounts
    {
        if let Some(end) = last_statement_end(db, &account.id)?
            && end + Days::new(u64::from(settings.import_after_days)) <= today
        {
            out.push(due(
                format!("import:{}:{}", account.id, date::iso(end)),
                format!("Import the {} statement", account.name),
                format!(
                    "Its last statement ended on {}.",
                    date::day_month(end, order)
                ),
            ));
        }
        if settings.remind_gaps
            && let Some(note) = &account.note
        {
            out.push(due(
                format!("gap:{}:{note}", account.id),
                format!("A {} statement is missing", account.name),
                note.clone(),
            ));
        }
    }
    Ok(out)
}

fn tax_years(db: &Db, settings: &Settings, today: NaiveDate) -> AppResult<Vec<Due>> {
    let open: Vec<i32> = db
        .read(taxyears::rows)?
        .into_iter()
        .filter(|y| y.state == YearState::Open)
        .map(|y| y.year)
        .collect();
    let mut out = Vec::new();
    for &year in &open {
        let filing = NaiveDate::from_ymd_opt(year + 1, 3, 1);
        if settings.remind_filing && filing.is_some_and(|d| today >= d) {
            out.push(due(
                format!("filing:{year}"),
                format!("Tax year {year} can be closed"),
                "Close it in Hamster, then copy the Kennzahlen into FinanzOnline.".into(),
            ));
        }
    }
    for &year in &open {
        if settings.remind_questions && crate::yearcheck::due_on(db, year, today)? {
            let (phase, body) = if today.year() > year {
                (
                    "after",
                    "The year is over: run the year check once more before closing it.",
                )
            } else {
                (
                    "november",
                    "A few quick questions about things you might forget to claim.",
                )
            };
            out.push(due(
                format!("yearcheck:{year}:{phase}"),
                format!("Year check for Tax year {year}"),
                body.into(),
            ));
        }
    }
    Ok(out)
}

/// Every reminder due on `today`, told or not.
pub fn due_today(
    db: &Db,
    settings: &Settings,
    today: NaiveDate,
    order: DateOrder,
) -> AppResult<Vec<Due>> {
    let format = Format {
        number: settings.number_format,
        order,
    };
    let mut out = renewals(db, today, &format)?;
    if settings.remind_price_changes {
        out.extend(price_changes(db)?);
    }
    out.extend(statements_due(db, settings, today, order)?);
    out.extend(tax_years(db, settings, today)?);
    Ok(out)
}

/// Tells every due reminder not told before through `post`; returns how many.
pub fn tell(
    db: &Db,
    config_dir: &std::path::Path,
    settings: &Settings,
    today: NaiveDate,
    order: DateOrder,
    post: impl Fn(Notification),
) -> AppResult<usize> {
    let mut told = sent::load(config_dir);
    let fresh: Vec<Due> = due_today(db, settings, today, order)?
        .into_iter()
        .filter(|d| !told.contains(&d.key))
        .collect();
    for reminder in &fresh {
        post(reminder.notification.clone());
        told.insert(reminder.key.clone());
    }
    if !fresh.is_empty() {
        sent::save(config_dir, &told)?;
    }
    Ok(fresh.len())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::db::Source;
    use crate::statements::import_tests::open_with_accounts;
    use crate::subscriptions::edit::SubscriptionForm;
    use crate::subscriptions::store::{PlanRow, SubscriptionRow};
    use crate::taxyears::TaxYearRow;
    use crate::test_support::rules::{local_millis, minimal_rules, minimal_text, rules_text, set};
    use crate::test_support::statements::demo_month;
    use crate::test_support::subscriptions::{
        add, day, form, imported, insert_rows, plan_row, subscription_row, today,
    };
    use crate::year::tests::{insert, open_year};

    const ORDER: DateOrder = DateOrder::DayFirst;

    fn due_with(db: &Db, settings: &Settings, on: NaiveDate, prefix: &str) -> Vec<Due> {
        due_today(db, settings, on, ORDER)
            .unwrap()
            .into_iter()
            .filter(|d| d.key.starts_with(prefix))
            .collect()
    }

    fn keys(dues: &[Due]) -> Vec<&str> {
        dues.iter().map(|d| d.key.as_str()).collect()
    }

    fn notification(title: &str, body: &str) -> Notification {
        Notification {
            title: title.into(),
            body: body.into(),
        }
    }

    fn renewing(
        id: &str,
        name: &str,
        reminder: Reminder,
        first_charge: &str,
    ) -> (SubscriptionRow, PlanRow) {
        (
            SubscriptionRow {
                reminder,
                ..subscription_row(id, name, &name.to_uppercase())
            },
            plan_row(&format!("plan-{id}"), id, 1_299, first_charge, None),
        )
    }

    fn add_renewing(db: &Db, id: &str, name: &str, reminder: Reminder, first_charge: &str) {
        let (sub, plan) = renewing(id, name, reminder, first_charge);
        insert_rows(db, &sub, &[plan]);
    }

    fn year_row(year: i32, closed: bool) -> TaxYearRow {
        TaxYearRow {
            state: if closed {
                YearState::Closed
            } else {
                YearState::Open
            },
            closed_at: closed.then_some(2),
            ..TaxYearRow::new(year, format!("{year} v1"), minimal_text(year), 1)
        }
    }

    fn settings_with(patch: impl FnOnce(&mut Settings)) -> Settings {
        let mut settings = Settings::default();
        patch(&mut settings);
        settings
    }

    fn quiet_about_years() -> Settings {
        settings_with(|s| {
            s.remind_filing = false;
            s.remind_questions = false;
        })
    }

    #[test]
    fn a_renewal_is_due_from_the_reminder_days_before_the_charge_through_its_day() {
        let (_dir, db) = imported(&[]);
        add_renewing(&db, "s1", "Streamflix", Reminder::ThreeDays, "2026-11-16");

        for (on, expected) in [
            (day(2026, 11, 12), vec![]),
            (day(2026, 11, 13), vec!["renewal:s1:2026-11-16"]),
            (day(2026, 11, 15), vec!["renewal:s1:2026-11-16"]),
            (day(2026, 11, 16), vec!["renewal:s1:2026-11-16"]),
            (day(2026, 11, 17), vec![]),
        ] {
            let dues = due_with(&db, &Settings::default(), on, "renewal:");
            assert_eq!(keys(&dues), expected, "{on}");
        }
    }

    #[test]
    fn a_renewal_names_the_subscription_the_amount_and_the_weekday() {
        let (_dir, db) = imported(&[]);
        add_renewing(&db, "s1", "Streamflix", Reminder::ThreeDays, "2026-11-16");

        let dues = due_with(&db, &Settings::default(), today(), "renewal:");

        assert_eq!(
            dues,
            vec![Due {
                key: "renewal:s1:2026-11-16".into(),
                notification: notification("Streamflix renews soon", "€12.99 on Mon, 16 Nov"),
            }]
        );
    }

    #[test]
    fn the_reminder_length_decides_the_first_day_a_renewal_is_due() {
        for (reminder, first_due, day_before) in [
            (Reminder::OneDay, day(2026, 11, 15), day(2026, 11, 14)),
            (Reminder::OneWeek, day(2026, 11, 9), day(2026, 11, 8)),
        ] {
            let (_dir, db) = imported(&[]);
            add_renewing(&db, "s1", "Streamflix", reminder, "2026-11-16");

            let on = due_with(&db, &Settings::default(), first_due, "renewal:");
            let before = due_with(&db, &Settings::default(), day_before, "renewal:");

            assert_eq!(keys(&on), ["renewal:s1:2026-11-16"], "{reminder:?}");
            assert_eq!(keys(&before), Vec::<&str>::new(), "{reminder:?}");
        }
    }

    #[test]
    fn a_subscription_with_its_reminder_off_never_renews_soon() {
        let (_dir, db) = imported(&[]);
        add_renewing(&db, "s1", "Streamflix", Reminder::Off, "2026-11-16");

        for on in [day(2026, 11, 9), day(2026, 11, 13), day(2026, 11, 16)] {
            assert_eq!(
                due_with(&db, &Settings::default(), on, "renewal:"),
                vec![],
                "{on}"
            );
        }
    }

    #[test]
    fn an_ended_subscription_never_renews_soon() {
        let (_dir, db) = imported(&[]);
        let (sub, plan) = renewing("s1", "Streamflix", Reminder::ThreeDays, "2026-11-16");
        let ended = SubscriptionRow {
            ended_on: Some("2026-11-01".into()),
            ..sub
        };
        insert_rows(&db, &ended, &[plan]);

        assert_eq!(
            due_with(&db, &Settings::default(), today(), "renewal:"),
            vec![]
        );
    }

    fn lumen_form() -> SubscriptionForm {
        SubscriptionForm {
            currency: "USD".into(),
            ..form("Lumen AI", "LUMEN AI", "20.00")
        }
    }

    #[test]
    fn a_line_charged_at_another_amount_than_its_plan_asks_about_the_price() {
        let (_dir, db) = imported(&[demo_month(10, 2026, 10, 141_230)]);
        add(&db, &lumen_form());
        let line = db
            .read(statements::all_transactions)
            .unwrap()
            .into_iter()
            .find(|t| format!("{} {}", t.text, t.extra).contains("LUMEN AI"))
            .unwrap();

        let dues = due_with(&db, &Settings::default(), today(), "plan:");

        assert_eq!(
            dues,
            vec![Due {
                key: format!("plan:{}", line.id),
                notification: notification(
                    "Lumen AI charged a different amount",
                    "Review the line: a new price, a one-off, or not this subscription."
                ),
            }]
        );
    }

    #[test]
    fn price_change_reminders_can_be_turned_off() {
        let (_dir, db) = imported(&[demo_month(10, 2026, 10, 141_230)]);
        add(&db, &lumen_form());
        let settings = settings_with(|s| s.remind_price_changes = false);

        assert_eq!(due_with(&db, &settings, today(), "plan:"), vec![]);
    }

    #[test]
    fn an_account_is_due_for_import_once_the_days_since_its_last_statement_have_passed() {
        let (_dir, db) = imported(&[demo_month(10, 2026, 10, 141_230)]);

        for (on, expected) in [
            (day(2026, 11, 19), vec![]),
            (day(2026, 11, 20), vec!["import:account-1:2026-10-21"]),
            (day(2026, 12, 15), vec!["import:account-1:2026-10-21"]),
        ] {
            let dues = due_with(&db, &Settings::default(), on, "import:");
            assert_eq!(keys(&dues), expected, "{on}");
        }
    }

    #[test]
    fn the_import_reminder_names_the_account_and_the_end_of_its_last_statement() {
        let (_dir, db) = imported(&[demo_month(10, 2026, 10, 141_230)]);

        let dues = due_with(&db, &Settings::default(), day(2026, 11, 20), "import:");

        assert_eq!(
            dues,
            vec![Due {
                key: "import:account-1:2026-10-21".into(),
                notification: notification(
                    "Import the Private statement",
                    "Its last statement ended on 21 Oct."
                ),
            }]
        );
    }

    #[test]
    fn the_import_reminder_follows_the_configured_days() {
        let (_dir, db) = imported(&[demo_month(10, 2026, 10, 141_230)]);
        let settings = settings_with(|s| s.import_after_days = 5);

        for (on, expected) in [
            (day(2026, 10, 25), vec![]),
            (day(2026, 10, 26), vec!["import:account-1:2026-10-21"]),
        ] {
            assert_eq!(
                keys(&due_with(&db, &settings, on, "import:")),
                expected,
                "{on}"
            );
        }
    }

    #[test]
    fn an_account_without_statements_has_no_import_reminder() {
        let (_dir, db) = imported(&[]);

        assert_eq!(
            due_with(&db, &Settings::default(), day(2027, 6, 1), "import:"),
            vec![]
        );
    }

    fn gap_statements() -> Vec<crate::test_support::statements::Statement> {
        let august = demo_month(8, 2026, 8, 141_230);
        let october = demo_month(10, 2026, 10, august.new_balance());
        vec![august, october]
    }

    #[test]
    fn a_gap_in_the_statements_is_reminded_with_its_note() {
        let (_dir, db) = imported(&gap_statements());

        let dues = due_with(&db, &Settings::default(), today(), "gap:");

        assert_eq!(
            dues,
            vec![Due {
                key: "gap:account-1:Statement Nr. 009/2026 was never imported: no lines between 21 Aug and 1 Oct."
                    .into(),
                notification: notification(
                    "A Private statement is missing",
                    "Statement Nr. 009/2026 was never imported: no lines between 21 Aug and 1 Oct."
                ),
            }]
        );
    }

    #[test]
    fn gap_reminders_can_be_turned_off() {
        let (_dir, db) = imported(&gap_statements());
        let settings = settings_with(|s| s.remind_gaps = false);

        assert_eq!(due_with(&db, &settings, today(), "gap:"), vec![]);
    }

    #[test]
    fn a_chain_without_a_gap_has_no_gap_reminder() {
        let september = demo_month(9, 2026, 9, 141_230);
        let october = demo_month(10, 2026, 10, september.new_balance());
        let (_dir, db) = imported(&[september, october]);

        assert_eq!(due_with(&db, &Settings::default(), today(), "gap:"), vec![]);
    }

    #[test]
    fn an_open_year_can_be_closed_from_the_first_of_march_after_it() {
        let (_dir, db) = imported(&[]);
        insert(&db, crate::taxyears::TABLE, &year_row(2025, false));

        for (on, expected) in [
            (day(2026, 2, 28), vec![]),
            (day(2026, 3, 1), vec!["filing:2025"]),
            (day(2026, 11, 13), vec!["filing:2025"]),
        ] {
            let dues = due_with(&db, &Settings::default(), on, "filing:");
            assert_eq!(keys(&dues), expected, "{on}");
        }
    }

    #[test]
    fn the_filing_reminder_names_the_year() {
        let (_dir, db) = imported(&[]);
        insert(&db, crate::taxyears::TABLE, &year_row(2025, false));

        let dues = due_with(&db, &Settings::default(), today(), "filing:");

        assert_eq!(
            dues,
            vec![Due {
                key: "filing:2025".into(),
                notification: notification(
                    "Tax year 2025 can be closed",
                    "Close it in Hamster, then copy the Kennzahlen into FinanzOnline."
                ),
            }]
        );
    }

    #[test]
    fn every_open_year_past_its_filing_day_is_reminded_and_closed_years_never() {
        let (_dir, db) = imported(&[]);
        insert(&db, crate::taxyears::TABLE, &year_row(2025, false));
        insert(&db, crate::taxyears::TABLE, &year_row(2024, false));
        insert(&db, crate::taxyears::TABLE, &year_row(2023, true));

        let dues = due_with(&db, &Settings::default(), today(), "filing:");

        let mut found = keys(&dues);
        found.sort_unstable();
        assert_eq!(found, ["filing:2024", "filing:2025"]);
    }

    #[test]
    fn filing_reminders_can_be_turned_off() {
        let (_dir, db) = imported(&[]);
        insert(&db, crate::taxyears::TABLE, &year_row(2025, false));
        let settings = settings_with(|s| s.remind_filing = false);

        assert_eq!(due_with(&db, &settings, today(), "filing:"), vec![]);
    }

    fn asking_rules() -> serde_json::Value {
        let mut rules = minimal_rules(2026);
        set(
            &mut rules,
            "/items/0/ask",
            serde_json::json!({ "order": 1, "question": "Did you buy books?" }),
        );
        rules
    }

    fn asking_year(state: YearState) -> (tempfile::TempDir, Db) {
        let (dir, db) = open_with_accounts();
        insert(
            &db,
            crate::taxyears::TABLE,
            &TaxYearRow {
                state,
                closed_at: (state == YearState::Closed).then_some(2),
                ..TaxYearRow::new(2026, "2026 v1".into(), rules_text(&asking_rules()), 1)
            },
        );
        (dir, db)
    }

    fn ran_on(db: &Db, (y, m, d): (i32, u32, u32)) {
        crate::yearcheck::complete(db, 2026, local_millis(y, m, d), Source::Ui).unwrap();
    }

    #[test]
    fn the_year_check_is_reminded_in_november_and_december_and_again_after_the_year() {
        let (_dir, db) = asking_year(YearState::Open);

        for (on, expected) in [
            (day(2026, 10, 31), vec![]),
            (day(2026, 11, 1), vec!["yearcheck:2026:november"]),
            (day(2026, 12, 31), vec!["yearcheck:2026:november"]),
            (day(2027, 1, 1), vec!["yearcheck:2026:after"]),
        ] {
            let dues = due_with(&db, &Settings::default(), on, "yearcheck:");
            assert_eq!(keys(&dues), expected, "{on}");
        }
    }

    #[test]
    fn the_year_check_reminders_name_the_year_and_say_what_to_do() {
        let (_dir, db) = asking_year(YearState::Open);
        let settings = Settings::default();

        assert_eq!(
            due_with(&db, &settings, today(), "yearcheck:"),
            vec![Due {
                key: "yearcheck:2026:november".into(),
                notification: notification(
                    "Year check for Tax year 2026",
                    "A few quick questions about things you might forget to claim."
                ),
            }]
        );
        assert_eq!(
            due_with(&db, &settings, day(2027, 1, 9), "yearcheck:"),
            vec![Due {
                key: "yearcheck:2026:after".into(),
                notification: notification(
                    "Year check for Tax year 2026",
                    "The year is over: run the year check once more before closing it."
                ),
            }]
        );
    }

    #[test]
    fn a_completed_run_silences_the_reminder_until_the_year_is_over() {
        let (_dir, db) = asking_year(YearState::Open);
        let settings = Settings::default();
        ran_on(&db, (2026, 11, 10));

        assert_eq!(due_with(&db, &settings, today(), "yearcheck:"), vec![]);
        assert_eq!(
            keys(&due_with(&db, &settings, day(2027, 1, 9), "yearcheck:")),
            ["yearcheck:2026:after"]
        );

        ran_on(&db, (2027, 1, 9));
        assert_eq!(
            due_with(&db, &settings, day(2027, 1, 9), "yearcheck:"),
            vec![]
        );
    }

    #[test]
    fn year_check_reminders_can_be_turned_off() {
        let (_dir, db) = asking_year(YearState::Open);
        let settings = settings_with(|s| s.remind_questions = false);

        for on in [today(), day(2027, 1, 9)] {
            assert_eq!(due_with(&db, &settings, on, "yearcheck:"), vec![], "{on}");
        }
    }

    #[test]
    fn a_closed_year_and_a_ruleset_without_ask_are_never_reminded() {
        let (_closed_dir, closed) = asking_year(YearState::Closed);
        let (_plain_dir, plain) = open_year();

        for on in [today(), day(2027, 1, 9)] {
            let settings = Settings::default();
            assert_eq!(
                due_with(&closed, &settings, on, "yearcheck:"),
                vec![],
                "closed {on}"
            );
            assert_eq!(
                due_with(&plain, &settings, on, "yearcheck:"),
                vec![],
                "plain {on}"
            );
        }
    }

    #[test]
    fn nothing_is_due_without_subscriptions_statements_or_open_questions() {
        let (_dir, db) = imported(&[]);

        assert_eq!(
            due_today(&db, &quiet_about_years(), today(), ORDER).unwrap(),
            vec![]
        );
    }

    #[test]
    fn tell_posts_each_due_reminder_once_and_remembers_it() {
        let (dir, db) = imported(&[]);
        add_renewing(&db, "s1", "Streamflix", Reminder::ThreeDays, "2026-11-16");
        let settings = quiet_about_years();
        let posted = RefCell::new(Vec::new());

        let first = tell(&db, dir.path(), &settings, today(), ORDER, |n| {
            posted.borrow_mut().push(n)
        })
        .unwrap();

        assert_eq!(first, 1);
        assert_eq!(
            *posted.borrow(),
            vec![notification(
                "Streamflix renews soon",
                "€12.99 on Mon, 16 Nov"
            )]
        );
        assert_eq!(
            sent::load(dir.path()).into_iter().collect::<Vec<_>>(),
            ["renewal:s1:2026-11-16"]
        );
        assert!(dir.path().join("reminders.json").exists());

        let again = tell(&db, dir.path(), &settings, today(), ORDER, |n| {
            posted.borrow_mut().push(n)
        })
        .unwrap();

        assert_eq!(again, 0);
        assert_eq!(posted.borrow().len(), 1);
    }

    #[test]
    fn tell_posts_a_reminder_that_becomes_due_later_and_not_the_ones_told_before() {
        let (dir, db) = imported(&[]);
        add_renewing(&db, "s1", "Streamflix", Reminder::ThreeDays, "2026-11-16");
        let settings = quiet_about_years();
        let posted = RefCell::new(Vec::new());
        tell(&db, dir.path(), &settings, today(), ORDER, |n| {
            posted.borrow_mut().push(n)
        })
        .unwrap();
        posted.borrow_mut().clear();

        add_renewing(
            &db,
            "s2",
            "Tonfeld Audiobooks",
            Reminder::ThreeDays,
            "2026-11-14",
        );
        let count = tell(&db, dir.path(), &settings, today(), ORDER, |n| {
            posted.borrow_mut().push(n)
        })
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            *posted.borrow(),
            vec![notification(
                "Tonfeld Audiobooks renews soon",
                "€12.99 on Sat, 14 Nov"
            )]
        );
        assert_eq!(
            sent::load(dir.path()).into_iter().collect::<Vec<_>>(),
            ["renewal:s1:2026-11-16", "renewal:s2:2026-11-14"]
        );
    }

    #[test]
    fn tell_with_nothing_due_posts_nothing_and_writes_no_file() {
        let (dir, db) = imported(&[]);
        let posted = RefCell::new(Vec::new());

        let count = tell(&db, dir.path(), &quiet_about_years(), today(), ORDER, |n| {
            posted.borrow_mut().push(n)
        })
        .unwrap();

        assert_eq!(count, 0);
        assert!(posted.borrow().is_empty());
        assert!(!dir.path().join("reminders.json").exists());
    }
}
