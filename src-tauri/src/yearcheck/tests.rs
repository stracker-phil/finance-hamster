//! The year check against a temp database with a 2026 tax year whose rules ask six things.

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::statements::import_tests::open_with_accounts;
use crate::taxyears::TaxYearRow;
use crate::test_support::rules::{local_millis, minimal_rules, rules_text, set};
use crate::test_support::subscriptions::day;
use crate::year::tests::{insert, open_year, open_year_with, rules};

const NOW: i64 = 1_700_000_000;

/// The year rules with asks on parking and Telearbeitstage (order 1 and 2), phone and books (2 and 3), tips (3)
/// and the Telearbeitspauschale (5); a hidden item and one reported by others never get asked.
fn asking() -> Value {
    let mut rules = rules();
    for (pointer, order, question) in [
        ("/items/3/ask", 1, "Did you drive to a client?"),
        (
            "/questions/0/ask",
            2,
            "How many days did you work from home?",
        ),
        ("/items/0/ask", 3, "Did you buy books?"),
        ("/items/2/ask", 3, "Did you pay tips?"),
        (
            "/questions/1/ask",
            5,
            "What did the employer pay for home office?",
        ),
    ] {
        set(
            &mut rules,
            pointer,
            json!({ "order": order, "question": question }),
        );
    }
    set(
        &mut rules,
        "/items/1/ask",
        json!({ "order": 2, "question": "Did you buy a phone?", "hint": "Contract or device" }),
    );
    rules
}

fn asking_for(year: i32) -> Value {
    let mut rules = asking();
    set(&mut rules, "/year", json!(year));
    set(&mut rules, "/ruleset", json!(format!("{year} v1")));
    rules
}

fn one_ask() -> Value {
    let mut rules = minimal_rules(2026);
    set(
        &mut rules,
        "/items/0/ask",
        json!({ "order": 1, "question": "Did you buy books?" }),
    );
    rules
}

fn asked_year() -> (TempDir, Db) {
    open_year_with(&asking())
}

fn year_with(rules: &Value, state: YearState) -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    let year = rules["year"].as_i64().unwrap() as i32;
    insert(
        &db,
        taxyears::TABLE,
        &TaxYearRow {
            state,
            closed_at: (state == YearState::Closed).then_some(2),
            ..TaxYearRow::new(year, format!("{year} v1"), rules_text(rules), 1)
        },
    );
    (dir, db)
}

fn run(db: &Db, year: i32, (y, m, d): (i32, u32, u32)) -> Written {
    complete(db, year, local_millis(y, m, d), Source::Ui).unwrap()
}

