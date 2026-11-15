use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::check::read;
use super::model::{Evidence, Ruleset};
use crate::test_support::rules::{minimal_rules, set};

const SEED: &str = include_str!("../../resources/rules/rules-2026.json");
const SEED_2025: &str = include_str!("../../resources/rules/rules-2025.json");

fn no_keys() -> BTreeSet<String> {
    BTreeSet::new()
}

fn edited(edits: &[(&str, Value)]) -> Value {
    let mut rules = minimal_rules(2026);
    for (pointer, value) in edits {
        set(&mut rules, pointer, value.clone());
    }
    rules
}

fn read_value(rules: &Value) -> Result<Ruleset, Vec<String>> {
    read(&rules.to_string(), &no_keys())
}

fn problems_of(edits: &[(&str, Value)]) -> Vec<String> {
    read_value(&edited(edits)).unwrap_err()
}

fn car_rate(from: &str, to: &str) -> Value {
    json!({ "vehicle": "car", "perKm": 0.5, "from": from, "to": to })
}

fn rates(list: Vec<Value>) -> Value {
    json!({ "kilometergeld": list })
}

#[test]
fn a_minimal_file_reads_back_as_typed_rules() {
    let ruleset = read_value(&minimal_rules(2026)).unwrap();
    assert_eq!(ruleset.year, 2026);
    assert_eq!(ruleset.ruleset, "2026 v1");
    assert_eq!(ruleset.items.len(), 1);
    assert_eq!(ruleset.items[0].evidence(), Evidence::Required);
}

#[test]
fn the_shipped_seed_is_valid_with_twelve_items_all_verified() {
    let ruleset = read(SEED, &no_keys()).unwrap();
    assert_eq!(ruleset.year, 2026);
    assert_eq!(ruleset.ruleset, "2026 v4");
    assert_eq!(ruleset.items.len(), 12);
    assert_eq!(ruleset.unverified_count(), 0);
}

#[test]
fn the_shipped_2025_seed_is_valid_with_twelve_items_all_verified() {
    let ruleset = read(SEED_2025, &no_keys()).unwrap();
    assert_eq!(ruleset.year, 2025);
    assert_eq!(ruleset.ruleset, "2025 v3");
    assert_eq!(ruleset.items.len(), 12);
    assert_eq!(ruleset.unverified_count(), 0);
}

