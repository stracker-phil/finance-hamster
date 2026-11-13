//! Rules against a temp database: saving, deleting and re-running them over imported lines. The helpers
//! serve the other rules test modules.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::engine::OwnAccount;
use super::store::{self, Action, Builtin, DecisionRow, RuleRow};
use super::{ItemOption, RuleForm, builtin_rules, delete, ensure_builtins, items, rerun, save};
use crate::accounts::{self, AccountRow};
use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::statements::import;
use crate::statements::import_tests::{october, open_with_accounts};
use crate::statements::store::{self as statements, StatementRow, TransactionRow};
use crate::taxyears::{self, TaxYearRow, YearState};
use crate::test_support::rules::{minimal_rules, rules_text, set};
use crate::test_support::statements::{Statement, StatementLine};

pub(super) const NOW: i64 = 1_700_000_000;
pub(super) const PRIVATE_IBAN: &str = "AT241999900000000001";
pub(super) const HOUSEHOLD_IBAN: &str = "AT941999900000000002";
const BUSINESS_IBAN: &str = "AT671999900000000003";

/// The rules file of `year`: item `books` (KZ 720) under `item_name`, and a hidden item `hidden`.
pub(super) fn rules_json(year: i32, item_name: &str) -> Value {
    let mut rules = minimal_rules(year);
    set(&mut rules, "/items/0/name", json!(item_name));
    if let Some(list) = rules["items"].as_array_mut() {
        list.push(json!({
            "key": "hidden",
            "kz": "721",
            "name": "Hidden",
            "section": "main",
            "kind": "documents",
            "visible": false
        }));
    }
    rules
}

pub(super) fn add_year(db: &Db, year: i32, state: YearState, item_name: &str) {
    let row = TaxYearRow {
        state,
        closed_at: (state == YearState::Closed).then_some(1),
        ..TaxYearRow::new(
            year,
            format!("{year} v1"),
            rules_text(&rules_json(year, item_name)),
            1,
        )
    };
    db.write(
        Op::new(
            "Add year",
            vec![Change::insert(taxyears::TABLE, &row).unwrap()],
        ),
        Source::App,
    )
    .unwrap();
}

/// The demo accounts as `account-1..3` and an open 2026 tax year whose item `books` reads `Fachliteratur`.
pub(super) fn setup() -> (TempDir, Db) {
    let (dir, db) = open_with_accounts();
    add_year(&db, 2026, YearState::Open, "Fachliteratur");
    (dir, db)
}

pub(super) fn ignore(tokens: &[&str]) -> RuleForm {
    RuleForm {
        contains: tokens.iter().map(|t| (*t).to_string()).collect(),
        amount: None,
        tolerance: None,
        account_id: None,
        own_counterparty: false,
        action: Action::Ignore,
        item_key: None,
    }
}

pub(super) fn assign(tokens: &[&str], item_key: &str) -> RuleForm {
    RuleForm {
        action: Action::Anv,
        item_key: Some(item_key.into()),
        ..ignore(tokens)
    }
}

pub(super) fn add(db: &Db, form: RuleForm) -> Written {
    save(db, None, &form, NOW, Source::Ui).unwrap()
}

pub(super) fn rules_of(db: &Db) -> Vec<RuleRow> {
    db.read(store::rules).unwrap()
}

/// The rule that runs last.
pub(super) fn last_rule(db: &Db) -> RuleRow {
    rules_of(db).pop().unwrap()
}

pub(super) fn rule_row(id: &str, position: i64, tokens: &[&str], action: Action) -> RuleRow {
    RuleRow {
        id: id.into(),
        position,
        builtin: None,
        contains: serde_json::to_string(tokens).unwrap(),
        amount_cents: None,
        tolerance_cents: 0,
        account_id: None,
        own_counterparty: false,
        action,
        item_key: (action == Action::Anv).then(|| "books".to_string()),
        subscription_id: None,
        document_id: None,
        created_at: position,
    }
}

/// Stores a rule without applying it to the lines.
pub(super) fn insert_rule(db: &Db, row: &RuleRow) {
    db.write(
        Op::new("Add rule", vec![Change::insert(store::RULES, row).unwrap()]),
        Source::App,
    )
    .unwrap();
}

