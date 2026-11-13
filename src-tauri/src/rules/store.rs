//! The `rules` and `decisions` rows. A decision belongs to one statement line and shares its id; a decision
//! without a rule was made by hand.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;
use crate::statements::store::TRANSACTIONS;

pub const RULES: &str = "rules";
pub const DECISIONS: &str = "decisions";

/// What a rule or a hand decision does with a line.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Action {
    Ignore,
    /// Assigns a Werbungskosten item.
    Anv,
    Subscription,
}

impl Action {
    fn parse(text: &str) -> Action {
        match text {
            "anv" => Action::Anv,
            "subscription" => Action::Subscription,
            _ => Action::Ignore,
        }
    }
}

/// The rules that come with the app; they can be edited but not deleted.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    Transfers,
    BankFees,
}

impl Builtin {
    fn parse(text: &str) -> Option<Builtin> {
        match text {
            "transfers" => Some(Builtin::Transfers),
            "bank_fees" => Some(Builtin::BankFees),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuleRow {
    pub id: String,
    /// Lower runs first.
    pub position: i64,
    pub builtin: Option<Builtin>,
    /// The bank text tokens, any of which matches, as a JSON array.
    pub contains: String,
    /// Matched against the line's amount without its sign.
    pub amount_cents: Option<i64>,
    pub tolerance_cents: i64,
    pub account_id: Option<String>,
    pub own_counterparty: bool,
    pub action: Action,
    pub item_key: Option<String>,
    pub subscription_id: Option<String>,
    /// A recurring document, such as a rental contract, linked to every line the rule assigns.
    pub document_id: Option<String>,
    pub created_at: i64,
}

impl RuleRow {
    /// The non-blank tokens; a blank one would match every line.
    pub fn tokens(&self) -> Vec<String> {
        let tokens: Vec<String> = serde_json::from_str(&self.contains).unwrap_or_default();
        tokens
            .into_iter()
            .filter(|t| !t.trim().is_empty())
            .collect()
    }
}

/// How one statement line was decided; it shares the line's id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionRow {
    pub id: String,
    pub kind: Action,
    pub item_key: Option<String>,
    pub subscription_id: Option<String>,
    pub document_id: Option<String>,
    /// None when decided by hand.
    pub rule_id: Option<String>,
    pub decided_at: i64,
}

impl DecisionRow {
    pub fn by_hand(&self) -> bool {
        self.rule_id.is_none()
    }

    /// The same outcome, whenever it was decided.
    pub fn same_as(&self, other: &DecisionRow) -> bool {
        let mut this = self.clone();
        this.decided_at = other.decided_at;
        this == *other
    }
}

const RULE_COLUMNS: &str = "id, position, builtin, contains, amount_cents, tolerance_cents, account_id, \
     own_counterparty, action, item_key, subscription_id, document_id, created_at";

fn rule(row: &rusqlite::Row) -> rusqlite::Result<RuleRow> {
    let builtin: Option<String> = row.get(2)?;
    let action: String = row.get(8)?;
    Ok(RuleRow {
        id: row.get(0)?,
        position: row.get(1)?,
        builtin: builtin.as_deref().and_then(Builtin::parse),
        contains: row.get(3)?,
        amount_cents: row.get(4)?,
        tolerance_cents: row.get(5)?,
        account_id: row.get(6)?,
        own_counterparty: row.get(7)?,
        action: Action::parse(&action),
        item_key: row.get(9)?,
        subscription_id: row.get(10)?,
        document_id: row.get(11)?,
        created_at: row.get(12)?,
    })
}

/// Every rule in the order they run.
pub fn rules(conn: &Connection) -> AppResult<Vec<RuleRow>> {
    let sql = format!("SELECT {RULE_COLUMNS} FROM {RULES} ORDER BY position, created_at");
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], rule)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

const DECISION_COLUMNS: &str =
    "id, kind, item_key, subscription_id, document_id, rule_id, decided_at";

fn decision(row: &rusqlite::Row) -> rusqlite::Result<DecisionRow> {
    let kind: String = row.get(1)?;
    Ok(DecisionRow {
        id: row.get(0)?,
        kind: Action::parse(&kind),
        item_key: row.get(2)?,
        subscription_id: row.get(3)?,
        document_id: row.get(4)?,
        rule_id: row.get(5)?,
        decided_at: row.get(6)?,
    })
}

/// Every decision by line id.
pub fn decisions(conn: &Connection) -> AppResult<HashMap<String, DecisionRow>> {
    let sql = format!("SELECT {DECISION_COLUMNS} FROM {DECISIONS}");
    let mut query = conn.prepare(&sql)?;
    let rows = query.query_map([], decision)?;
    rows.map(|r| r.map(|d| (d.id.clone(), d)).map_err(Into::into))
        .collect()
}

/// How many lines a rule decided, and the newest one's value date.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Matches {
    pub count: u32,
    pub last: Option<String>,
}