fn view_on(db: &Db, on: NaiveDate) -> YearCheckView {
    view(db, 2026, on, NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap()
}

fn listed(db: &Db, format: NumberFormat) -> Vec<CheckQuestion> {
    let ruleset = serde_json::from_value(asking()).unwrap();
    questions(db, &ruleset, format).unwrap()
}

fn rows_stored(db: &Db) -> i64 {
    db.read(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {TABLE}"), [], |r| r.get(0))?))
        .unwrap()
}

#[test]
fn questions_follow_their_order_with_questions_before_items_and_file_order_on_a_tie() {
    let (_dir, db) = asked_year();

    let keys: Vec<String> = listed(&db, NumberFormat::DotDecimal)
        .into_iter()
        .map(|q| q.key)
        .collect();

    assert_eq!(
        keys,
        [
            "parking",
            "telearbeitstage",
            "phone",
            "books",
            "tips",
            "pauschale"
        ]
    );
}

#[test]
fn an_item_question_names_its_kennzahl_and_what_yes_adds() {
    let (_dir, db) = asked_year();
    let all = listed(&db, NumberFormat::DotDecimal);

    let item = |key: &str| all.iter().find(|q| q.key == key).unwrap().clone();

    assert_eq!(
        item("parking"),
        CheckQuestion {
            key: "parking".into(),
            kz: Some("724".into()),
            question: "Did you drive to a client?".into(),
            hint: None,
            add: Some(AddKind::Trip),
            unit: None,
            value: String::new(),
        }
    );
    assert_eq!(
        item("phone"),
        CheckQuestion {
            key: "phone".into(),
            kz: Some("721".into()),
            question: "Did you buy a phone?".into(),
            hint: Some("Contract or device".into()),
            add: Some(AddKind::Document),
            unit: None,
            value: String::new(),
        }
    );
    assert_eq!(item("tips").add, Some(AddKind::Amount));
}

#[test]
fn a_number_question_carries_its_unit_and_current_answer_as_the_year_page_shows_it() {
    let (_dir, db) = asked_year();
    let ruleset: Ruleset = serde_json::from_value(asking()).unwrap();
    let before = listed(&db, NumberFormat::DotDecimal);
    answers::answer(
        &db,
        &ruleset,
        2026,
        "telearbeitstage",
        "28",
        NOW,
        Source::Ui,
    )
    .unwrap();
    answers::answer(&db, &ruleset, 2026, "pauschale", "1234.5", NOW, Source::Ui).unwrap();

    let dot = listed(&db, NumberFormat::DotDecimal);
    let comma = listed(&db, NumberFormat::CommaDecimal);

    let numbers = |list: &[CheckQuestion]| -> Vec<(String, Option<String>, String)> {
        list.iter()
            .filter(|q| q.kz.is_none())
            .map(|q| (q.key.clone(), q.unit.clone(), q.value.clone()))
            .collect()
    };
    assert_eq!(
        numbers(&before),
        [
            ("telearbeitstage".into(), Some("days".into()), String::new()),
            ("pauschale".into(), None, String::new())
        ]
    );
    assert_eq!(
        numbers(&dot),
        [
            ("telearbeitstage".into(), Some("days".into()), "28".into()),
            ("pauschale".into(), None, "1,234.50".into())
        ]
    );
    assert_eq!(numbers(&comma)[1].2, "1.234,50");
    let number = &dot[1];
    assert_eq!(
        (number.kz.as_deref(), number.add, number.question.as_str()),
        (None, None, "How many days did you work from home?")
    );
}

#[test]
fn removing_ask_removes_the_question() {
    let mut without = asking();
    without["items"][3].as_object_mut().unwrap().remove("ask");
    without["questions"][0]
        .as_object_mut()
        .unwrap()
        .remove("ask");
    let (_dir, db) = open_year_with(&without);
    let ruleset: Ruleset = serde_json::from_value(without).unwrap();

    let keys: Vec<String> = questions(&db, &ruleset, NumberFormat::DotDecimal)
        .unwrap()
        .into_iter()
        .map(|q| q.key)
        .collect();

    assert_eq!(keys, ["phone", "books", "tips", "pauschale"]);
}

#[test]
fn a_hidden_item_and_one_reported_by_others_are_never_asked() {
    let mut rules = asking();
    for pointer in ["/items/4/ask", "/items/5/ask"] {
        set(
            &mut rules,
            pointer,
            json!({ "order": 0, "question": "Hidden?" }),
        );
    }
    let (_dir, db) = open_year_with(&rules);
    let ruleset: Ruleset = serde_json::from_value(rules).unwrap();

    let keys: Vec<String> = questions(&db, &ruleset, NumberFormat::DotDecimal)
        .unwrap()
        .into_iter()
        .map(|q| q.key)
        .collect();

    assert_eq!(keys.len(), 6);
    assert!(!keys.contains(&"secret".to_string()) && !keys.contains(&"employer".to_string()));
}

#[test]
fn a_year_without_a_run_has_no_last_run_and_a_completed_one_reads_as_its_local_day() {
    let (_dir, db) = asked_year();
    assert_eq!(last_run(&db, 2026).unwrap(), None);

    run(&db, 2026, (2026, 6, 2));

    assert_eq!(last_run(&db, 2026).unwrap(), Some(day(2026, 6, 2)));
    assert_eq!(last_run(&db, 2025).unwrap(), None);
}

#[test]
fn the_line_tells_when_the_check_ran_and_whether_it_is_still_up_to_date() {
    let cases: [(Option<(i32, u32, u32)>, NaiveDate, &str); 5] = [
        (None, day(2026, 11, 13), "Year check: not run yet"),
        (
            Some((2026, 6, 2)),
            day(2026, 11, 13),
            "Year check: last run 2 Jun 2026 · run it again after 31 Dec",
        ),
        (
            Some((2026, 6, 2)),
            day(2027, 1, 9),
            "Year check: last run 2 Jun 2026 · purchases and trips since then may be missing",
        ),
        (
            Some((2027, 1, 9)),
            day(2027, 1, 9),
            "Year check: last run 9 Jan 2027",
        ),
        (
            Some((2026, 12, 31)),
            day(2026, 12, 31),
            "Year check: last run 31 Dec 2026 · run it again after 31 Dec",
        ),
    ];
    for (ran, today, expected) in cases {
        let (_dir, db) = asked_year();
        if let Some(on) = ran {
            run(&db, 2026, on);
        }

        assert_eq!(view_on(&db, today).line, expected, "{ran:?} on {today}");
    }
}

#[test]
fn the_view_reports_the_last_run_label_in_the_date_order_and_whether_the_year_is_over() {
    let (_dir, db) = asked_year();
    run(&db, 2026, (2026, 11, 14));

    let day_first = view_on(&db, day(2026, 12, 31));
    let month_first = view(
        &db,
        2026,
        day(2027, 1, 1),
        NumberFormat::DotDecimal,
        DateOrder::MonthFirst,
    )
    .unwrap();

    assert_eq!(day_first.last_run.as_deref(), Some("14 Nov 2026"));
    assert!(!day_first.year_over);
    assert_eq!(month_first.last_run.as_deref(), Some("Nov 14, 2026"));
    assert!(month_first.year_over);
    assert!(day_first.open);
}

#[test]
fn the_close_prompt_names_the_missing_or_early_run_and_how_many_questions_wait() {
    let questions = "It takes a minute: 6 questions, most answered with No.";
    let (_dir, db) = asked_year();

    assert_eq!(
        view_on(&db, day(2027, 1, 9)).close_prompt,
        Some(format!(
            "The year check for 2026 hasn't run yet. {questions}"
        ))
    );

    run(&db, 2026, (2026, 6, 2));
    assert_eq!(
        view_on(&db, day(2027, 1, 9)).close_prompt,
        Some(format!(
            "The last year check for 2026 was on 2 Jun 2026, before the year ended. Purchases and trips since then may be missing. {questions}"
        ))
    );

    run(&db, 2026, (2027, 1, 9));
    assert_eq!(view_on(&db, day(2027, 1, 9)).close_prompt, None);
}

#[test]
fn a_single_question_reads_in_the_singular_in_the_close_prompt() {
    let (_dir, db) = year_with(&one_ask(), YearState::Open);

    assert_eq!(
        view_on(&db, day(2027, 1, 9)).close_prompt,
        Some(
            "The year check for 2026 hasn't run yet. It takes a minute: 1 question, most answered with No."
                .to_string()
        )
    );
}

#[test]
fn a_ruleset_without_ask_has_no_questions_and_no_close_prompt() {
    let (_dir, db) = open_year();

    let view = view_on(&db, day(2027, 1, 9));

    assert!(view.questions.is_empty());
    assert_eq!(view.close_prompt, None);
}

#[test]
fn the_view_serializes_with_camel_case_fields() {
    let (_dir, db) = asked_year();
    run(&db, 2026, (2026, 11, 14));

    let value = serde_json::to_value(view_on(&db, day(2026, 11, 14))).unwrap();

    let mut keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "closePrompt",
            "lastRun",
            "line",
            "open",
            "questions",
            "year",
            "yearOver"
        ]
    );
    assert_eq!(
        value["questions"][0],
        json!({
            "key": "parking", "kz": "724", "question": "Did you drive to a client?",
            "hint": null, "add": "trip", "unit": null, "value": ""
        })
    );
    assert_eq!(value["lastRun"], json!("14 Nov 2026"));
}

