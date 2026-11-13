//! A change to one row, generic over the tables: rows travel as JSON objects keyed by column, so every change
//! has an exact inverse and a field-level journal entry (ADR 0007, ADR 0015).

use rusqlite::types::Value as SqlValue;
use rusqlite::{Transaction, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};

/// A row by column name; every table has a text `id` primary key.
pub type Row = Map<String, Value>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Insert {
        table: String,
        row: Row,
    },
    Update {
        table: String,
        before: Row,
        after: Row,
    },
    Delete {
        table: String,
        row: Row,
    },
}

/// One changed field, as the journal records it.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldChange {
    pub entity: String,
    pub entity_id: String,
    pub field: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

impl Change {
    /// A change from a serializable record: anything that serializes to a JSON object with an `id`.
    pub fn insert(table: &str, record: &impl Serialize) -> AppResult<Change> {
        Ok(Change::Insert {
            table: table.into(),
            row: to_row(record)?,
        })
    }

    pub fn update(
        table: &str,
        before: &impl Serialize,
        after: &impl Serialize,
    ) -> AppResult<Change> {
        Ok(Change::Update {
            table: table.into(),
            before: to_row(before)?,
            after: to_row(after)?,
        })
    }

    pub fn delete(table: &str, record: &impl Serialize) -> AppResult<Change> {
        Ok(Change::Delete {
            table: table.into(),
            row: to_row(record)?,
        })
    }

    pub fn table(&self) -> &str {
        match self {
            Change::Insert { table, .. }
            | Change::Update { table, .. }
            | Change::Delete { table, .. } => table,
        }
    }

    pub fn inverse(&self) -> Change {
        match self {
            Change::Insert { table, row } => Change::Delete {
                table: table.clone(),
                row: row.clone(),
            },
            Change::Delete { table, row } => Change::Insert {
                table: table.clone(),
                row: row.clone(),
            },
            Change::Update {
                table,
                before,
                after,
            } => Change::Update {
                table: table.clone(),
                before: after.clone(),
                after: before.clone(),
            },
        }
    }

    /// Every field the change sets, removes or alters; an update lists only the fields that differ.
    pub fn fields(&self) -> Vec<FieldChange> {
        let entry =
            |table: &str, row: &Row, field: &str, old: Option<&Value>, new: Option<&Value>| {
                FieldChange {
                    entity: table.to_string(),
                    entity_id: id_of(row).unwrap_or_default(),
                    field: field.to_string(),
                    old: old.cloned(),
                    new: new.cloned(),
                }
            };
        match self {
            Change::Insert { table, row } => row
                .iter()
                .map(|(k, v)| entry(table, row, k, None, Some(v)))
                .collect(),
            Change::Delete { table, row } => row
                .iter()
                .map(|(k, v)| entry(table, row, k, Some(v), None))
                .collect(),
            Change::Update {
                table,
                before,
                after,
            } => {
                let mut keys: Vec<&String> = before.keys().chain(after.keys()).collect();
                keys.sort();
                keys.dedup();
                keys.into_iter()
                    .filter(|k| before.get(*k) != after.get(*k))
                    .map(|k| entry(table, after, k, before.get(k), after.get(k)))
                    .collect()
            }
        }
    }

    /// Writes the change; an update or delete fails when no row has the id.
    pub(super) fn apply(&self, tx: &Transaction) -> AppResult<()> {
        match self {
            Change::Insert { table, row } => {
                check_name(table)?;
                let columns: Vec<&str> = row.keys().map(String::as_str).collect();
                columns.iter().try_for_each(|c| check_name(c))?;
                tx.execute(
                    &insert_statement(table, &columns),
                    params_from_iter(row.values().map(sql_value)),
                )?;
            }
            Change::Update {
                table,
                before,
                after,
            } => {
                check_name(table)?;
                let id = required_id(after)?;
                if id_of(before).as_deref() != Some(id.as_str()) {
                    return Err(AppError::Internal(format!(
                        "An update of {table} changes the id"
                    )));
                }
                let changed: Vec<(&String, &Value)> = after
                    .iter()
                    .filter(|(k, v)| before.get(*k) != Some(*v))
                    .collect();
                if changed.is_empty() {
                    return Ok(());
                }
                changed.iter().try_for_each(|(c, _)| check_name(c))?;
                let columns: Vec<&str> = changed.iter().map(|(c, _)| c.as_str()).collect();
                let mut values: Vec<SqlValue> = changed.iter().map(|(_, v)| sql_value(v)).collect();
                values.push(SqlValue::Text(id.clone()));
                let count =
                    tx.execute(&update_statement(table, &columns), params_from_iter(values))?;
                expect_one(count, table, &id)?;
            }
            Change::Delete { table, row } => {
                check_name(table)?;
                let id = required_id(row)?;
                let count = tx.execute(&format!("DELETE FROM {table} WHERE id = ?"), [&id])?;
                expect_one(count, table, &id)?;
            }
        }
        Ok(())
    }
}

fn insert_statement(table: &str, columns: &[&str]) -> String {
    let marks = vec!["?"; columns.len()].join(", ");
    format!(
        "INSERT INTO {table} ({}) VALUES ({marks})",
        columns.join(", ")
    )
}

fn update_statement(table: &str, columns: &[&str]) -> String {
    let sets: Vec<String> = columns.iter().map(|c| format!("{c} = ?")).collect();
    format!("UPDATE {table} SET {} WHERE id = ?", sets.join(", "))
}