pub fn matches_by_rule(conn: &Connection) -> AppResult<HashMap<String, Matches>> {
    let mut query = conn.prepare(&format!(
        "SELECT d.rule_id, COUNT(*), MAX(t.value_date) FROM {DECISIONS} d \
         JOIN {TRANSACTIONS} t ON t.id = d.id WHERE d.rule_id IS NOT NULL GROUP BY d.rule_id"
    ))?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Matches {
                count: row.get(1)?,
                last: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::{
        add, decide_by_hand, ignore, import_statement, insert_rule, last_rule, rule_row, rules_of,
        setup,
    };
    use crate::statements::import_tests::october;
    use serde_json::json;

    fn decided(kind: Action) -> DecisionRow {
        DecisionRow {
            id: "line-1".into(),
            kind,
            item_key: None,
            subscription_id: None,
            document_id: None,
            rule_id: Some("rule-1".into()),
            decided_at: 1,
        }
    }

    #[test]
    fn rules_run_by_position_and_ties_go_to_the_older_rule() {
        let (_dir, db) = setup();
        for (id, position, created_at) in [("c", 5, 1), ("a", 0, 2), ("b", 0, 1)] {
            insert_rule(
                &db,
                &RuleRow {
                    created_at,
                    ..rule_row(id, position, &["x"], Action::Ignore)
                },
            );
        }

        let ids: Vec<String> = rules_of(&db).into_iter().map(|r| r.id).collect();

        assert_eq!(ids, ["b", "a", "c"]);
    }

    #[test]
    fn a_rule_row_round_trips_with_every_field() {
        let (_dir, db) = setup();
        let full = RuleRow {
            id: "rule-full".into(),
            position: -1,
            builtin: Some(Builtin::BankFees),
            contains: r#"["Kontoführung"]"#.into(),
            amount_cents: Some(2_595),
            tolerance_cents: 100,
            account_id: Some("account-1".into()),
            own_counterparty: true,
            action: Action::Anv,
            item_key: Some("books".into()),
            subscription_id: Some("sub-1".into()),
            document_id: Some("doc-1".into()),
            created_at: 42,
        };
        let bare = RuleRow {
            id: "rule-bare".into(),
            position: 3,
            builtin: None,
            amount_cents: None,
            tolerance_cents: 0,
            account_id: None,
            own_counterparty: false,
            action: Action::Subscription,
            item_key: None,
            subscription_id: None,
            document_id: None,
            ..full.clone()
        };
        insert_rule(&db, &full);
        insert_rule(&db, &bare);

        assert_eq!(rules_of(&db), vec![full, bare]);
    }

    #[test]
    fn tokens_read_the_json_list_and_an_unreadable_list_is_empty() {
        let row = |contains: &str| RuleRow {
            contains: contains.into(),
            ..rule_row("rule-1", 0, &[], Action::Ignore)
        };

        assert_eq!(row(r#"["A","B c"]"#).tokens(), vec!["A", "B c"]);
        assert_eq!(row("[]").tokens(), Vec::<String>::new());
        assert_eq!(row("oops").tokens(), Vec::<String>::new());
    }

    #[test]
    fn blank_tokens_are_dropped_from_the_list() {
        let row = RuleRow {
            contains: r#"[" ", "ALPEN", ""]"#.into(),
            ..rule_row("rule-1", 0, &[], Action::Ignore)
        };

        assert_eq!(row.tokens(), vec!["ALPEN"]);
    }

    #[test]
    fn actions_and_built_ins_are_stored_in_snake_case() {
        for (action, wire) in [
            (Action::Ignore, "ignore"),
            (Action::Anv, "anv"),
            (Action::Subscription, "subscription"),
        ] {
            assert_eq!(serde_json::to_value(action).unwrap(), json!(wire));
        }
        for (builtin, wire) in [
            (Builtin::Transfers, "transfers"),
            (Builtin::BankFees, "bank_fees"),
        ] {
            assert_eq!(serde_json::to_value(builtin).unwrap(), json!(wire));
        }
    }

    #[test]
    fn decisions_are_read_by_the_id_of_their_line() {
        let (dir, db) = setup();
        add(&db, ignore(&["Tankstelle", "Nordwind"]));
        let rule = last_rule(&db);
        import_statement(&db, dir.path(), &october());

        let all = db.read(decisions).unwrap();

        assert_eq!(all.len(), 2);
        for (line_id, decision) in &all {
            assert_eq!(&decision.id, line_id);
            assert_eq!(decision.rule_id.as_deref(), Some(rule.id.as_str()));
        }
    }

    #[test]
    fn matches_count_the_lines_a_rule_decided_with_the_newest_value_date() {
        let (dir, db) = setup();
        add(&db, ignore(&["Tankstelle"]));
        let tank = last_rule(&db);
        add(&db, ignore(&["E-Comm"]));
        let card = last_rule(&db);
        import_statement(&db, dir.path(), &october());
        decide_by_hand(&db, "CAFE");

        let matches = db.read(matches_by_rule).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches.get(&tank.id),
            Some(&Matches {
                count: 1,
                last: Some("2026-10-05".into())
            })
        );
        assert_eq!(
            matches.get(&card.id),
            Some(&Matches {
                count: 2,
                last: Some("2026-10-10".into())
            })
        );
    }

    #[test]
    fn a_rule_that_decided_nothing_has_no_matches_entry() {
        let (_dir, db) = setup();
        add(&db, ignore(&["Tankstelle"]));

        assert!(db.read(matches_by_rule).unwrap().is_empty());
        assert_eq!(
            Matches::default(),
            Matches {
                count: 0,
                last: None
            }
        );
    }

    #[test]
    fn the_same_decision_at_another_time_is_the_same_outcome() {
        let a = decided(Action::Ignore);

        assert!(a.same_as(&DecisionRow {
            decided_at: 99,
            ..a.clone()
        }));
        assert!(a.same_as(&a));
    }

    #[test]
    fn a_decision_differing_in_any_outcome_field_is_not_the_same() {
        let a = decided(Action::Ignore);
        let others = [
            (
                "id",
                DecisionRow {
                    id: "line-2".into(),
                    ..a.clone()
                },
            ),
            (
                "kind",
                DecisionRow {
                    kind: Action::Anv,
                    ..a.clone()
                },
            ),
            (
                "item",
                DecisionRow {
                    item_key: Some("books".into()),
                    ..a.clone()
                },
            ),
            (
                "subscription",
                DecisionRow {
                    subscription_id: Some("sub-1".into()),
                    ..a.clone()
                },
            ),
            (
                "document",
                DecisionRow {
                    document_id: Some("doc-1".into()),
                    ..a.clone()
                },
            ),
            (
                "hand",
                DecisionRow {
                    rule_id: None,
                    ..a.clone()
                },
            ),
            (
                "rule",
                DecisionRow {
                    rule_id: Some("rule-2".into()),
                    ..a.clone()
                },
            ),
        ];

        for (what, other) in others {
            assert!(!a.same_as(&other), "{what}");
        }
    }
}