#[test]
fn a_year_that_does_not_exist_is_not_found() {
    let (_dir, db) = asked_year();

    let shown = view(
        &db,
        2031,
        day(2031, 1, 1),
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    );

    assert!(matches!(shown, Err(AppError::NotFound(_))), "{shown:?}");
    assert!(matches!(
        complete(&db, 2031, NOW, Source::Ui),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn the_check_is_due_from_the_first_of_november_until_a_run_in_november_or_december() {
    let cases: [(NaiveDate, Option<(i32, u32, u32)>, bool); 7] = [
        (day(2026, 10, 31), None, false),
        (day(2026, 11, 1), None, true),
        (day(2026, 12, 31), None, true),
        (day(2026, 11, 1), Some((2026, 11, 1)), false),
        (day(2026, 12, 31), Some((2026, 11, 14)), false),
        (day(2026, 11, 14), Some((2026, 6, 2)), true),
        (day(2026, 10, 31), Some((2026, 6, 2)), false),
    ];
    for (today, ran, expected) in cases {
        let (_dir, db) = asked_year();
        if let Some(on) = ran {
            run(&db, 2026, on);
        }

        assert_eq!(
            due_on(&db, 2026, today).unwrap(),
            expected,
            "{today} after {ran:?}"
        );
    }
}

#[test]
fn after_the_year_the_check_is_due_again_until_a_run_after_31_dec() {
    let cases: [(NaiveDate, Option<(i32, u32, u32)>, bool); 6] = [
        (day(2027, 1, 9), None, true),
        (day(2027, 1, 9), Some((2026, 11, 14)), true),
        (day(2027, 1, 9), Some((2026, 6, 2)), true),
        (day(2027, 1, 1), Some((2026, 12, 31)), true),
        (day(2027, 1, 9), Some((2027, 1, 9)), false),
        (day(2027, 1, 1), Some((2027, 1, 1)), false),
    ];
    for (today, ran, expected) in cases {
        let (_dir, db) = asked_year();
        if let Some(on) = ran {
            run(&db, 2026, on);
        }

        assert_eq!(
            due_on(&db, 2026, today).unwrap(),
            expected,
            "{today} after {ran:?}"
        );
    }
}

#[test]
fn a_closed_year_and_a_ruleset_without_ask_are_never_due() {
    let (_closed_dir, closed) = year_with(&asking(), YearState::Closed);
    let (_plain_dir, plain) = open_year();

    for today in [day(2026, 11, 14), day(2027, 1, 9)] {
        assert!(!due_on(&closed, 2026, today).unwrap(), "closed {today}");
        assert!(!due_on(&plain, 2026, today).unwrap(), "plain {today}");
    }
}

#[test]
fn due_years_lists_the_open_years_that_are_due_oldest_first() {
    let (_dir, db) = asked_year();
    for (year, state) in [(2025, YearState::Open), (2024, YearState::Closed)] {
        insert(
            &db,
            taxyears::TABLE,
            &TaxYearRow {
                state,
                closed_at: (state == YearState::Closed).then_some(2),
                ..TaxYearRow::new(year, format!("{year} v1"), rules_text(&asking_for(year)), 1)
            },
        );
    }

    assert_eq!(due_years(&db, day(2027, 1, 9)).unwrap(), [2025, 2026]);

    run(&db, 2026, (2027, 1, 9));
    assert_eq!(due_years(&db, day(2027, 1, 9)).unwrap(), [2025]);
    assert_eq!(due_years(&db, day(2026, 10, 31)).unwrap(), [2025]);
}

#[test]
fn close_needs_a_run_only_when_the_rules_ask_and_none_ran_after_31_dec() {
    let (_dir, db) = asked_year();
    assert!(close_needs_run(&db, 2026).unwrap());

    run(&db, 2026, (2026, 12, 31));
    assert!(close_needs_run(&db, 2026).unwrap());

    run(&db, 2026, (2027, 1, 9));
    assert!(!close_needs_run(&db, 2026).unwrap());

    let (_plain_dir, plain) = open_year();
    assert!(!close_needs_run(&plain, 2026).unwrap());
}

#[test]
fn completing_writes_one_op_with_the_year_in_its_label() {
    let (_dir, db) = asked_year();

    let written = run(&db, 2026, (2026, 11, 14));

    assert_eq!(written.label, "Complete Year Check 2026");
    assert_eq!(rows_stored(&db), 1);
    assert_eq!(last_run(&db, 2026).unwrap(), Some(day(2026, 11, 14)));
}

#[test]
fn a_second_completion_updates_the_same_row_and_the_view_shows_the_new_date() {
    let (_dir, db) = asked_year();
    run(&db, 2026, (2026, 6, 2));

    let written = run(&db, 2026, (2026, 11, 14));

    assert_eq!(written.label, "Complete Year Check 2026");
    assert_eq!(rows_stored(&db), 1);
    assert_eq!(
        view_on(&db, day(2026, 11, 14)).last_run.as_deref(),
        Some("14 Nov 2026")
    );
}

#[test]
fn a_closed_year_refuses_a_completion_and_stores_nothing() {
    let (_dir, db) = year_with(&asking(), YearState::Closed);

    let err = complete(&db, 2026, NOW, Source::Ui).unwrap_err();

    assert_eq!(
        err,
        AppError::InvalidInput("Tax year 2026 is closed; reopen it to run the year check".into())
    );
    assert_eq!(rows_stored(&db), 0);
}

#[test]
fn undoing_a_completion_restores_the_previous_run_and_then_none() {
    let (_dir, db) = asked_year();
    run(&db, 2026, (2026, 6, 2));
    run(&db, 2026, (2026, 11, 14));

    db.undo().unwrap().unwrap();
    assert_eq!(last_run(&db, 2026).unwrap(), Some(day(2026, 6, 2)));

    db.undo().unwrap().unwrap();
    assert_eq!(last_run(&db, 2026).unwrap(), None);
    assert_eq!(rows_stored(&db), 0);
}
