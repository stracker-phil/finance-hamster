//! Editing a year's rules in Settings: one value at a time, or the whole JSON. Either way the year then keeps
//! the owner's version, named after the ruleset it started from.

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::view::{self, FieldKind, RulesField};
use super::{check, diff};
use crate::db::{Db, Written};
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::settings::NumberFormat;
use crate::taxyears::{self, RulesSource, TaxYearRow};

/// What saving an edited JSON would do.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct JsonPreview {
    pub problems: Vec<String>,
    pub changes: Vec<String>,
}

fn invalid(message: String) -> AppError {
    AppError::InvalidInput(message)
}

fn year_row(db: &Db, year: i32) -> AppResult<TaxYearRow> {
    db.read(|c| taxyears::row(c, year))?
        .ok_or_else(|| AppError::NotFound(crate::tr!("Tax year {year}", "Steuerjahr {year}")))
}

/// The year's rules as pretty JSON, for the editor.
pub fn json(db: &Db, year: i32) -> AppResult<String> {
    let value: Value = serde_json::from_str(&year_row(db, year)?.rules)?;
    Ok(serde_json::to_string_pretty(&value)? + "\n")
}

/// A first edit names the owner's version after the built-in one: `ruleset` gains " (own)", and `basedOn`
/// records the built-in ruleset the year started from.
fn mark_own(value: &mut Value, row: &TaxYearRow) {
    if row.rules_source == RulesSource::Own {
        return;
    }
    if let Value::Object(map) = value {
        map.insert("basedOn".into(), Value::from(row.ruleset.clone()));
        map.insert(
            "ruleset".into(),
            Value::from(format!("{} (own)", row.ruleset)),
        );
    }
}

fn item_index(value: &Value, key: &str) -> AppResult<usize> {
    value
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .position(|i| i.get("key").and_then(Value::as_str) == Some(key))
        })
        .ok_or_else(|| {
            invalid(crate::tr!(
                "There is no item \"{key}\"",
                "Es gibt keinen Posten „{key}“"
            ))
        })
}

/// The JSON pointer for a field path; `items/<key>/…` names the item by key, the pointer by index.
fn pointer(value: &Value, path: &str) -> AppResult<String> {
    match path
        .strip_prefix("items/")
        .and_then(|rest| rest.split_once('/'))
    {
        Some((key, rest)) => Ok(format!("/items/{}/{rest}", item_index(value, key)?)),
        None => Ok(format!("/{path}")),
    }
}

fn parse(kind: FieldKind, input: &str) -> AppResult<(Value, i64)> {
    let refuse = || {
        invalid(crate::tr!(
            "\"{input}\" isn't a valid value",
            "„{input}“ ist kein gültiger Wert"
        ))
    };
    match kind {
        FieldKind::Amount => {
            let cents = parse_cents(input).filter(|c| *c >= 0).ok_or_else(refuse)?;
            Ok((serde_json::json!(cents as f64 / 100.0), cents))
        }
        FieldKind::Count | FieldKind::Percent => {
            let n: i64 = input.trim().parse().map_err(|_| refuse())?;
            if n < 0 || (kind == FieldKind::Percent && n > 100) {
                return Err(refuse());
            }
            Ok((Value::from(n), n))
        }
    }
}

/// The editable field at `path` in the year's view; anything else can't be changed here.
fn field_at(row: &TaxYearRow, path: &str, format: NumberFormat) -> AppResult<RulesField> {
    let ruleset =
        check::read(&row.rules, &Default::default()).map_err(|p| invalid(p.join("; ")))?;
    view::year_view(row, &ruleset, format, crate::date::DateOrder::system())
        .groups
        .into_iter()
        .flat_map(|g| g.fields)
        .find(|f| f.path == path)
        .ok_or_else(|| {
            invalid(crate::tr!(
                "\"{path}\" can't be edited here",
                "„{path}“ lässt sich hier nicht ändern"
            ))
        })
}

/// Changes one value from the year's view (`field.path`), keeping everything else.
pub fn set_value(
    db: &Db,
    year: i32,
    path: &str,
    input: &str,
    format: NumberFormat,
    now: i64,
) -> AppResult<Written> {
    let row = year_row(db, year)?;
    let field = field_at(&row, path, format)?;
    let (new, number) = parse(field.kind, input)?;
    let mut value: Value = serde_json::from_str(&row.rules)?;
    let target = pointer(&value, path)?;
    *value.pointer_mut(&target).ok_or_else(|| {
        invalid(crate::tr!(
            "\"{path}\" isn't in the rules",
            "„{path}“ steht nicht in den Regeln"
        ))
    })? = new;
    mark_own(&mut value, &row);
    let shown = view::shown_value(field.kind, number, field.unit.as_deref(), format);
    let label = crate::tr!(
        "Change {} for {year} to {shown}",
        "{} für {year} auf {shown} ändern",
        field.label
    );
    super::set_own(db, year, serde_json::to_string_pretty(&value)?, label, now)
}

/// What saving `text` as the year's rules would change, or why it can't be saved.
pub fn preview_json(db: &Db, year: i32, text: &str) -> AppResult<JsonPreview> {
    let row = year_row(db, year)?;
    let used = db.read(super::used_keys)?;
    Ok(match check::read(text, &used) {
        Err(problems) => JsonPreview {
            problems,
            changes: Vec::new(),
        },
        Ok(next) => {
            let current = check::read(&row.rules, &Default::default()).ok();
            JsonPreview {
                problems: super::wrong_year(next.year, year).into_iter().collect(),
                changes: current
                    .map(|c| diff::describe(&c, &next))
                    .unwrap_or_default(),
            }
        }
    })
}

/// Saves `text` as the owner's version of the year's rules.
pub fn set_json(db: &Db, year: i32, text: &str, now: i64) -> AppResult<Written> {
    let row = year_row(db, year)?;
    let mut value: Value = serde_json::from_str(text).map_err(|e| invalid(e.to_string()))?;
    let unchanged_name = value.get("ruleset").and_then(Value::as_str) == Some(row.ruleset.as_str());
    if unchanged_name {
        mark_own(&mut value, &row);
    }
    super::set_own(
        db,
        year,
        serde_json::to_string_pretty(&value)?,
        crate::tr!(
            "Edit the Tax Rules for {year}",
            "Steuerregeln für {year} bearbeiten"
        ),
        now,
    )
}