pub(super) fn import_statement(db: &Db, dir: &Path, statement: &Statement) -> StatementRow {
    let pending = import::prepare(db, dir, statement.pdf()).unwrap();
    import::commit(db, dir, pending, NOW, Source::Import)
        .unwrap()
        .1
}

fn decisions_of(db: &Db) -> HashMap<String, DecisionRow> {
    db.read(store::decisions).unwrap()
}

fn line_with(db: &Db, needle: &str) -> TransactionRow {
    db.read(statements::all_transactions)
        .unwrap()
        .into_iter()
        .find(|l| format!("{} {}", l.text, l.extra).contains(needle))
        .unwrap_or_else(|| panic!("no line with {needle}"))
}

/// The decision of the line whose text or detail lines contain `needle`.
pub(super) fn decision_for(db: &Db, needle: &str) -> Option<DecisionRow> {
    decisions_of(db).remove(&line_with(db, needle).id)
}

/// Decides the line by hand: assigned to `books`, no rule.
pub(super) fn decide_by_hand(db: &Db, needle: &str) -> DecisionRow {
    let row = DecisionRow {
        id: line_with(db, needle).id,
        kind: Action::Anv,
        item_key: Some("books".into()),
        subscription_id: None,
        document_id: None,
        rule_id: None,
        decided_at: 5,
    };
    db.write(
        Op::new(
            "Decide",
            vec![Change::insert(store::DECISIONS, &row).unwrap()],
        ),
        Source::App,
    )
    .unwrap();
    row
}

/// A line of the demo Private account, a STREAMFLIX card payment.
pub(super) fn line() -> TransactionRow {
    TransactionRow {
        id: "line-1".into(),
        statement_id: "statement-1".into(),
        account_id: "account-1".into(),
        page: 1,
        row: 1,
        value_date: "2026-10-07".into(),
        cents: -1_299,
        text: "E-COMM 12,99 AT K1 06.10. STREAMFLIX WIEN 040".into(),
        extra: "[]".into(),
        counterparty: None,
        counterparty_iban: None,
        card_kind: Some("online".into()),
        card_date: Some("2026-10-06".into()),
        card_time: None,
        merchant: None,
        original_cents: None,
        original_currency: None,
        fx_fee: None,
        fx_rate: None,
        fx_date: None,
        transfer: false,
    }
}

pub(super) fn own_accounts() -> Vec<OwnAccount> {
    vec![
        ("account-1".into(), PRIVATE_IBAN.into()),
        ("account-2".into(), HOUSEHOLD_IBAN.into()),
        ("account-3".into(), BUSINESS_IBAN.into()),
    ]
}

/// A standing order from Private to Household on `day` of October 2026.
pub(super) fn household_transfer(day: u32) -> StatementLine {
    StatementLine {
        text: "Dauerauftrag".into(),
        extra: vec![
            "HOUSEHOLD".into(),
            "AT94 1999 9000 0000 0002".into(),
            "HAUSHALT".into(),
        ],
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 10, day).unwrap(),
        cents: -50_000,
    }
}

fn set_own(db: &Db, id: &str, own: bool) {
    let before = db
        .read(accounts::rows)
        .unwrap()
        .into_iter()
        .find(|a| a.id == id)
        .unwrap();
    let after = AccountRow {
        own,
        ..before.clone()
    };
    db.write(
        Op::new(
            "Edit account",
            vec![Change::update(accounts::TABLE, &before, &after).unwrap()],
        ),
        Source::App,
    )
    .unwrap();
}

