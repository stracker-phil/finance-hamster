//! The rules built into the app: one checked `rules-<year>.json` per year from `resources/rules/`, compiled in
//! (ADR 0051). A year starts from its built-in rules; edits made in Settings live in the year's row.

use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};

const BUNDLED: [(i32, &str); 2] = [
    (2025, include_str!("../../resources/rules/rules-2025.json")),
    (2026, include_str!("../../resources/rules/rules-2026.json")),
];

/// The built-in rules text for `year`, if the app has one.
pub fn bundled(year: i32) -> Option<&'static str> {
    BUNDLED
        .iter()
        .find(|(y, _)| *y == year)
        .map(|(_, text)| *text)
}

/// Every year the app has built-in rules for, oldest first.
pub fn bundled_years() -> impl Iterator<Item = i32> {
    BUNDLED.iter().map(|(year, _)| *year)
}

/// Next year's draft from an open year's file: its year, `<year> v1`, `basedOn` the old ruleset and the
/// Kilometergeld ranges moved by one year. Everything else is copied as it is.
pub fn next_year_text(text: &str) -> AppResult<String> {
    let Value::Object(old) = serde_json::from_str::<Value>(text)? else {
        return Err(AppError::InvalidInput(crate::tr!(
            "The rules file isn't an object",
            "Die Regeldatei ist kein Objekt"
        )));
    };
    let year = old.get("year").and_then(Value::as_i64).ok_or_else(|| {
        AppError::InvalidInput(crate::tr!(
            "The rules file has no year",
            "Die Regeldatei hat kein Jahr"
        ))
    })?;
    let ruleset = old.get("ruleset").cloned().unwrap_or(Value::Null);
    let mut next = Map::new();
    for (key, value) in old {
        match key.as_str() {
            "basedOn" => continue,
            "year" => {
                next.insert(key, Value::from(year + 1));
                next.insert("basedOn".into(), ruleset.clone());
            }
            "ruleset" => {
                next.insert(key, Value::from(format!("{} v1", year + 1)));
            }
            "rates" => {
                next.insert(key, shift_rates(value));
            }
            _ => {
                next.insert(key, value);
            }
        }
    }
    let mut out = serde_json::to_string_pretty(&Value::Object(next))?;
    out.push('\n');
    Ok(out)
}

fn shift_rates(mut rates: Value) -> Value {
    let ranges = rates
        .get_mut("kilometergeld")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten();
    for range in ranges {
        for field in ["from", "to"] {
            if let Some(Value::String(day)) = range.get_mut(field)
                && let Some(next) = next_year_day(day)
            {
                *day = next;
            }
        }
    }
    rates
}

fn next_year_day(day: &str) -> Option<String> {
    use chrono::{Datelike, NaiveDate};
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    // 29 February has no counterpart in the next year; the range then ends a day earlier.
    let next = date
        .with_year(date.year() + 1)
        .or_else(|| NaiveDate::from_ymd_opt(date.year() + 1, date.month(), 28))?;
    Some(next.format("%Y-%m-%d").to_string())
}
