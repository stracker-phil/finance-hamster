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
