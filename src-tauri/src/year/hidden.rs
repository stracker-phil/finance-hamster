//! Categories the owner hides while recording a tax year (ADR 0053): one row per year and item. Only the
//! Categories view reads them; the FinanzOnline view and the pickers always show every item.

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::taxrules::model::{ItemKind, Ruleset};

pub const HIDDEN: &str = "hidden_items";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HiddenRow {
    /// `2026:union-dues`.
    pub id: String,
    pub year: i32,
    pub item_key: String,
}

impl HiddenRow {
    pub fn new(year: i32, item_key: &str) -> Self {
        Self {
            id: format!("{year}:{item_key}"),
            year,
            item_key: item_key.to_string(),
        }
    }
}

/// One category of the year with its switch, for the Categories sheet.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct CategoryChoice {
    pub key: String,
    pub label: String,
    pub examples: Option<String>,
    pub shown: bool,
}

/// The item keys hidden in `year`.
pub fn hidden(conn: &Connection, year: i32) -> AppResult<BTreeSet<String>> {
    let mut query = conn.prepare(&format!("SELECT item_key FROM {HIDDEN} WHERE year = ?"))?;
    let keys = query.query_map([year], |r| r.get::<_, String>(0))?;
    Ok(keys.collect::<Result<_, _>>()?)
}

/// Every category the year records, in the rules' order, with whether it is shown.
pub fn choices(ruleset: &Ruleset, hidden: &BTreeSet<String>) -> Vec<CategoryChoice> {
    ruleset
        .items
        .iter()
        .filter(|i| i.visible && i.kind != ItemKind::ReportedByOthers)
        .map(|i| CategoryChoice {
            key: i.key.clone(),
            label: i.display_label().to_string(),
            examples: i.display_examples().map(str::to_string),
            shown: !hidden.contains(&i.key),
        })
        .collect()
}

/// Shows or hides one category of `year`; None when it already is that way.
pub fn set_shown(
    db: &Db,
    ruleset: &Ruleset,
    year: i32,
    key: &str,
    shown: bool,
) -> AppResult<Option<Written>> {
    let item = ruleset.items.iter().find(|i| i.key == key).ok_or_else(|| {
        AppError::NotFound(crate::tr!(
            "No category {key} in {year}",
            "Keine Kategorie {key} in {year}"
        ))
    })?;
    let row = HiddenRow::new(year, key);
    let is_hidden = db.read(|c| hidden(c, year))?.contains(key);
    let (change, label) = match (shown, is_hidden) {
        (true, true) => (
            Change::delete(HIDDEN, &row)?,
            crate::tr!(
                "Show {} in {year}",
                "{} in {year} zeigen",
                item.display_label()
            ),
        ),
        (false, false) => (
            Change::insert(HIDDEN, &row)?,
            crate::tr!(
                "Hide {} in {year}",
                "{} in {year} ausblenden",
                item.display_label()
            ),
        ),
        _ => return Ok(None),
    };
    db.write(Op::new(label, vec![change]), Source::Ui).map(Some)
}