#[test]
fn both_shipped_seeds_carry_the_same_items_and_kennzahlen() {
    let keys_and_kz = |text: &str| -> Vec<(String, String)> {
        let rules: Value = serde_json::from_str(text).unwrap();
        rules["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| (i["key"].to_string(), i["kz"].to_string()))
            .collect()
    };

    assert_eq!(keys_and_kz(SEED_2025), keys_and_kz(SEED));
}

#[test]
fn both_shipped_seeds_label_every_visible_item_in_english_and_german_and_ask_nothing() {
    for (name, text) in [("2025", SEED_2025), ("2026", SEED)] {
        let ruleset = read(text, &no_keys()).unwrap();
        for item in ruleset.items.iter().filter(|i| i.visible) {
            assert!(
                item.label.as_deref().is_some_and(|l| !l.trim().is_empty()),
                "{name} label of {}",
                item.key
            );
            let german = item.de.as_ref().and_then(|d| d.label.as_deref());
            assert!(
                german.is_some_and(|l| !l.trim().is_empty()),
                "{name} German label of {}",
                item.key
            );
        }
        assert!(
            ruleset.items.iter().all(|i| i.ask.is_none()),
            "{name} items"
        );
        assert!(
            ruleset.questions.iter().all(|q| q.ask.is_none()),
            "{name} questions"
        );
        assert!(
            ruleset
                .questions
                .iter()
                .all(|q| q.id != "telearbeitspauschale"),
            "{name} questions"
        );
    }
}

fn days_and_paid() -> Value {
    json!([
        { "id": "days", "label": "Days", "type": "integer" },
        { "id": "paid", "label": "Paid", "type": "amount" }
    ])
}

fn per_unit(question: &str) -> Value {
    json!([{ "type": "reduceByPerUnit", "question": question, "perUnit": 3.0, "maxUnits": 100 }])
}

#[test]
fn a_per_unit_reduction_must_name_an_existing_question() {
    assert_eq!(
        problems_of(&[("/items/0/estimate", per_unit("days"))]),
        vec!["items[0] (books): the question \"days\" doesn't exist".to_string()]
    );
}

#[test]
fn a_per_unit_reduction_needs_an_integer_question() {
    assert_eq!(
        problems_of(&[
            ("/questions", days_and_paid()),
            ("/items/0/estimate", per_unit("paid"))
        ]),
        vec![
            "items[0] (books): reduceByPerUnit needs an integer question, \"paid\" is an amount"
                .to_string()
        ]
    );
    assert!(
        read_value(&edited(&[
            ("/questions", days_and_paid()),
            ("/items/0/estimate", per_unit("days"))
        ]))
        .is_ok()
    );
}

#[test]
fn a_per_unit_reduction_problem_reads_in_german_when_the_language_is_german() {
    let problems = crate::i18n::with_lang(crate::settings::Language::De, || {
        problems_of(&[
            ("/questions", days_and_paid()),
            ("/items/0/estimate", per_unit("paid")),
        ])
    });
    assert_eq!(
        problems,
        vec![
            "items[0] (books): reduceByPerUnit braucht eine Ganzzahlfrage, „paid“ ist ein Betrag"
                .to_string()
        ]
    );
}

#[test]
fn an_empty_german_item_label_or_examples_text_is_reported_with_its_field() {
    for field in ["label", "examples"] {
        assert_eq!(
            problems_of(&[("/items/0/de", json!({ field: "  " }))]),
            vec![format!(
                "items[0].de.{field}: the German text is empty; remove it to show the English one"
            )],
            "{field}"
        );
    }
}

fn ask(order: u32, question: &str) -> Value {
    json!({ "order": order, "question": question })
}

#[test]
fn an_ask_on_an_item_and_on_a_question_reads_with_its_order_question_and_hint() {
    let ruleset = read_value(&edited(&[
        (
            "/questions",
            json!([{
                "id": "days", "label": "Days", "type": "integer",
                "ask": { "order": 2, "question": "How many days?", "hint": "See the calendar" }
            }]),
        ),
        ("/items/0/ask", ask(1, "Did you buy books?")),
    ]))
    .unwrap();

    let item = ruleset.items[0].ask.as_ref().unwrap();
    assert_eq!(
        (item.order, item.question.as_str(), item.hint.as_deref()),
        (1, "Did you buy books?", None)
    );
    let question = ruleset.questions[0].ask.as_ref().unwrap();
    assert_eq!(
        (
            question.order,
            question.question.as_str(),
            question.hint.as_deref()
        ),
        (2, "How many days?", Some("See the calendar"))
    );
}

#[test]
fn an_empty_ask_question_is_reported_where_it_is() {
    let questions = json!([{
        "id": "days", "label": "Days", "type": "integer",
        "ask": { "order": 1, "question": "  " }
    }]);
    assert_eq!(
        problems_of(&[("/items/0/ask", ask(1, ""))]),
        vec!["items[0].ask.question: the question is empty".to_string()]
    );
    assert_eq!(
        problems_of(&[("/questions", questions)]),
        vec!["questions[0].ask.question: the question is empty".to_string()]
    );
}

#[test]
fn a_hidden_item_or_one_reported_by_others_cannot_be_asked_about() {
    assert_eq!(
        problems_of(&[
            ("/items/0/visible", json!(false)),
            ("/items/0/ask", ask(1, "Books?")),
        ]),
        vec!["items[0].ask: a hidden item can't be asked about; show it or remove ask".to_string()]
    );
    assert_eq!(
        problems_of(&[
            ("/items/0/kind", json!("reportedByOthers")),
            ("/items/0/ask", ask(1, "Books?")),
        ]),
        vec!["items[0].ask: an item reported by others has nothing to add".to_string()]
    );
}

#[test]
fn an_ask_with_an_unknown_field_fails_to_parse() {
    let problems = problems_of(&[(
        "/items/0/ask",
        json!({ "order": 1, "question": "Books?", "colour": "red" }),
    )]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("unknown field `colour`"),
        "{problems:?}"
    );
}

#[test]
fn a_ruleset_with_ask_round_trips_and_one_without_serializes_no_ask_key() {
    let with = read_value(&edited(&[("/items/0/ask", ask(3, "Books?"))])).unwrap();
    let copy: Ruleset = serde_json::from_str(&serde_json::to_string(&with).unwrap()).unwrap();
    assert_eq!(copy, with);
    assert_eq!(
        serde_json::to_value(&with).unwrap()["items"][0]["ask"],
        json!({ "order": 3, "question": "Books?", "hint": null })
    );

    let without = read_value(&minimal_rules(2026)).unwrap();
    let value = serde_json::to_value(&without).unwrap();
    assert!(value["items"][0].get("ask").is_none());
}

#[test]
fn an_empty_german_text_is_reported_with_the_field_it_is_in() {
    let questions = json!([{
        "id": "days", "label": "Days", "type": "integer",
        "de": { "label": "", "help": "Tage" },
        "ask": { "order": 1, "question": "How many?", "de": { "question": "Wie viele?", "hint": "  " } }
    }]);
    let cases: Vec<(Vec<(&str, Value)>, &str)> = vec![
        (
            vec![("/questions", questions)],
            "questions[0].de.label: the German text is empty; remove it to show the English one",
        ),
        (
            vec![("/sections/0/de", json!({ "name": " " }))],
            "sections[0].de.name: the German text is empty; remove it to show the English one",
        ),
        (
            vec![("/items/0/de", json!({ "name": "Bücher", "note": "" }))],
            "items[0].de.note: the German text is empty; remove it to show the English one",
        ),
        (
            vec![(
                "/items/0/ask",
                json!({ "order": 1, "question": "Books?", "de": { "question": "" } }),
            )],
            "items[0].ask.de.question: the German text is empty; remove it to show the English one",
        ),
    ];
    for (edits, expected) in cases {
        let problems = problems_of(&edits);
        assert!(
            problems.contains(&expected.to_string()),
            "{expected}: {problems:?}"
        );
    }
    assert_eq!(
        problems_of(&[(
            "/questions",
            json!([{
                "id": "days", "label": "Days", "type": "integer",
                "ask": { "order": 1, "question": "How many?", "de": { "hint": "" } }
            }])
        )]),
        vec![
            "questions[0].ask.de.hint: the German text is empty; remove it to show the English one"
                .to_string()
        ]
    );
}

#[test]
fn an_empty_german_text_is_reported_in_german_when_the_language_is_german() {
    let problems = crate::i18n::with_lang(crate::settings::Language::De, || {
        problems_of(&[("/items/0/de", json!({ "name": "" }))])
    });
    assert_eq!(
        problems,
        vec![
            "items[0].de.name: Der deutsche Text ist leer; entferne ihn, um den englischen zu zeigen"
                .to_string()
        ]
    );
}

#[test]
fn german_texts_that_say_something_are_valid() {
    let ruleset = read_value(&edited(&[
        ("/sections/0/de", json!({ "name": "Hauptteil" })),
        (
            "/items/0/de",
            json!({ "name": "Bücher", "note": "Fachbücher" }),
        ),
        (
            "/questions",
            json!([{
                "id": "days", "label": "Days", "type": "integer",
                "de": { "label": "Tage", "help": "Tage zu Hause", "unit": "Tage" }
            }]),
        ),
    ]))
    .unwrap();
    assert_eq!(
        ruleset.items[0].de.as_ref().unwrap().name.as_deref(),
        Some("Bücher")
    );
}

#[test]
fn a_german_object_with_an_unknown_field_fails_to_parse() {
    let cases = ["/items/0/de", "/sections/0/de", "/items/0/ask"];
    for pointer in cases {
        let value = if pointer == "/items/0/ask" {
            json!({ "order": 1, "question": "Books?", "de": { "colour": "rot" } })
        } else {
            json!({ "colour": "rot" })
        };
        let problems = problems_of(&[(pointer, value)]);
        assert_eq!(problems.len(), 1, "{pointer}: {problems:?}");
        assert!(
            problems[0].contains("unknown field `colour`"),
            "{pointer}: {problems:?}"
        );
    }
}

#[test]
fn every_shipped_seed_has_german_for_each_question_label_and_ask_question() {
    for (name, text) in [("2025", SEED_2025), ("2026", SEED)] {
        let ruleset = read(text, &no_keys()).unwrap();
        for question in &ruleset.questions {
            let label = question.de.as_ref().and_then(|d| d.label.as_deref());
            assert!(
                label.is_some_and(|t| !t.trim().is_empty()),
                "{name} label of {}",
                question.id
            );
            if let Some(ask) = &question.ask {
                let text = ask.de.as_ref().and_then(|d| d.question.as_deref());
                assert!(
                    text.is_some_and(|t| !t.trim().is_empty()),
                    "{name} ask of {}",
                    question.id
                );
            }
        }
        for item in &ruleset.items {
            if let Some(ask) = &item.ask {
                let text = ask.de.as_ref().and_then(|d| d.question.as_deref());
                assert!(
                    text.is_some_and(|t| !t.trim().is_empty()),
                    "{name} ask of {}",
                    item.key
                );
            }
        }
    }
}

#[test]
fn a_second_section_and_an_item_reported_by_others_are_valid_without_code_changes() {
    let mut rules: Value = serde_json::from_str(SEED).unwrap();
    rules["sections"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "extra", "name": "Extra" }));
    rules["items"].as_array_mut().unwrap().push(json!({
        "key": "paid-by-employer",
        "kz": "900",
        "name": "Paid by the employer",
        "section": "extra",
        "kind": "reportedByOthers"
    }));

    let ruleset = read_value(&rules).unwrap();
    assert_eq!(ruleset.items.len(), 13);
    let info = ruleset.items.last().unwrap();
    assert_eq!(info.section, "extra");
    assert_eq!(info.evidence(), Evidence::None);
}