fn to_row(record: &impl Serialize) -> AppResult<Row> {
    match serde_json::to_value(record)? {
        Value::Object(row) => Ok(row),
        _ => Err(AppError::Internal(
            "A record must serialize to an object".into(),
        )),
    }
}

fn id_of(row: &Row) -> Option<String> {
    match row.get("id")? {
        Value::String(id) => Some(id.clone()),
        other => Some(other.to_string()),
    }
}

fn required_id(row: &Row) -> AppResult<String> {
    id_of(row).ok_or_else(|| AppError::Internal("A row has no id".into()))
}

fn expect_one(count: usize, table: &str, id: &str) -> AppResult<()> {
    if count == 1 {
        Ok(())
    } else {
        Err(AppError::NotFound(format!("No {table} row {id}")))
    }
}

/// Table and column names come from code, never from input; anything but a plain identifier is a bug.
fn check_name(name: &str) -> AppResult<()> {
    let plain = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit());
    if plain {
        Ok(())
    } else {
        Err(AppError::Internal(format!("Not a plain SQL name: {name}")))
    }
}

/// Booleans become integers and nested values JSON text.
fn sql_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(SqlValue::Integer)
            .unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or_default())),
        Value::String(s) => SqlValue::Text(s.clone()),
        nested => SqlValue::Text(nested.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn row(value: Value) -> Row {
        to_row(&value).unwrap()
    }

    fn field(entity_id: &str, field: &str, old: Option<Value>, new: Option<Value>) -> FieldChange {
        FieldChange {
            entity: "things".into(),
            entity_id: entity_id.into(),
            field: field.into(),
            old,
            new,
        }
    }

    #[test]
    fn constructors_turn_records_into_rows_of_their_table() {
        let record = json!({ "id": "a", "name": "Rent" });
        assert_eq!(
            Change::insert("things", &record).unwrap(),
            Change::Insert {
                table: "things".into(),
                row: row(record.clone())
            }
        );
        assert_eq!(
            Change::delete("things", &record).unwrap(),
            Change::Delete {
                table: "things".into(),
                row: row(record.clone())
            }
        );
        let after = json!({ "id": "a", "name": "Flat" });
        let update = Change::update("things", &record, &after).unwrap();
        assert_eq!(
            update,
            Change::Update {
                table: "things".into(),
                before: row(record),
                after: row(after)
            }
        );
        assert_eq!(update.table(), "things");
    }

    #[test]
    fn a_change_serializes_with_a_snake_case_kind_tag() {
        let change = Change::delete("things", &json!({ "id": "a" })).unwrap();
        assert_eq!(
            serde_json::to_value(&change).unwrap(),
            json!({ "kind": "delete", "table": "things", "row": { "id": "a" } })
        );
    }

    #[test]
    fn the_inverse_of_an_insert_is_a_delete_and_back() {
        let insert = Change::insert("things", &json!({ "id": "a" })).unwrap();
        let delete = Change::delete("things", &json!({ "id": "a" })).unwrap();
        assert_eq!(insert.inverse(), delete);
        assert_eq!(delete.inverse(), insert);
    }

    #[test]
    fn the_inverse_of_an_update_swaps_before_and_after() {
        let before = json!({ "id": "a", "name": "Rent" });
        let after = json!({ "id": "a", "name": "Flat" });
        assert_eq!(
            Change::update("things", &before, &after).unwrap().inverse(),
            Change::update("things", &after, &before).unwrap()
        );
    }

    #[test]
    fn an_insert_lists_every_field_with_no_old_value() {
        let change = Change::insert("things", &json!({ "id": "a", "name": "Rent" })).unwrap();
        assert_eq!(
            change.fields(),
            vec![
                field("a", "id", None, Some(json!("a"))),
                field("a", "name", None, Some(json!("Rent"))),
            ]
        );
    }

    #[test]
    fn a_delete_lists_every_field_with_no_new_value() {
        let change = Change::delete("things", &json!({ "id": "a", "name": "Rent" })).unwrap();
        assert_eq!(
            change.fields(),
            vec![
                field("a", "id", Some(json!("a")), None),
                field("a", "name", Some(json!("Rent")), None),
            ]
        );
    }

    #[test]
    fn an_update_lists_only_the_fields_that_differ() {
        let before = json!({ "id": "a", "name": "Rent", "cents": 100, "note": "x" });
        let after = json!({ "id": "a", "name": "Flat", "cents": 100, "extra": true });
        assert_eq!(
            Change::update("things", &before, &after).unwrap().fields(),
            vec![
                field("a", "extra", None, Some(json!(true))),
                field("a", "name", Some(json!("Rent")), Some(json!("Flat"))),
                field("a", "note", Some(json!("x")), None),
            ]
        );
    }

    #[test]
    fn an_update_that_changes_nothing_lists_no_fields() {
        let same = json!({ "id": "a", "name": "Rent" });
        assert_eq!(
            Change::update("things", &same, &same).unwrap().fields(),
            vec![]
        );
    }

    #[test]
    fn a_numeric_id_is_written_as_text_in_the_field_list() {
        let change = Change::insert("things", &json!({ "id": 7 })).unwrap();
        assert_eq!(
            change.fields(),
            vec![field("7", "id", None, Some(json!(7)))]
        );
    }
}
