//! Rules files for tests: the smallest valid file for a year, as JSON to mutate and as file text.

use serde_json::{Value, json};

/// A valid rules file for `year`: one section, one documents item, no rates or questions.
pub fn minimal_rules(year: i32) -> Value {
    json!({
        "schemaVersion": 1,
        "ruleset": format!("{year} v1"),
        "year": year,
        "settings": { "gwgLimit": 1000.00, "halfYearFrom": "07-01" },
        "sections": [{ "id": "main", "name": "Main" }],
        "items": [{
            "key": "books",
            "kz": "720",
            "name": "Books",
            "section": "main",
            "kind": "documents"
        }]
    })
}

/// `minimal_rules(year)` with every kind of value Settings → Tax Rules can edit: a useful life, a Kilometergeld
/// rate, a question, and items with a private share and with estimate rules, one hidden item and one item
/// with nothing to edit.
pub fn rich_rules(year: i32) -> Value {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/usefulLife", json!({ "computer": 3 }));
    set(
        &mut rules,
        "/rates",
        json!({ "kilometergeld": [{
            "vehicle": "car", "perKm": 0.42,
            "from": format!("{year}-01-01"), "to": format!("{year}-12-31")
        }]}),
    );
    set(
        &mut rules,
        "/questions",
        json!([{ "id": "days", "label": "Days", "type": "integer", "unit": "days" }]),
    );
    set(&mut rules, "/items/0/privateShare", json!(20));
    set(
        &mut rules,
        "/items/0/estimate",
        json!([
            { "type": "cap", "amount": 300.00 },
            { "type": "requires", "question": "days", "min": 5 },
            { "type": "reduceByPerUnit", "question": "days", "perUnit": 3.00, "maxUnits": 100 }
        ]),
    );
    rules["items"].as_array_mut().expect("items").extend([
        json!({
            "key": "notes", "kz": "721", "name": "Notes", "section": "main", "kind": "documents",
            "visible": false, "privateShare": 50
        }),
        json!({
            "key": "plain", "kz": "722", "name": "Plain", "section": "main", "kind": "documents"
        }),
        json!({
            "key": "dues", "kz": "723", "name": "Dues", "section": "main", "kind": "amount",
            "estimate": [{ "type": "cap", "amount": 120.00, "carryOver": true }]
        }),
    ]);
    rules
}

/// Sets the value at a JSON pointer (`/settings/gwgLimit`); the path must exist, or end in a new object key.
pub fn set(rules: &mut Value, pointer: &str, value: Value) {
    let (parent, key) = pointer.rsplit_once('/').expect("a pointer with a key");
    let target = rules.pointer_mut(parent).expect("the parent exists");
    match target {
        Value::Object(map) => {
            map.insert(key.to_string(), value);
        }
        Value::Array(list) => list[key.parse::<usize>().expect("an index")] = value,
        other => panic!("can't set {pointer} in {other}"),
    }
}

/// The text of a rules file as Hamster writes it.
pub fn rules_text(rules: &Value) -> String {
    let mut text = serde_json::to_string_pretty(rules).expect("rules serialize");
    text.push('\n');
    text
}

/// Noon of a local day as millis, the way a completed year check is stamped.
pub fn local_millis(year: i32, month: u32, day: u32) -> i64 {
    use chrono::TimeZone;
    chrono::Local
        .with_ymd_and_hms(year, month, day, 12, 0, 0)
        .unwrap()
        .timestamp_millis()
}

/// `minimal_rules(year)` as file text.
pub fn minimal_text(year: i32) -> String {
    rules_text(&minimal_rules(year))
}