#[test]
fn each_schema_constraint_is_reported_with_where_it_is() {
    let cases: Vec<(Vec<(&str, Value)>, String)> = vec![
        (
            vec![("/schemaVersion", json!(2))],
            "schemaVersion: this Hamster reads version 1, not 2".into(),
        ),
        (
            vec![("/ruleset", json!("2026-v1"))],
            "ruleset: \"2026-v1\" should read like \"2026 v1\"".into(),
        ),
        (
            vec![("/ruleset", json!("2026 v"))],
            "ruleset: \"2026 v\" should read like \"2026 v1\"".into(),
        ),
        (
            vec![("/ruleset", json!("26 v1"))],
            "ruleset: \"26 v1\" should read like \"2026 v1\"".into(),
        ),
        (
            vec![("/year", json!(1999))],
            "year: 1999 is outside 2000 to 2100".into(),
        ),
        (
            vec![("/year", json!(2101))],
            "year: 2101 is outside 2000 to 2100".into(),
        ),
        (
            vec![("/settings/halfYearFrom", json!("13-01"))],
            "settings.halfYearFrom: \"13-01\" isn't a MM-DD day".into(),
        ),
        (
            vec![("/settings/halfYearFrom", json!("7-1"))],
            "settings.halfYearFrom: \"7-1\" isn't a MM-DD day".into(),
        ),
        (
            vec![("/settings/halfYearFrom", json!("02-30"))],
            "settings.halfYearFrom: \"02-30\" isn't a MM-DD day".into(),
        ),
        (
            vec![("/usefulLife", json!({ "computer": 0 }))],
            "usefulLife.computer: 0 is outside 1 to 50 years".into(),
        ),
        (
            vec![("/usefulLife", json!({ "furniture": 51 }))],
            "usefulLife.furniture: 51 is outside 1 to 50 years".into(),
        ),
        (
            vec![("/rates", rates(vec![car_rate("2026-1-01", "2026-12-31")]))],
            "rates.kilometergeld[0].from: \"2026-1-01\" isn't a YYYY-MM-DD date".into(),
        ),
        (
            vec![("/rates", rates(vec![car_rate("2026-01-01", "2026-02-30")]))],
            "rates.kilometergeld[0].to: \"2026-02-30\" isn't a YYYY-MM-DD date".into(),
        ),
        (
            vec![(
                "/questions",
                json!([{ "id": "Days", "label": "Days", "type": "integer" }]),
            )],
            "questions[0].id: \"Days\" isn't a key".into(),
        ),
        (
            vec![
                ("/sections/0/id", json!("Main")),
                ("/items/0/section", json!("Main")),
            ],
            "sections[0].id: \"Main\" isn't a key".into(),
        ),
        (
            vec![("/items/0/key", json!("books_1"))],
            "items[0].key: \"books_1\" isn't a key".into(),
        ),
        (
            vec![("/items/0/key", json!("1books"))],
            "items[0].key: \"1books\" isn't a key".into(),
        ),
        (
            vec![("/items/0/privateShare", json!(101))],
            "items[0].privateShare: more than 100 percent".into(),
        ),
        (
            vec![("/items/0/kind", json!("trips"))],
            "items[0].vehicle: a trips item needs a vehicle".into(),
        ),
        (
            vec![
                ("/items/0/kind", json!("reportedByOthers")),
                ("/items/0/evidence", json!("required")),
            ],
            "items[0].evidence: an item reported by others takes no evidence".into(),
        ),
        (
            vec![("/items", json!([]))],
            "items: at least one item is needed".into(),
        ),
    ];
    for (edits, expected) in cases {
        let pointers: Vec<&str> = edits.iter().map(|(p, _)| *p).collect();
        assert_eq!(problems_of(&edits), vec![expected], "{pointers:?}");
    }
}

