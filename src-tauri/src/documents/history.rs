//! A document's History and a tax year's Log, read from the journal: what was added, changed, linked,
//! unlinked, finished, unlocked, excluded, restored or deleted, and when. Written once, never edited.

use std::collections::{BTreeMap, HashMap};

use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::lifecycle;
use super::store::{self, DocumentRow};
use super::title_of;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::i18n::Text;
use crate::money::{Money, format_money};
use crate::review::title_of as transaction_title;
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::tr;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum HistoryKind {
    Added,
    Changed,
    Linked,
    Unlinked,
    Finished,
    Unlocked,
    Excluded,
    Restored,
    Deleted,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct HistoryEntry {
    pub kind: HistoryKind,
    /// `Vendor: Pixel Print → Pixelforge Ltd.`.
    pub text: String,
    /// `13 Nov 2026, 14:32`.
    pub when: String,
    /// The document it is about, for the year's Log.
    pub document: String,
    pub document_id: String,
}

/// One journal row about a document or one of its links.
struct Row {
    op: i64,
    at: i64,
    entity: String,
    field: String,
    old: Option<Value>,
    new: Option<Value>,
}

/// Every journal row about `doc_id`: its own fields and those of its links, oldest first.
fn rows_of(conn: &Connection, doc_id: &str) -> AppResult<Vec<Row>> {
    let quoted = Value::String(doc_id.to_string()).to_string();
    let mut query = conn.prepare(
        "SELECT op, at, entity, field, old, new FROM journal
         WHERE (entity = ?1 AND entity_id = ?2)
            OR (entity = ?3 AND entity_id IN (
                SELECT entity_id FROM journal WHERE entity = ?3 AND field = 'document_id'
                AND (new = ?4 OR old = ?4)))
         ORDER BY seq",
    )?;
    let parse = |text: Option<String>| text.and_then(|t| serde_json::from_str(&t).ok());
    let rows = query.query_map(
        rusqlite::params![store::DOCUMENTS, doc_id, store::LINKS, quoted],
        |row| {
            Ok(Row {
                op: row.get(0)?,
                at: row.get(1)?,
                entity: row.get(2)?,
                field: row.get(3)?,
                old: parse(row.get(4)?),
                new: parse(row.get(5)?),
            })
        },
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Names for the ids journal rows hold, and how values read.
struct Context {
    vendors: HashMap<String, String>,
    /// Item labels by key, from every tax year's rules (keys stay the same across years).
    items: HashMap<String, String>,
    lines: HashMap<String, String>,
    format: NumberFormat,
    order: DateOrder,
}

impl Context {
    fn value(&self, field: &str, value: Option<&Value>) -> String {
        let Some(value) = value.filter(|v| !v.is_null() && v.as_str() != Some("")) else {
            return "—".into();
        };
        match (field, value) {
            ("vendor_id", Value::String(id)) => self
                .vendors
                .get(id)
                .cloned()
                .unwrap_or_else(|| tr!("a vendor", "ein Händler")),
            ("pay_date" | "invoice_date", Value::String(text)) => date::parse(text)
                .map(|d| date::day_month_year(d, self.order))
                .unwrap_or_else(|| text.clone()),
            ("paid_cents" | "invoice_cents", Value::Number(n)) => n
                .as_i64()
                .map(|c| format_money(&Money::eur(c), self.format).replace('€', ""))
                .unwrap_or_default(),
            ("item_key", Value::String(key)) => {
                self.items.get(key).cloned().unwrap_or_else(|| key.clone())
            }
            ("work_share", Value::Number(n)) => format!("{n} %"),
            (_, Value::String(text)) => text.clone(),
            (_, other) => other.to_string(),
        }
    }
}

const FIELD_NAMES: [(&str, Text); 9] = [
    ("pay_date", Text::new("Payment date", "Zahlungsdatum")),
    ("paid_cents", Text::new("Paid", "Bezahlt")),
    ("vendor_id", Text::new("Vendor", "Händler")),
    ("item_key", Text::new("Item", "Posten")),
    ("work_share", Text::new("Work share", "Beruflicher Anteil")),
    ("note", Text::new("Note", "Notiz")),
    (
        "invoice_cents",
        Text::new("Invoice amount", "Rechnungsbetrag"),
    ),
    (
        "invoice_currency",
        Text::new("Invoice currency", "Rechnungswährung"),
    ),
    ("invoice_date", Text::new("Invoice date", "Rechnungsdatum")),
];

fn document_entries(
    rows: &[&Row],
    deleted_reason: Option<&str>,
    cx: &Context,
) -> Vec<(HistoryKind, String)> {
    let field = |name: &str| rows.iter().copied().find(|r| r.field == name);
    if let Some(id) = field("id") {
        return match (&id.old, &id.new) {
            (None, _) => vec![(HistoryKind::Added, tr!("Added", "Hinzugefügt"))],
            (_, None) => vec![(
                HistoryKind::Deleted,
                deleted_reason.map_or_else(
                    || tr!("Deleted", "Gelöscht"),
                    |r| tr!("Deleted: {r}", "Gelöscht: {r}"),
                ),
            )],
            _ => Vec::new(),
        };
    }
    let mut out = Vec::new();
    if let Some(state) = field("state") {
        let text = |v: &Option<Value>| {
            v.as_ref()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let (from, to) = (text(&state.old), text(&state.new));
        let reason = field("exclude_reason")
            .and_then(|r| r.new.as_ref().and_then(Value::as_str))
            .map(|r| format!(": {r}"))
            .unwrap_or_default();
        out.push(match (from.as_str(), to.as_str()) {
            (_, "finished") => (HistoryKind::Finished, tr!("Finished", "Fertig")),
            ("finished", "draft") => (HistoryKind::Unlocked, tr!("Unlocked", "Entsperrt")),
            (_, "excluded") => (
                HistoryKind::Excluded,
                tr!("Excluded{reason}", "Ausgeschlossen{reason}"),
            ),
            ("excluded", _) => (HistoryKind::Restored, tr!("Restored", "Wiederhergestellt")),
            _ => (
                HistoryKind::Changed,
                tr!("State: {from} → {to}", "Status: {from} → {to}"),
            ),
        });
    }
    for (name, label) in FIELD_NAMES {
        if let Some(r) = field(name) {
            out.push((
                HistoryKind::Changed,
                format!(
                    "{}: {} → {}",
                    label.get(),
                    cx.value(name, r.old.as_ref()),
                    cx.value(name, r.new.as_ref())
                ),
            ));
        }
    }
    out
}

fn link_entries(rows: &[&Row], cx: &Context) -> Vec<(HistoryKind, String)> {
    let line = |v: &Option<Value>| {
        v.as_ref()
            .and_then(Value::as_str)
            .and_then(|id| cx.lines.get(id).cloned())
            .unwrap_or_else(|| tr!("a bank line", "eine Buchung"))
    };
    rows.iter()
        .filter(|r| r.field == "transaction_id")
        .filter_map(|link| match (&link.old, &link.new) {
            (None, Some(_)) => Some((
                HistoryKind::Linked,
                tr!("Linked {}", "Verknüpft: {}", line(&link.new)),
            )),
            (Some(_), None) => Some((
                HistoryKind::Unlinked,
                tr!("Unlinked {}", "Verknüpfung gelöst: {}", line(&link.old)),
            )),
            _ => None,
        })
        .collect()
}

fn entries_of_op(
    rows: &[&Row],
    deleted_reason: Option<&str>,
    cx: &Context,
) -> Vec<(HistoryKind, String)> {
    let of = |entity: &str| -> Vec<&Row> {
        rows.iter()
            .copied()
            .filter(|r| r.entity == entity)
            .collect()
    };
    let mut out = document_entries(&of(store::DOCUMENTS), deleted_reason, cx);
    out.extend(link_entries(&of(store::LINKS), cx));
    out
}

fn context(db: &Db, format: NumberFormat, order: DateOrder) -> AppResult<Context> {
    let vendors = db
        .read(store::vendors)?
        .into_iter()
        .map(|v| (v.id, v.name))
        .collect();
    let lines = db
        .read(statements::all_transactions)?
        .into_iter()
        .map(|t| {
            let label = format!(
                "{} · {}",
                transaction_title(&t),
                format_money(&Money::eur(t.cents), format)
            );
            (t.id, label)
        })
        .collect();
    let mut items = HashMap::new();
    for row in db.read(crate::taxyears::rows)? {
        let Ok(ruleset) = serde_json::from_str::<crate::taxrules::model::Ruleset>(&row.rules)
        else {
            continue;
        };
        for item in &ruleset.items {
            items
                .entry(item.key.clone())
                .or_insert_with(|| item.display_label().to_string());
        }
    }
    Ok(Context {
        vendors,
        items,
        lines,
        format,
        order,
    })
}

fn when(at: i64, order: DateOrder) -> String {
    chrono::DateTime::from_timestamp_millis(at)
        .map(|t| {
            let local = t.with_timezone(&chrono::Local);
            format!(
                "{}, {}",
                date::day_month_year(local.date_naive(), order),
                local.format("%H:%M")
            )
        })
        .unwrap_or_default()
}

/// An entry with what orders it: when its op was written, then the op itself.
type Timed = ((i64, i64), HistoryEntry);

/// Newest op first; the entries of one op keep their order.
fn into_newest_first(mut entries: Vec<Timed>) -> Vec<HistoryEntry> {
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));
    entries.into_iter().map(|(_, e)| e).collect()
}

fn history_of(
    db: &Db,
    doc_id: &str,
    title: &str,
    deleted_reason: Option<&str>,
    cx: &Context,
) -> AppResult<Vec<Timed>> {
    let rows = db.read(|c| rows_of(c, doc_id))?;
    let mut by_op: BTreeMap<i64, Vec<&Row>> = BTreeMap::new();
    for row in &rows {
        by_op.entry(row.op).or_default().push(row);
    }
    let mut out = Vec::new();
    for (op, op_rows) in by_op {
        let at = op_rows.first().map_or(0, |r| r.at);
        for (kind, text) in entries_of_op(&op_rows, deleted_reason, cx) {
            out.push((
                (at, op),
                HistoryEntry {
                    kind,
                    text,
                    when: when(at, cx.order),
                    document: title.to_string(),
                    document_id: doc_id.to_string(),
                },
            ));
        }
    }
    Ok(out)
}

/// A document's History, newest first.
pub fn document_history(
    db: &Db,
    doc_id: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<HistoryEntry>> {
    let cx = context(db, format, order)?;
    let vendors = db.read(store::vendors)?;
    let title = match db.read(|c| store::document_by_id(c, doc_id))? {
        Some(doc) => title_of(&doc, &vendors),
        None => lifecycle::deletions(db)?
            .into_iter()
            .find(|d| d.document_id == doc_id)
            .map(|d| d.title)
            .unwrap_or_default(),
    };
    Ok(into_newest_first(history_of(
        db, doc_id, &title, None, &cx,
    )?))
}

fn year_of(doc: &DocumentRow) -> Option<i32> {
    doc.pay_date.as_deref()?.get(..4)?.parse().ok()
}

/// Every change to the documents paid in `year`, and those deleted from its folder, newest first.
/// `query` keeps the entries whose text or document contains it.
pub fn year_log(
    db: &Db,
    year: i32,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<HistoryEntry>> {
    let cx = context(db, format, order)?;
    let vendors = db.read(store::vendors)?;
    let mut entries = Vec::new();
    for doc in db.read(store::documents)? {
        if year_of(&doc) == Some(year) {
            entries.extend(history_of(
                db,
                &doc.id,
                &title_of(&doc, &vendors),
                None,
                &cx,
            )?);
        }
    }
    let folder = format!("{}/{year}/", super::files::FOLDER);
    for gone in lifecycle::deletions(db)? {
        if gone.file.starts_with(&folder) {
            entries.extend(history_of(
                db,
                &gone.document_id,
                &gone.title,
                Some(&gone.reason),
                &cx,
            )?);
        }
    }
    let needle = query.trim().to_lowercase();
    entries.retain(|(_, e)| {
        needle.is_empty()
            || e.text.to_lowercase().contains(&needle)
            || e.document.to_lowercase().contains(&needle)
    });
    Ok(into_newest_first(entries))
}