fn invalid_field(result: AppResult<Written>) -> String {
    match result {
        Err(AppError::InvalidField { field, .. }) => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn labels(tables: &[&str]) -> Vec<String> {
    tables.iter().map(|t| (*t).to_string()).collect()
}

#[test]
fn a_rule_form_travels_in_camel_case() {
    let form = RuleForm {
        contains: vec!["Cloudbox".into()],
        amount: Some("25,95".into()),
        tolerance: None,
        account_id: Some("account-1".into()),
        own_counterparty: true,
        action: Action::Anv,
        item_key: Some("books".into()),
    };
    let wire = json!({
        "contains": ["Cloudbox"],
        "amount": "25,95",
        "tolerance": null,
        "accountId": "account-1",
        "ownCounterparty": true,
        "action": "anv",
        "itemKey": "books"
    });

    assert_eq!(serde_json::to_value(&form).unwrap(), wire);
    assert_eq!(serde_json::from_value::<RuleForm>(wire).unwrap(), form);
}

#[test]
fn items_list_the_visible_items_of_the_open_year_with_their_kz() {
    let (_dir, db) = setup();

    assert_eq!(
        items(&db).unwrap(),
        vec![ItemOption {
            key: "books".into(),
            label: "KZ 720 · Fachliteratur".into(),
            kz: "720".into(),
        }]
    );
}

#[test]
fn without_a_tax_year_there_are_no_items() {
    let (_dir, db) = open_with_accounts();
    assert_eq!(items(&db).unwrap(), vec![]);
}

#[test]
fn the_open_year_gives_the_items_even_when_a_newer_year_is_closed() {
    let (_dir, db) = setup();
    add_year(&db, 2027, YearState::Closed, "Neu");

    let labels: Vec<String> = items(&db).unwrap().into_iter().map(|i| i.label).collect();

    assert_eq!(labels, vec!["KZ 720 · Fachliteratur"]);
}

#[test]
fn with_only_closed_years_the_newest_gives_the_items() {
    let (_dir, db) = open_with_accounts();
    add_year(&db, 2025, YearState::Closed, "Alt");
    add_year(&db, 2026, YearState::Closed, "Neu");

    let labels: Vec<String> = items(&db).unwrap().into_iter().map(|i| i.label).collect();

    assert_eq!(labels, vec!["KZ 720 · Neu"]);
}

#[test]
fn the_first_owner_rule_gets_position_zero_and_later_ones_follow() {
    let (_dir, db) = setup();
    add(&db, ignore(&["Cloudbox"]));
    add(&db, ignore(&["Lumen"]));

    let shown: Vec<(i64, Vec<String>)> = rules_of(&db)
        .iter()
        .map(|r| (r.position, r.tokens()))
        .collect();

    assert_eq!(
        shown,
        vec![
            (0, vec!["Cloudbox".to_string()]),
            (1, vec!["Lumen".to_string()])
        ]
    );
}

#[test]
fn an_added_rule_runs_after_the_built_in_ones() {
    let (_dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    add(&db, ignore(&["Cloudbox"]));

    let positions: Vec<i64> = rules_of(&db).iter().map(|r| r.position).collect();

    assert_eq!(positions, vec![-2, -1, 0]);
}

#[test]
fn a_saved_rule_stores_trimmed_tokens_and_amounts_as_absolute_cents() {
    let (_dir, db) = setup();
    let form = RuleForm {
        contains: vec![
            " Cloudbox ".into(),
            String::new(),
            "  ".into(),
            "Lumen".into(),
        ],
        amount: Some("-25,95".into()),
        tolerance: Some("1.00".into()),
        account_id: Some("account-1".into()),
        own_counterparty: false,
        action: Action::Ignore,
        item_key: Some("books".into()),
    };

    add(&db, form);

    let rule = last_rule(&db);
    assert_eq!(rule.tokens(), vec!["Cloudbox", "Lumen"]);
    assert_eq!(rule.amount_cents, Some(2_595));
    assert_eq!(rule.tolerance_cents, 100);
    assert_eq!(rule.account_id.as_deref(), Some("account-1"));
    assert_eq!(rule.item_key, None);
    assert_eq!(rule.builtin, None);
    assert_eq!((rule.action, rule.created_at), (Action::Ignore, NOW));
}

#[test]
fn an_item_rule_stores_its_item() {
    let (_dir, db) = setup();

    add(&db, assign(&["Seitenweise"], "books"));

    let rule = last_rule(&db);
    assert_eq!(rule.action, Action::Anv);
    assert_eq!(rule.item_key.as_deref(), Some("books"));
}

#[test]
fn each_kind_of_condition_alone_is_enough_for_a_rule() {
    let (_dir, db) = setup();
    let forms = [
        ignore(&["Cloudbox"]),
        RuleForm {
            amount: Some("9,99".into()),
            ..ignore(&[])
        },
        RuleForm {
            account_id: Some("account-2".into()),
            ..ignore(&[])
        },
        RuleForm {
            own_counterparty: true,
            ..ignore(&[])
        },
        RuleForm {
            tolerance: Some("0".into()),
            ..ignore(&["Lumen"])
        },
    ];

    for form in forms {
        save(&db, None, &form, NOW, Source::Ui).unwrap_or_else(|e| panic!("{form:?}: {e:?}"));
    }

    assert_eq!(rules_of(&db).len(), 5);
}

#[test]
fn a_rule_the_form_cannot_express_is_refused_naming_the_field() {
    let (_dir, db) = setup();
    let base = ignore(&["Cloudbox"]);
    let cases = [
        (
            "rule.conditions",
            RuleForm {
                contains: vec![" ".into(), String::new()],
                ..base.clone()
            },
        ),
        (
            "rule.conditions",
            RuleForm {
                amount: Some("  ".into()),
                ..ignore(&[])
            },
        ),
        (
            "rule.amount",
            RuleForm {
                amount: Some("abc".into()),
                ..base.clone()
            },
        ),
        (
            "rule.tolerance",
            RuleForm {
                tolerance: Some("1,00".into()),
                ..base.clone()
            },
        ),
        (
            "rule.tolerance",
            RuleForm {
                amount: Some("5".into()),
                tolerance: Some("x".into()),
                ..base.clone()
            },
        ),
        (
            "rule.account",
            RuleForm {
                account_id: Some("account-9".into()),
                ..base.clone()
            },
        ),
        (
            "rule.item",
            RuleForm {
                action: Action::Anv,
                ..base.clone()
            },
        ),
        ("rule.item", assign(&["Cloudbox"], "no-such-item")),
        ("rule.item", assign(&["Cloudbox"], "hidden")),
        (
            "rule.action",
            RuleForm {
                action: Action::Subscription,
                ..base.clone()
            },
        ),
    ];

    for (field, form) in cases {
        assert_eq!(
            invalid_field(save(&db, None, &form, NOW, Source::Ui)),
            field,
            "{form:?}"
        );
    }

    assert_eq!(rules_of(&db), vec![]);
}

#[test]
fn an_item_rule_needs_a_tax_year_to_take_its_item_from() {
    let (_dir, db) = open_with_accounts();

    let result = save(&db, None, &assign(&["Cloudbox"], "books"), NOW, Source::Ui);

    assert_eq!(invalid_field(result), "rule.item");
}

#[test]
fn saving_a_rule_applies_it_to_the_lines_already_imported_and_undo_reverts_both() {
    let (dir, db) = setup();
    import_statement(&db, dir.path(), &october());
    assert_eq!(decisions_of(&db), HashMap::new());

    let written = add(&db, ignore(&["Nordwind"]));

    assert_eq!(
        written,
        Written {
            label: "Add Rule".into(),
            tables: labels(&["decisions", "rules"]),
        }
    );
    let rule = last_rule(&db);
    let decision = decision_for(&db, "NORDWIND").unwrap();
    assert_eq!(
        decision,
        DecisionRow {
            id: decision.id.clone(),
            kind: Action::Ignore,
            item_key: None,
            subscription_id: None,
            document_id: None,
            rule_id: Some(rule.id),
            decided_at: NOW,
        }
    );
    assert_eq!(decisions_of(&db).len(), 1);

    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.label, "Add Rule");
    assert_eq!(rules_of(&db), vec![]);
    assert_eq!(decisions_of(&db), HashMap::new());
}

#[test]
fn an_item_rule_assigns_its_item_to_the_lines_it_matches() {
    let (dir, db) = setup();
    import_statement(&db, dir.path(), &october());

    add(&db, assign(&["Seitenweise"], "books"));

    let rule = last_rule(&db);
    let decision = decision_for(&db, "SEITENWEISE").unwrap();
    assert_eq!(
        (decision.kind, decision.item_key, decision.rule_id),
        (Action::Anv, Some("books".into()), Some(rule.id))
    );
}

#[test]
fn editing_a_rule_changes_which_lines_it_decides_and_undo_restores_them() {
    let (dir, db) = setup();
    add(&db, ignore(&["Tankstelle"]));
    let rule = last_rule(&db);
    import_statement(&db, dir.path(), &october());
    assert_eq!(
        decision_for(&db, "TANKSTELLE").map(|d| d.rule_id),
        Some(Some(rule.id.clone()))
    );

    let written = save(&db, Some(&rule.id), &ignore(&["Cafe"]), NOW + 1, Source::Ui).unwrap();

    assert_eq!(
        written,
        Written {
            label: "Edit Rule".into(),
            tables: labels(&["decisions", "rules"]),
        }
    );
    assert_eq!(decision_for(&db, "TANKSTELLE"), None);
    let cafe = decision_for(&db, "CAFE").unwrap();
    assert_eq!(
        (cafe.rule_id, cafe.decided_at),
        (Some(rule.id.clone()), NOW + 1)
    );
    let edited = last_rule(&db);
    assert_eq!(
        (edited.id.as_str(), edited.position, edited.created_at),
        (rule.id.as_str(), rule.position, rule.created_at)
    );
    assert_eq!(edited.tokens(), vec!["Cafe"]);

    db.undo().unwrap().unwrap();

    assert_eq!(last_rule(&db), rule);
    assert_eq!(decision_for(&db, "CAFE"), None);
    assert!(decision_for(&db, "TANKSTELLE").is_some());
}

#[test]
fn editing_a_built_in_rule_keeps_its_position_and_built_in_flag() {
    let (_dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    let fees = rules_of(&db)
        .into_iter()
        .find(|r| r.builtin == Some(Builtin::BankFees))
        .unwrap();

    save(
        &db,
        Some(&fees.id),
        &ignore(&["Kontoführung", "Gebühr"]),
        NOW,
        Source::Ui,
    )
    .unwrap();

    let after = rules_of(&db).into_iter().find(|r| r.id == fees.id).unwrap();
    assert_eq!(
        (after.builtin, after.position),
        (Some(Builtin::BankFees), -1)
    );
    assert_eq!(after.tokens(), vec!["Kontoführung", "Gebühr"]);
}

#[test]
fn editing_an_unknown_rule_is_not_found() {
    let (_dir, db) = setup();

    let result = save(&db, Some("no-such-rule"), &ignore(&["x"]), NOW, Source::Ui);

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

#[test]
fn an_invalid_edit_leaves_the_rule_as_it_was() {
    let (_dir, db) = setup();
    add(&db, ignore(&["Cloudbox"]));
    let rule = last_rule(&db);

    let result = save(&db, Some(&rule.id), &ignore(&[]), NOW, Source::Ui);

    assert_eq!(invalid_field(result), "rule.conditions");
    assert_eq!(last_rule(&db), rule);
}

#[test]
fn hand_decisions_survive_adding_editing_and_deleting_rules() {
    let (dir, db) = setup();
    import_statement(&db, dir.path(), &october());
    let hand = decide_by_hand(&db, "STREAMFLIX");

    add(&db, ignore(&["Streamflix"]));
    assert_eq!(decision_for(&db, "STREAMFLIX"), Some(hand.clone()));

    let rule = last_rule(&db);
    save(
        &db,
        Some(&rule.id),
        &ignore(&["Streamflix", "Lumen"]),
        NOW,
        Source::Ui,
    )
    .unwrap();
    assert_eq!(decision_for(&db, "STREAMFLIX"), Some(hand.clone()));
    assert!(decision_for(&db, "LUMEN AI").is_some());

    delete(&db, &rule.id, NOW, Source::Ui).unwrap();
    assert_eq!(decision_for(&db, "STREAMFLIX"), Some(hand));
    assert_eq!(decision_for(&db, "LUMEN AI"), None);
}

#[test]
fn re_running_rules_leaves_hand_decisions_alone_and_counts_only_the_lines_it_changes() {
    let (dir, db) = setup();
    import_statement(&db, dir.path(), &october());
    let hand = decide_by_hand(&db, "STREAMFLIX");
    insert_rule(&db, &rule_row("rule-card", 0, &["E-COMM"], Action::Ignore));

    let (_, lines) = rerun(&db, NOW, Source::Ui).unwrap().unwrap();

    assert_eq!(lines, 1);
    assert_eq!(decision_for(&db, "STREAMFLIX"), Some(hand));
    assert_eq!(
        decision_for(&db, "LUMEN AI").map(|d| d.rule_id),
        Some(Some("rule-card".to_string()))
    );
}

#[test]
fn deleting_a_rule_hands_its_lines_to_the_next_matching_rule_or_back_to_review() {
    let (dir, db) = setup();
    add(&db, ignore(&["Tankstelle"]));
    let first = last_rule(&db);
    add(&db, assign(&["Rhein"], "books"));
    let second = last_rule(&db);
    import_statement(&db, dir.path(), &october());
    assert_eq!(
        decision_for(&db, "TANKSTELLE").map(|d| d.rule_id),
        Some(Some(first.id.clone()))
    );

    let written = delete(&db, &first.id, NOW + 1, Source::Ui).unwrap();

    assert_eq!(
        written,
        Written {
            label: "Delete Rule".into(),
            tables: labels(&["decisions", "rules"]),
        }
    );
    assert_eq!(rules_of(&db), vec![second.clone()]);
    let handed = decision_for(&db, "TANKSTELLE").unwrap();
    assert_eq!(
        (
            handed.kind,
            handed.item_key,
            handed.rule_id,
            handed.decided_at
        ),
        (
            Action::Anv,
            Some("books".into()),
            Some(second.id.clone()),
            NOW + 1
        )
    );

    delete(&db, &second.id, NOW + 2, Source::Ui).unwrap();

    assert_eq!(rules_of(&db), vec![]);
    assert_eq!(decision_for(&db, "TANKSTELLE"), None);

    db.undo().unwrap().unwrap();

    assert_eq!(rules_of(&db), vec![second.clone()]);
    assert_eq!(
        decision_for(&db, "TANKSTELLE").map(|d| d.rule_id),
        Some(Some(second.id))
    );
}

#[test]
fn built_in_rules_cannot_be_deleted() {
    let (_dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();

    for id in ["builtin-transfers", "builtin-bank-fees"] {
        let result = delete(&db, id, NOW, Source::Ui);
        assert_eq!(
            result,
            Err(AppError::InvalidInput(
                "A built-in rule can't be deleted".into()
            )),
            "{id}"
        );
    }

    assert_eq!(rules_of(&db).len(), 2);
}

#[test]
fn deleting_an_unknown_rule_is_not_found() {
    let (_dir, db) = setup();

    let result = delete(&db, "no-such-rule", NOW, Source::Ui);

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

#[test]
fn re_running_rules_with_nothing_to_change_reports_none() {
    let (dir, db) = setup();
    assert_eq!(rerun(&db, NOW, Source::Ui).unwrap(), None);

    add(&db, ignore(&["Tankstelle"]));
    import_statement(&db, dir.path(), &october());

    assert_eq!(rerun(&db, NOW, Source::Ui).unwrap(), None);
    assert_eq!(rerun(&db, NOW + 1_000, Source::Ui).unwrap(), None);
}

#[test]
fn re_running_applies_a_rule_stored_since_and_reports_how_many_lines_changed() {
    let (dir, db) = setup();
    import_statement(&db, dir.path(), &october());
    insert_rule(&db, &rule_row("rule-card", 0, &["E-COMM"], Action::Ignore));

    let (written, lines) = rerun(&db, NOW, Source::Ui).unwrap().unwrap();

    assert_eq!(
        written,
        Written {
            label: "Re-run Rules".into(),
            tables: labels(&["decisions"]),
        }
    );
    assert_eq!(lines, 2);
    assert_eq!(decisions_of(&db).len(), 2);
    assert_eq!(rerun(&db, NOW, Source::Ui).unwrap(), None);

    db.undo().unwrap().unwrap();

    assert_eq!(decisions_of(&db), HashMap::new());
}

#[test]
fn re_running_removes_the_decisions_of_a_rule_that_is_gone() {
    let (dir, db) = setup();
    let rule = rule_row("rule-card", 0, &["E-COMM"], Action::Ignore);
    insert_rule(&db, &rule);
    import_statement(&db, dir.path(), &october());
    assert_eq!(decisions_of(&db).len(), 2);
    db.write(
        Op::new(
            "Remove rule",
            vec![Change::delete(store::RULES, &rule).unwrap()],
        ),
        Source::App,
    )
    .unwrap();

    let (_, lines) = rerun(&db, NOW, Source::Ui).unwrap().unwrap();

    assert_eq!(lines, 2);
    assert_eq!(decisions_of(&db), HashMap::new());
}

#[test]
fn re_running_picks_up_an_account_that_became_one_of_the_owners() {
    let (dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    set_own(&db, "account-2", false);
    let mut statement = october();
    statement.lines.insert(1, household_transfer(2));
    import_statement(&db, dir.path(), &statement);
    assert_eq!(decision_for(&db, "HOUSEHOLD"), None);

    set_own(&db, "account-2", true);
    let (written, lines) = rerun(&db, NOW, Source::Ui).unwrap().unwrap();

    assert_eq!(written.label, "Re-run Rules");
    assert_eq!(lines, 1);
    let decision = decision_for(&db, "HOUSEHOLD").unwrap();
    assert_eq!(
        (decision.kind, decision.rule_id),
        (Action::Ignore, Some("builtin-transfers".to_string()))
    );
    assert_eq!(rerun(&db, NOW, Source::Ui).unwrap(), None);
}

#[test]
fn the_built_in_rules_are_added_once() {
    let (_dir, db) = setup();

    let written = ensure_builtins(&db, NOW, Source::App).unwrap().unwrap();

    assert_eq!(
        written,
        Written {
            label: "Add Built-in Rules".into(),
            tables: labels(&["rules"]),
        }
    );
    assert_eq!(rules_of(&db), builtin_rules(NOW));
    let shown: Vec<(String, Option<Builtin>, i64, Vec<String>, bool, Action)> = rules_of(&db)
        .into_iter()
        .map(|r| {
            let tokens = r.tokens();
            (
                r.id,
                r.builtin,
                r.position,
                tokens,
                r.own_counterparty,
                r.action,
            )
        })
        .collect();
    assert_eq!(
        shown,
        vec![
            (
                "builtin-transfers".to_string(),
                Some(Builtin::Transfers),
                -2,
                vec![],
                true,
                Action::Ignore
            ),
            (
                "builtin-bank-fees".to_string(),
                Some(Builtin::BankFees),
                -1,
                vec!["Kontoführung".to_string(), "Debitkarte".to_string()],
                false,
                Action::Ignore
            ),
        ]
    );
    assert_eq!(ensure_builtins(&db, NOW, Source::App).unwrap(), None);
    assert_eq!(rules_of(&db).len(), 2);
}

#[test]
fn adding_the_built_in_rules_decides_the_lines_already_imported_in_the_same_op() {
    let (dir, db) = setup();
    let mut statement = october();
    statement.lines.insert(1, household_transfer(2));
    import_statement(&db, dir.path(), &statement);
    assert_eq!(decisions_of(&db), HashMap::new());

    let written = ensure_builtins(&db, NOW + 1, Source::App).unwrap().unwrap();

    assert_eq!(
        written,
        Written {
            label: "Add Built-in Rules".into(),
            tables: labels(&["decisions", "rules"]),
        }
    );
    let decision = decision_for(&db, "HOUSEHOLD").unwrap();
    assert_eq!(
        (decision.kind, decision.rule_id, decision.decided_at),
        (
            Action::Ignore,
            Some("builtin-transfers".to_string()),
            NOW + 1
        )
    );
    assert_eq!(decisions_of(&db).len(), 1);
    assert_eq!(ensure_builtins(&db, NOW + 2, Source::App).unwrap(), None);
}

#[test]
fn only_the_missing_built_in_rule_is_added_back() {
    let (_dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    let fees = rules_of(&db)
        .into_iter()
        .find(|r| r.builtin == Some(Builtin::BankFees))
        .unwrap();
    db.write(
        Op::new(
            "Remove rule",
            vec![Change::delete(store::RULES, &fees).unwrap()],
        ),
        Source::App,
    )
    .unwrap();

    let written = ensure_builtins(&db, NOW, Source::App).unwrap().unwrap();

    assert_eq!(written.tables, labels(&["rules"]));
    let ids: Vec<String> = rules_of(&db).into_iter().map(|r| r.id).collect();
    assert_eq!(ids, vec!["builtin-transfers", "builtin-bank-fees"]);
}

#[test]
fn built_in_rules_run_before_the_owners_even_when_the_owners_came_first() {
    let (_dir, db) = setup();
    add(&db, ignore(&["Cloudbox"]));

    ensure_builtins(&db, NOW, Source::App).unwrap();

    let ids: Vec<String> = rules_of(&db)
        .into_iter()
        .map(|r| {
            r.builtin
                .map_or_else(|| "owner".to_string(), |_| r.id.clone())
        })
        .collect();
    assert_eq!(ids, vec!["builtin-transfers", "builtin-bank-fees", "owner"]);
}

#[test]
fn an_imported_line_a_rule_matches_arrives_decided_and_the_rest_wait_for_review() {
    let (dir, db) = setup();
    add(&db, ignore(&["Nordwind"]));
    let rule = last_rule(&db);

    import_statement(&db, dir.path(), &october());

    let salary = decision_for(&db, "NORDWIND").unwrap();
    assert_eq!(
        salary,
        DecisionRow {
            id: salary.id.clone(),
            kind: Action::Ignore,
            item_key: None,
            subscription_id: None,
            document_id: None,
            rule_id: Some(rule.id),
            decided_at: NOW,
        }
    );
    assert_eq!(decisions_of(&db).len(), 1);
    assert_eq!(decision_for(&db, "STREAMFLIX"), None);
}

#[test]
fn an_imported_line_gets_the_first_matching_rules_decision() {
    let (dir, db) = setup();
    add(&db, assign(&["Seitenweise"], "books"));
    add(&db, ignore(&["Seitenweise", "Nordwind"]));

    import_statement(&db, dir.path(), &october());

    let book = decision_for(&db, "SEITENWEISE").unwrap();
    assert_eq!(
        (book.kind, book.item_key),
        (Action::Anv, Some("books".into()))
    );
    assert_eq!(decision_for(&db, "NORDWIND").unwrap().kind, Action::Ignore);
}

#[test]
fn an_import_without_rules_decides_nothing() {
    let (dir, db) = setup();

    import_statement(&db, dir.path(), &october());

    assert_eq!(decisions_of(&db), HashMap::new());
}

#[test]
fn a_transfer_to_another_own_account_is_ignored_by_the_built_in_rule_on_import() {
    let (dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    let mut statement = october();
    statement.lines.insert(1, household_transfer(2));

    import_statement(&db, dir.path(), &statement);

    let decision = decision_for(&db, "HOUSEHOLD").unwrap();
    assert_eq!(
        (decision.kind, decision.rule_id),
        (Action::Ignore, Some("builtin-transfers".to_string()))
    );
    assert_eq!(decisions_of(&db).len(), 1);
}

#[test]
fn undoing_an_import_removes_its_decisions_and_keeps_the_rules() {
    let (dir, db) = setup();
    add(&db, ignore(&["Nordwind", "Tankstelle"]));
    import_statement(&db, dir.path(), &october());
    assert_eq!(decisions_of(&db).len(), 2);

    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.label, "Import Statement Nr. 010/2026");
    assert_eq!(decisions_of(&db), HashMap::new());
    assert_eq!(rules_of(&db).len(), 1);
}