#[test]
fn a_kz_must_be_three_digits_or_slash_separated_groups_of_three() {
    for kz in ["72", "7200", "718-916", "7a7", "718/", ""] {
        assert_eq!(
            problems_of(&[("/items/0/kz", json!(kz))]),
            vec![format!(
                "items[0].kz: \"{kz}\" should read like \"717\" or \"718/916\""
            )],
            "{kz}"
        );
    }
    for kz in ["717", "718/916", "001"] {
        assert!(
            read_value(&edited(&[("/items/0/kz", json!(kz))])).is_ok(),
            "{kz}"
        );
    }
}

#[test]
fn no_sections_is_reported_together_with_the_items_that_lose_their_section() {
    assert_eq!(
        problems_of(&[("/sections", json!([]))]),
        vec![
            "sections: at least one section is needed".to_string(),
            "items[0] (books): the section \"main\" doesn't exist".to_string(),
        ]
    );
}

#[test]
fn problems_read_in_german_when_the_language_is_german() {
    let german = crate::i18n::with_lang(crate::settings::Language::De, || {
        problems_of(&[
            ("/schemaVersion", json!(2)),
            ("/ruleset", json!("2026-v1")),
            ("/year", json!(1999)),
        ])
    });

    assert_eq!(
        german,
        vec![
            "schemaVersion: Dieser Hamster liest Version 1, nicht 2".to_string(),
            "ruleset: „2026-v1“ sollte wie „2026 v1“ aussehen".to_string(),
            "year: 1999 liegt nicht zwischen 2000 und 2100".to_string(),
        ]
    );
}