/// The rows that carry `from`'s hidden categories over to a new year `to`.
pub fn carried_over(conn: &Connection, from: i32, to: i32) -> AppResult<Vec<Change>> {
    hidden(conn, from)?
        .iter()
        .map(|key| Change::insert(HIDDEN, &HiddenRow::new(to, key)))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::date::DateOrder;
    use crate::settings::{Language, NumberFormat};
    use crate::taxyears::{self, TaxYearRow, YearState};
    use crate::test_support::rules::rules_text;
    use crate::year::page::page;
    use crate::year::tests::{open_year, rules, ruleset};

    fn keys(db: &Db, year: i32) -> Vec<String> {
        db.read(|c| hidden(c, year)).unwrap().into_iter().collect()
    }

    fn written(label: &str) -> Written {
        Written {
            label: label.into(),
            tables: vec![HIDDEN.into()],
        }
    }

    fn hide(db: &Db, year: i32, key: &str) {
        set_shown(db, &ruleset(), year, key, false).unwrap();
    }

    fn switches(ruleset: &Ruleset, hidden: &[&str]) -> Vec<(String, bool)> {
        let hidden = hidden.iter().map(|k| k.to_string()).collect();
        choices(ruleset, &hidden)
            .into_iter()
            .map(|c| (c.key, c.shown))
            .collect()
    }

    #[test]
    fn hiding_a_category_stores_it_for_that_year_only() {
        let (_dir, db) = open_year();

        let result = set_shown(&db, &ruleset(), 2026, "books", false).unwrap();

        assert_eq!(result, Some(written("Hide Books in 2026")));
        assert_eq!(keys(&db, 2026), ["books"]);
        assert!(keys(&db, 2025).is_empty());
    }

    #[test]
    fn showing_a_hidden_category_removes_it() {
        let (_dir, db) = open_year();
        hide(&db, 2026, "books");
        hide(&db, 2026, "phone");

        let result = set_shown(&db, &ruleset(), 2026, "books", true).unwrap();

        assert_eq!(result, Some(written("Show Books in 2026")));
        assert_eq!(keys(&db, 2026), ["phone"]);
    }

    #[test]
    fn setting_a_category_to_the_state_it_has_changes_nothing() {
        let (_dir, db) = open_year();
        assert_eq!(
            set_shown(&db, &ruleset(), 2026, "books", true).unwrap(),
            None
        );
        hide(&db, 2026, "books");

        assert_eq!(
            set_shown(&db, &ruleset(), 2026, "books", false).unwrap(),
            None
        );

        assert_eq!(keys(&db, 2026), ["books"]);
    }

    #[test]
    fn an_unknown_category_is_not_found() {
        let (_dir, db) = open_year();

        let err = set_shown(&db, &ruleset(), 2026, "nope", false).unwrap_err();

        assert_eq!(err, AppError::NotFound("No category nope in 2026".into()));
        assert!(keys(&db, 2026).is_empty());
    }

    #[test]
    fn the_switch_labels_follow_the_language() {
        let (_dir, db) = open_year();

        let hid = crate::i18n::with_lang(Language::De, || {
            set_shown(&db, &ruleset(), 2026, "books", false).unwrap()
        });
        let showed = crate::i18n::with_lang(Language::De, || {
            set_shown(&db, &ruleset(), 2026, "books", true).unwrap()
        });

        assert_eq!(hid, Some(written("Books in 2026 ausblenden")));
        assert_eq!(showed, Some(written("Books in 2026 zeigen")));
    }

    #[test]
    fn undoing_a_switch_restores_the_previous_choice() {
        let (_dir, db) = open_year();
        hide(&db, 2026, "books");
        db.undo().unwrap().unwrap();
        assert!(keys(&db, 2026).is_empty());

        hide(&db, 2026, "books");
        set_shown(&db, &ruleset(), 2026, "books", true).unwrap();
        db.undo().unwrap().unwrap();

        assert_eq!(keys(&db, 2026), ["books"]);
    }

    #[test]
    fn a_closed_year_still_takes_its_switches() {
        let (_dir, db) = open_year();
        let open = TaxYearRow::new(2026, "2026 v1".into(), rules_text(&rules()), 1);
        let closed = TaxYearRow {
            state: YearState::Closed,
            closed_at: Some(2),
            ..open.clone()
        };
        db.write(
            Op::new(
                "Close year",
                vec![Change::update(taxyears::TABLE, &open, &closed).unwrap()],
            ),
            Source::App,
        )
        .unwrap();

        let hid = set_shown(&db, &ruleset(), 2026, "books", false).unwrap();
        let showed = set_shown(&db, &ruleset(), 2026, "books", true).unwrap();

        assert_eq!(hid, Some(written("Hide Books in 2026")));
        assert_eq!(showed, Some(written("Show Books in 2026")));
        assert!(keys(&db, 2026).is_empty());
    }

    #[test]
    fn choices_list_the_visible_items_in_rules_order_without_those_reported_by_others() {
        assert_eq!(
            switches(&ruleset(), &[]),
            [
                ("books".to_string(), true),
                ("phone".to_string(), true),
                ("tips".to_string(), true),
                ("parking".to_string(), true),
            ]
        );
    }

    #[test]
    fn choices_switch_off_exactly_the_hidden_items() {
        assert_eq!(
            switches(&ruleset(), &["phone", "parking", "unknown"]),
            [
                ("books".to_string(), true),
                ("phone".to_string(), false),
                ("tips".to_string(), true),
                ("parking".to_string(), false),
            ]
        );
    }

    #[test]
    fn a_choice_reaches_the_page_in_camel_case_with_its_label_and_examples() {
        let mut rules = rules();
        rules["items"][0]["label"] = json!("Reading");
        rules["items"][0]["examples"] = json!("Novels, manuals");
        let ruleset: Ruleset = serde_json::from_value(rules).unwrap();

        let first = choices(&ruleset, &BTreeSet::from(["books".to_string()]))
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(
            serde_json::to_value(first).unwrap(),
            json!({
                "key": "books",
                "label": "Reading",
                "examples": "Novels, manuals",
                "shown": false
            })
        );
    }

    #[test]
    fn the_year_page_marks_only_the_hidden_item_of_that_year_as_not_shown() {
        let (_dir, db) = open_year();
        hide(&db, 2026, "books");
        hide(&db, 2025, "phone");

        let year_page = page(&db, 2026, "", NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap();

        let shown: Vec<(&str, bool)> = year_page
            .kennzahlen
            .iter()
            .map(|k| (k.key.as_str(), k.shown))
            .collect();
        assert_eq!(
            shown,
            [
                ("books", false),
                ("phone", true),
                ("tips", true),
                ("parking", true),
                ("employer", true),
            ]
        );
    }

    #[test]
    fn carrying_over_copies_the_hidden_items_into_the_new_year() {
        let (_dir, db) = open_year();
        hide(&db, 2026, "books");
        hide(&db, 2026, "tips");
        hide(&db, 2025, "phone");

        let changes = db.read(|c| carried_over(c, 2026, 2027)).unwrap();

        db.write(Op::new("Start", changes), Source::App).unwrap();
        assert_eq!(keys(&db, 2027), ["books", "tips"]);
        assert_eq!(keys(&db, 2026), ["books", "tips"]);
    }

    #[test]
    fn carrying_over_from_a_year_without_hidden_items_changes_nothing() {
        let (_dir, db) = open_year();

        assert!(db.read(|c| carried_over(c, 2026, 2027)).unwrap().is_empty());
    }
}
