//! What a Reload changes, one plain line per changed value, shown before it is applied.

use std::collections::BTreeSet;

use serde_json::Value;

use super::model::Ruleset;

/// Values compared one by one; `notes`, `sources` and `basedOn` don't change a result.
const PLAIN: &[&str] = &["ruleset", "settings", "usefulLife", "rates"];
/// Lists compared entry by entry under their id field.
const KEYED: &[(&str, &str)] = &[("sections", "id"), ("questions", "id"), ("items", "key")];

pub fn describe(old: &Ruleset, new: &Ruleset) -> Vec<String> {
    let (Ok(o), Ok(n)) = (serde_json::to_value(old), serde_json::to_value(new)) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for key in PLAIN {
        compare(key, o.get(*key), n.get(*key), &mut lines);
    }
    for (list, id) in KEYED {
        keyed(list, id, o.get(*list), n.get(*list), &mut lines);
    }
    if !lines.is_empty() && old.ruleset == new.ruleset {
        lines.push(format!(
            "The ruleset is still \"{}\"; bump it to tell the versions apart",
            new.ruleset
        ));
    }
    lines
}

fn compare(path: &str, old: Option<&Value>, new: Option<&Value>, lines: &mut Vec<String>) {
    match (old, new) {
        (Some(Value::Object(o)), Some(Value::Object(n))) => {
            let keys: BTreeSet<&String> = o.keys().chain(n.keys()).collect();
            for key in keys {
                compare(&format!("{path}.{key}"), o.get(key), n.get(key), lines);
            }
        }
        (Some(Value::Array(o)), Some(Value::Array(n))) => {
            for i in 0..o.len().max(n.len()) {
                compare(&format!("{path}[{i}]"), o.get(i), n.get(i), lines);
            }
        }
        (o, n) if o != n => lines.push(format!("{path}: {} → {}", show(o), show(n))),
        _ => {}
    }
}

fn keyed(list: &str, id: &str, old: Option<&Value>, new: Option<&Value>, lines: &mut Vec<String>) {
    let (o, n) = (by_id(old, id), by_id(new, id));
    for (key, value) in &o {
        match find(&n, key) {
            None => lines.push(format!("{list}: \"{key}\"{} removed", label(value))),
            Some(next) => compare(&format!("{list}.{key}"), Some(value), Some(next), lines),
        }
    }
    for (key, value) in &n {
        if find(&o, key).is_none() {
            lines.push(format!("{list}: \"{key}\"{} added", label(value)));
        }
    }
}

fn find<'a>(all: &[(&str, &'a Value)], key: &str) -> Option<&'a Value> {
    all.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn by_id<'a>(list: Option<&'a Value>, id: &str) -> Vec<(&'a str, &'a Value)> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|e| (e.get(id).and_then(Value::as_str).unwrap_or_default(), e))
        .collect()
}

/// ` (KZ 169 Digitale Arbeitsmittel)` for an item, the name or label of anything else.
fn label(entry: &Value) -> String {
    let text = |field: &str| entry.get(field).and_then(Value::as_str);
    match (text("kz"), text("name").or(text("label"))) {
        (Some(kz), Some(name)) => format!(" (KZ {kz} {name})"),
        (None, Some(name)) => format!(" ({name})"),
        _ => String::new(),
    }
}

fn show(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "none".into(),
        Some(Value::Number(n)) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && n.is_f64() => format!("{f:.0}"),
            _ => n.to_string(),
        },
        Some(v) => v.to_string(),
    }
}