#[test]
fn every_problem_is_reported_at_once() {
    let problems = problems_of(&[
        ("/schemaVersion", json!(3)),
        ("/year", json!(1900)),
        ("/items/0/kz", json!("x")),
    ]);
    assert_eq!(problems.len(), 3, "{problems:?}");
}

#[test]
fn boundary_values_are_accepted() {
    let valid: Vec<(&str, Vec<(&str, Value)>)> = vec![
        ("leap day", vec![("/settings/halfYearFrom", json!("02-29"))]),
        ("year 2000", vec![("/year", json!(2000))]),
        ("year 2100", vec![("/year", json!(2100))]),
        (
            "private share 100",
            vec![("/items/0/privateShare", json!(100))],
        ),
        ("private share 0", vec![("/items/0/privateShare", json!(0))]),
        (
            "useful life 1 and 50",
            vec![("/usefulLife", json!({ "computer": 1, "furniture": 50 }))],
        ),
        (
            "reported by others with evidence none",
            vec![
                ("/items/0/kind", json!("reportedByOthers")),
                ("/items/0/evidence", json!("none")),
            ],
        ),
    ];
    for (name, edits) in valid {
        assert!(read_value(&edited(&edits)).is_ok(), "{name}");
    }
}

#[test]
fn unknown_fields_and_unknown_types_fail_to_parse() {
    let cases: Vec<(Vec<(&str, Value)>, &str)> = vec![
        (vec![("/colour", json!("red"))], "unknown field `colour`"),
        (
            vec![("/items/0/colour", json!("red"))],
            "unknown field `colour`",
        ),
        (
            vec![("/settings/gwgLimmit", json!(1))],
            "unknown field `gwgLimmit`",
        ),
        (
            vec![(
                "/items/0/estimate",
                json!([{ "type": "floor", "amount": 1 }]),
            )],
            "unknown variant `floor`",
        ),
        (
            vec![("/items/0/kind", json!("bogus"))],
            "unknown variant `bogus`",
        ),
    ];
    for (edits, fragment) in cases {
        let problems = problems_of(&edits);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains(fragment), "{fragment}: {problems:?}");
    }
}

#[test]
fn a_missing_required_field_or_broken_json_fails_to_parse() {
    let mut rules = minimal_rules(2026);
    rules.as_object_mut().unwrap().remove("items");
    assert_eq!(read_value(&rules).unwrap_err().len(), 1);
    assert_eq!(read("{ not json", &no_keys()).unwrap_err().len(), 1);
}

#[test]
fn amounts_must_not_be_negative_or_have_more_than_two_decimals() {
    for amount in [json!(-1), json!(1000.001), json!(0.005)] {
        let problems = problems_of(&[("/settings/gwgLimit", amount.clone())]);
        assert_eq!(problems.len(), 1, "{amount}");
        assert!(
            problems[0].contains("isn't an amount of at least 0 with at most two decimals"),
            "{amount}: {problems:?}"
        );
    }
}

#[test]
fn a_repeated_item_key_is_refused() {
    let mut rules = minimal_rules(2026);
    let item = rules["items"][0].clone();
    rules["items"].as_array_mut().unwrap().push(item);
    assert_eq!(
        read_value(&rules).unwrap_err(),
        vec!["items: the key \"books\" is used twice".to_string()]
    );
}

#[test]
fn a_key_that_entries_use_cannot_go_missing() {
    let used: BTreeSet<String> = ["books".to_string(), "gone".to_string()].into();
    let err = read(&minimal_rules(2026).to_string(), &used).unwrap_err();
    assert_eq!(
        err,
        vec![
            "items: \"gone\" is missing, but entries use it; keys can't be removed or renamed"
                .to_string()
        ]
    );

    let present: BTreeSet<String> = ["books".to_string()].into();
    assert!(read(&minimal_rules(2026).to_string(), &present).is_ok());
}

#[test]
fn an_item_in_an_unknown_section_is_refused() {
    assert_eq!(
        problems_of(&[("/items/0/section", json!("other"))]),
        vec!["items[0] (books): the section \"other\" doesn't exist".to_string()]
    );
}

#[test]
fn estimate_rules_must_name_existing_questions() {
    for (kind, rule) in [
        (
            "requires",
            json!({ "type": "requires", "question": "days", "min": 26 }),
        ),
        (
            "reduceBy",
            json!({ "type": "reduceBy", "question": "days" }),
        ),
    ] {
        assert_eq!(
            problems_of(&[("/items/0/estimate", json!([rule]))]),
            vec!["items[0] (books): the question \"days\" doesn't exist".to_string()],
            "{kind}"
        );
    }
}

#[test]
fn reduce_by_needs_an_amount_question_while_requires_accepts_either() {
    let questions = json!([
        { "id": "days", "label": "Days", "type": "integer" },
        { "id": "paid", "label": "Paid", "type": "amount" }
    ]);
    let reduce_days = json!([{ "type": "reduceBy", "question": "days" }]);
    assert_eq!(
        problems_of(&[
            ("/questions", questions.clone()),
            ("/items/0/estimate", reduce_days)
        ]),
        vec![
            "items[0] (books): reduceBy needs an amount question, \"days\" is an integer"
                .to_string()
        ]
    );

    let fine = json!([
        { "type": "reduceBy", "question": "paid" },
        { "type": "requires", "question": "days", "min": 26 },
        { "type": "requires", "question": "paid", "min": 1.5 },
        { "type": "cap", "amount": 300.00, "carryOver": true }
    ]);
    assert!(
        read_value(&edited(&[
            ("/questions", questions),
            ("/items/0/estimate", fine)
        ]))
        .is_ok()
    );
}

#[test]
fn a_depreciation_asset_class_needs_a_useful_life() {
    let depreciation = json!({ "assetClass": "computer" });
    assert_eq!(
        problems_of(&[("/items/0/depreciation", depreciation.clone())]),
        vec!["items[0] (books): the asset class \"computer\" has no usefulLife".to_string()]
    );
    assert!(
        read_value(&edited(&[
            ("/items/0/depreciation", depreciation),
            ("/usefulLife", json!({ "computer": 3 })),
        ]))
        .is_ok()
    );
}

#[test]
fn kilometergeld_ranges_must_run_forward_and_stay_inside_the_year() {
    assert_eq!(
        problems_of(&[("/rates", rates(vec![car_rate("2026-06-30", "2026-06-01")]))]),
        vec!["rates.kilometergeld[0]: from is after to".to_string()]
    );
    assert_eq!(
        problems_of(&[("/rates", rates(vec![car_rate("2025-12-31", "2026-12-31")]))]),
        vec!["rates.kilometergeld[0]: the range falls outside 2026".to_string()]
    );
    assert_eq!(
        problems_of(&[("/rates", rates(vec![car_rate("2026-01-01", "2027-01-01")]))]),
        vec!["rates.kilometergeld[0]: the range falls outside 2026".to_string()]
    );
}

#[test]
fn kilometergeld_ranges_of_one_vehicle_must_not_overlap_but_other_vehicles_may_share_days() {
    let overlapping = rates(vec![
        car_rate("2026-01-01", "2026-06-30"),
        car_rate("2026-06-30", "2026-12-31"),
    ]);
    assert_eq!(
        problems_of(&[("/rates", overlapping)]),
        vec!["rates.kilometergeld[0] and [1]: the ranges overlap".to_string()]
    );

    let adjacent = rates(vec![
        car_rate("2026-01-01", "2026-06-30"),
        car_rate("2026-07-01", "2026-12-31"),
    ]);
    assert!(read_value(&edited(&[("/rates", adjacent)])).is_ok());

    let other_vehicle = rates(vec![
        car_rate("2026-01-01", "2026-12-31"),
        json!({ "vehicle": "motorbike", "perKm": 0.24, "from": "2026-01-01", "to": "2026-12-31" }),
    ]);
    assert!(read_value(&edited(&[("/rates", other_vehicle)])).is_ok());
}

#[test]
fn a_trips_item_needs_a_kilometergeld_rate_for_its_vehicle() {
    let trips = [
        ("/items/0/kind", json!("trips")),
        ("/items/0/vehicle", json!("motorbike")),
    ];
    let mut edits = trips.to_vec();
    edits.push(("/rates", rates(vec![car_rate("2026-01-01", "2026-12-31")])));
    assert_eq!(
        problems_of(&edits),
        vec![
            "items[0] (books): there is no kilometergeld rate for the vehicle \"motorbike\""
                .to_string()
        ]
    );

    let mut with_rate = trips.to_vec();
    with_rate.push((
        "/rates",
        rates(vec![
            json!({ "vehicle": "motorbike", "perKm": 0.24, "from": "2026-01-01", "to": "2026-12-31" }),
        ]),
    ));
    assert!(read_value(&edited(&with_rate)).is_ok());
}
