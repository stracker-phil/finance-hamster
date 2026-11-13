use std::collections::HashMap;

use tempfile::TempDir;

use super::{
    PlanChanged, ReviewLine, ReviewQueue, decide, key_of, queue, title_of, undecided_by_account,
};
use crate::date::DateOrder;
use crate::db::{Db, Source, Written};
use crate::error::AppError;
use crate::rules::store::{self as rules_store, Action, DecisionRow, RuleRow};
use crate::settings::NumberFormat;
use crate::statements::import;
use crate::statements::import_tests::{october, open_with_accounts, open_with_accounts_and_year};
use crate::statements::store::{self as statements, TransactionRow};
use crate::subscriptions::store as subscription_store;
use crate::test_support::demo::seed_if_empty;
use crate::test_support::statements::{Statement, demo_month};
use crate::test_support::subscriptions::{add as add_subscription, form as subscription_form};
use crate::test_support::temp_config_dir;

const NOW: i64 = 1_700_000_000;

fn september() -> Statement {
    demo_month(9, 2026, 9, 141_230)
}

fn november() -> Statement {
    demo_month(11, 2026, 11, 141_230)
}

fn import_all(db: &Db, dir: &TempDir, statements: &[Statement]) {
    for statement in statements {
        let pending = import::prepare(db, dir.path(), statement.pdf()).unwrap();
        import::commit(db, dir.path(), pending, NOW, Source::Import).unwrap();
    }
}

/// The demo accounts, an open 2026 year with item `books`, and the given statements imported without rules.
fn imported(statements: &[Statement]) -> (TempDir, Db) {
    let (dir, db) = open_with_accounts_and_year();
    import_all(&db, &dir, statements);
    (dir, db)
}

fn queue_of(db: &Db) -> ReviewQueue {
    queue(db, NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap()
}

fn titles(queue: &ReviewQueue) -> Vec<&str> {
    queue.lines.iter().map(|l| l.title.as_str()).collect()
}

fn waiting<'a>(queue: &'a ReviewQueue, title: &str) -> Vec<&'a ReviewLine> {
    queue.lines.iter().filter(|l| l.title == title).collect()
}

fn all_lines(db: &Db) -> Vec<TransactionRow> {
    db.read(statements::all_transactions).unwrap()
}

/// The id of the line titled `title` that is value-dated in `month` of 2026.
fn line_id(db: &Db, title: &str, month: u32) -> String {
    all_lines(db)
        .into_iter()
        .find(|l| title_of(l) == title && l.value_date.starts_with(&format!("2026-{month:02}")))
        .unwrap_or_else(|| panic!("no line {title} in month {month}"))
        .id
}

fn decisions(db: &Db) -> HashMap<String, DecisionRow> {
    db.read(rules_store::decisions).unwrap()
}

fn rules(db: &Db) -> Vec<RuleRow> {
    db.read(rules_store::rules).unwrap()
}

fn decision_of(db: &Db, title: &str, month: u32) -> Option<DecisionRow> {
    decisions(db).remove(&line_id(db, title, month))
}

/// The id of a subscription whose text matches no demo line, so a hand decision can link to it.
fn hand_linked_subscription(db: &Db) -> String {
    let existing = db
        .read(subscription_store::subscriptions)
        .unwrap()
        .into_iter()
        .find(|s| s.name == "Hand Linked");
    match existing {
        Some(sub) => sub.id,
        None => add_subscription(
            db,
            &subscription_form("Hand Linked", "NO SUCH TEXT", "1.00"),
        ),
    }
}

fn decide_line(
    db: &Db,
    title: &str,
    month: u32,
    action: Action,
    item_key: Option<&str>,
    always: bool,
) -> Written {
    let subscription_id = (action == Action::Subscription).then(|| hand_linked_subscription(db));
    decide(
        db,
        &line_id(db, title, month),
        action,
        item_key.map(Into::into),
        subscription_id,
        always,
        NOW + 5,
        Source::Ui,
    )
    .unwrap()
}

fn invalid_field(result: Result<Written, AppError>) -> String {
    match result {
        Err(AppError::InvalidField { field, .. }) => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn written(label: &str, tables: &[&str]) -> Written {
    Written {
        label: label.into(),
        tables: tables.iter().map(|t| (*t).to_string()).collect(),
    }
}

fn row(text: &str, merchant: Option<&str>, counterparty: Option<&str>) -> TransactionRow {
    TransactionRow {
        id: "line-1".into(),
        statement_id: "statement-1".into(),
        account_id: "account-1".into(),
        page: 1,
        row: 1,
        value_date: "2026-10-07".into(),
        cents: -1_299,
        text: text.into(),
        extra: "[]".into(),
        counterparty: counterparty.map(Into::into),
        counterparty_iban: None,
        card_kind: None,
        card_date: None,
        card_time: None,
        merchant: merchant.map(Into::into),
        original_cents: None,
        original_currency: None,
        fx_fee: None,
        fx_rate: None,
        fx_date: None,
        transfer: false,
    }
}

#[test]
fn a_title_is_the_merchant_then_the_counterparty_then_the_booking_text() {
    for (line, expected) in [
        (
            row("E-COMM", Some("STREAMFLIX WIEN"), Some("Other")),
            "STREAMFLIX WIEN",
        ),
        (
            row("Gehalt", None, Some("NORDWIND SOFTWARE GMBH")),
            "NORDWIND SOFTWARE GMBH",
        ),
        (row("Entgelt", None, None), "Entgelt"),
    ] {
        assert_eq!(title_of(&line), expected);
    }
}

#[test]
fn a_key_is_the_first_word_with_letters_in_upper_case_or_two_words_when_the_first_is_short() {
    for (title, expected) in [
        ("TANKSTELLE RHEIN LUSTENAU", "TANKSTELLE"),
        ("DM DROGERIE MARKT", "DM DROGERIE"),
        ("cafe linde dornbirn", "CAFE"),
        ("12 Bakery Muster", "BAKERY"),
        ("DM", "DM"),
        ("123 456", "123 456"),
        ("  ", ""),
    ] {
        assert_eq!(key_of(title), expected, "{title:?}");
    }
}

#[test]
fn lines_without_a_decision_are_counted_per_account() {
    let (dir, db) = imported(&[october()]);
    assert_eq!(
        db.read(undecided_by_account).unwrap(),
        HashMap::from([("account-1".to_string(), 7)])
    );

    import_all(&db, &dir, &[september()]);
    decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, false);

    assert_eq!(
        db.read(undecided_by_account).unwrap(),
        HashMap::from([("account-1".to_string(), 13)])
    );
}

#[test]
fn no_lines_means_nothing_is_counted() {
    let (_dir, db) = open_with_accounts();
    assert_eq!(db.read(undecided_by_account).unwrap(), HashMap::new());
}

#[test]
fn the_queue_lists_undecided_lines_newest_first() {
    let (_dir, db) = imported(&[october()]);

    let queue = queue_of(&db);

    assert_eq!(
        titles(&queue),
        [
            "CAFE LINDE DORNBIRN",
            "BUCHHANDLUNG SEITENWEISE",
            "LUMEN AI SAN FRANCISCO",
            "STREAMFLIX WIEN",
            "TANKSTELLE RHEIN LUSTENAU",
            "HAUSVERWALTUNG MUSTER",
            "NORDWIND SOFTWARE GMBH",
        ]
    );
}

#[test]
fn a_queued_line_shows_its_account_date_amount_raw_text_and_key() {
    let (_dir, db) = imported(&[october()]);

    let queue = queue_of(&db);

    assert_eq!(
        waiting(&queue, "NORDWIND SOFTWARE GMBH"),
        [&ReviewLine {
            id: line_id(&db, "NORDWIND SOFTWARE GMBH", 10),
            account_name: "Private".into(),
            account_index: 0,
            date: "Thu, 1 Oct".into(),
            title: "NORDWIND SOFTWARE GMBH".into(),
            amount: "+€2,850.00".into(),
            raw: "Gehalt\nNORDWIND SOFTWARE GMBH\nAT94 1999 9000 0000 0099\nREF: LOHN".into(),
            key: "NORDWIND".into(),
            suggestion: None,
            item_key: None,
            item_why: None,
            plan_changed: None,
        }]
    );
    let card = &waiting(&queue, "STREAMFLIX WIEN")[0];
    assert_eq!(
        (card.date.as_str(), card.amount.as_str(), card.raw.as_str()),
        (
            "Wed, 7 Oct",
            "−€12.99",
            "E-COMM 12,99 AT K1 06.10. STREAMFLIX WIEN 040"
        )
    );
}

#[test]
fn the_queue_follows_the_number_format_and_date_order() {
    let (_dir, db) = imported(&[october()]);

    let queue = queue(&db, NumberFormat::CommaDecimal, DateOrder::MonthFirst).unwrap();

    let line = &waiting(&queue, "NORDWIND SOFTWARE GMBH")[0];
    assert_eq!(
        (line.date.as_str(), line.amount.as_str()),
        ("Thu, Oct 1", "+€2.850,00")
    );
}

#[test]
fn the_queue_offers_the_items_of_the_open_year() {
    let (_dir, db) = imported(&[october()]);

    let keys: Vec<String> = queue_of(&db).items.into_iter().map(|i| i.key).collect();

    assert_eq!(keys, ["books"]);
}

#[test]
fn without_a_tax_year_the_queue_has_no_items() {
    let (dir, db) = open_with_accounts();
    import_all(&db, &dir, &[october()]);

    assert_eq!(queue_of(&db).items, vec![]);
}

#[test]
fn lines_with_a_decision_leave_the_queue() {
    let (_dir, db) = imported(&[october()]);

    decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, false);

    let queue = queue_of(&db);
    assert_eq!(queue.lines.len(), 6);
    assert!(waiting(&queue, "STREAMFLIX WIEN").is_empty());
}

#[test]
fn a_line_suggests_the_action_and_item_of_the_newest_hand_decision_for_its_key() {
    let (_dir, db) = imported(&[september(), october(), november()]);
    decide_line(
        &db,
        "BUCHHANDLUNG SEITENWEISE",
        9,
        Action::Ignore,
        None,
        false,
    );
    decide_line(
        &db,
        "BUCHHANDLUNG SEITENWEISE",
        10,
        Action::Anv,
        Some("books"),
        false,
    );

    let queue = queue_of(&db);

    let line = &waiting(&queue, "BUCHHANDLUNG SEITENWEISE")[0];
    assert_eq!(line.id, line_id(&db, "BUCHHANDLUNG SEITENWEISE", 11));
    assert_eq!(line.suggestion, Some(Action::Anv));
    assert_eq!(line.item_key.as_deref(), Some("books"));
    assert_eq!(
        line.item_why.as_deref(),
        Some("last used for BUCHHANDLUNG SEITENWEISE")
    );
}

#[test]
fn a_suggestion_without_an_item_has_no_item_hint() {
    let (_dir, db) = imported(&[september(), october()]);
    decide_line(&db, "CAFE LINDE DORNBIRN", 9, Action::Ignore, None, false);

    let queue = queue_of(&db);

    let line = &waiting(&queue, "CAFE LINDE DORNBIRN")[0];
    assert_eq!(
        (
            line.suggestion,
            line.item_key.as_deref(),
            line.item_why.as_deref()
        ),
        (Some(Action::Ignore), None, None)
    );
}

#[test]
fn the_seeded_demo_keeps_only_the_cafe_lines_in_the_queue() {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    seed_if_empty(
        &db,
        dir.path(),
        chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap(),
    )
    .unwrap();

    let queue = queue_of(&db);

    let mut distinct: Vec<&str> = titles(&queue);
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct, ["CAFE LINDE DORNBIRN"]);
    assert_eq!(
        queue
            .subscriptions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["Lumen AI", "Streamflix"]
    );
}

#[test]
fn deciding_a_line_by_hand_stores_a_decision_without_a_rule() {
    let (_dir, db) = imported(&[october()]);

    let result = decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, false);

    assert_eq!(result, written("Ignore: STREAMFLIX WIEN", &["decisions"]));
    assert_eq!(
        decision_of(&db, "STREAMFLIX WIEN", 10),
        Some(DecisionRow {
            id: line_id(&db, "STREAMFLIX WIEN", 10),
            kind: Action::Ignore,
            item_key: None,
            subscription_id: None,
            document_id: None,
            rule_id: None,
            decided_at: NOW + 5,
        })
    );
    assert_eq!(rules(&db), vec![]);
}

#[test]
fn the_label_names_the_action_and_the_line() {
    let (_dir, db) = imported(&[october()]);

    for (title, action, item, label) in [
        (
            "BUCHHANDLUNG SEITENWEISE",
            Action::Anv,
            Some("books"),
            "ANV: BUCHHANDLUNG SEITENWEISE",
        ),
        (
            "STREAMFLIX WIEN",
            Action::Subscription,
            None,
            "Subscription: STREAMFLIX WIEN",
        ),
        (
            "CAFE LINDE DORNBIRN",
            Action::Ignore,
            None,
            "Ignore: CAFE LINDE DORNBIRN",
        ),
    ] {
        let result = decide_line(&db, title, 10, action, item, false);
        assert_eq!(result.label, label);
    }
}

#[test]
fn an_item_decision_keeps_its_item_and_any_other_action_drops_it() {
    let (_dir, db) = imported(&[october()]);

    decide_line(
        &db,
        "BUCHHANDLUNG SEITENWEISE",
        10,
        Action::Anv,
        Some("books"),
        false,
    );
    decide_line(
        &db,
        "CAFE LINDE DORNBIRN",
        10,
        Action::Ignore,
        Some("books"),
        false,
    );
    decide_line(
        &db,
        "STREAMFLIX WIEN",
        10,
        Action::Subscription,
        Some("books"),
        false,
    );

    let item = |title: &str| decision_of(&db, title, 10).and_then(|d| d.item_key);
    assert_eq!(item("BUCHHANDLUNG SEITENWEISE").as_deref(), Some("books"));
    assert_eq!(item("CAFE LINDE DORNBIRN"), None);
    assert_eq!(item("STREAMFLIX WIEN"), None);
}

#[test]
fn an_item_decision_needs_an_item_of_the_year() {
    let (_dir, db) = imported(&[october()]);
    let id = line_id(&db, "BUCHHANDLUNG SEITENWEISE", 10);

    for item in [None, Some("no-such-item".to_string()), Some(String::new())] {
        let result = decide(
            &db,
            &id,
            Action::Anv,
            item.clone(),
            None,
            true,
            NOW,
            Source::Ui,
        );
        assert_eq!(invalid_field(result), "review.item", "{item:?}");
    }

    assert_eq!(decisions(&db), HashMap::new());
    assert_eq!(rules(&db), vec![]);
}

#[test]
fn deciding_an_unknown_line_is_not_found() {
    let (_dir, db) = imported(&[october()]);

    let result = decide(
        &db,
        "no-such-line",
        Action::Ignore,
        None,
        None,
        false,
        NOW,
        Source::Ui,
    );

    assert!(matches!(result, Err(AppError::NotFound(_))), "{result:?}");
}

#[test]
fn a_hand_decision_replaces_the_decision_a_rule_made_and_undo_brings_it_back() {
    let (dir, db) = imported(&[september()]);
    decide_line(&db, "CAFE LINDE DORNBIRN", 9, Action::Ignore, None, true);
    import_all(&db, &dir, &[october()]);
    let by_rule = decision_of(&db, "CAFE LINDE DORNBIRN", 10).unwrap();
    assert!(!by_rule.by_hand());

    let result = decide_line(
        &db,
        "CAFE LINDE DORNBIRN",
        10,
        Action::Subscription,
        None,
        false,
    );

    assert_eq!(result.tables, ["decisions"]);
    let now = decision_of(&db, "CAFE LINDE DORNBIRN", 10).unwrap();
    assert_eq!((now.kind, now.rule_id), (Action::Subscription, None));

    db.undo().unwrap().unwrap();

    assert_eq!(decision_of(&db, "CAFE LINDE DORNBIRN", 10), Some(by_rule));
}

#[test]
fn always_adds_a_rule_for_the_key_and_applies_it_to_the_other_undecided_lines() {
    let (_dir, db) = imported(&[september(), october()]);

    let result = decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, true);

    assert_eq!(
        result,
        written(
            "Ignore: STREAMFLIX WIEN · rule created",
            &["decisions", "rules"]
        )
    );
    let rule = rules(&db).pop().unwrap();
    assert_eq!(
        (
            rule.tokens(),
            rule.action,
            rule.item_key.clone(),
            rule.position,
            rule.created_at
        ),
        (
            vec!["STREAMFLIX".to_string()],
            Action::Ignore,
            None,
            0,
            NOW + 5
        )
    );
    let earlier = decision_of(&db, "STREAMFLIX WIEN", 9).unwrap();
    assert_eq!(
        (earlier.kind, earlier.rule_id, earlier.decided_at),
        (Action::Ignore, Some(rule.id.clone()), NOW + 5)
    );
    let decided = decision_of(&db, "STREAMFLIX WIEN", 10).unwrap();
    assert_eq!((decided.kind, decided.rule_id), (Action::Ignore, None));
    assert_eq!(decision_of(&db, "LUMEN AI SAN FRANCISCO", 9), None);
}

#[test]
fn an_item_rule_carries_its_item_to_the_lines_it_decides() {
    let (_dir, db) = imported(&[september(), october()]);

    decide_line(
        &db,
        "BUCHHANDLUNG SEITENWEISE",
        10,
        Action::Anv,
        Some("books"),
        true,
    );

    let rule = rules(&db).pop().unwrap();
    assert_eq!(rule.item_key.as_deref(), Some("books"));
    let other = decision_of(&db, "BUCHHANDLUNG SEITENWEISE", 9).unwrap();
    assert_eq!(
        (other.kind, other.item_key, other.rule_id),
        (Action::Anv, Some("books".into()), Some(rule.id))
    );
}

#[test]
fn always_leaves_the_hand_decisions_of_other_lines_alone() {
    let (_dir, db) = imported(&[september(), october()]);
    decide_line(&db, "STREAMFLIX WIEN", 9, Action::Subscription, None, false);
    let before = decision_of(&db, "STREAMFLIX WIEN", 9);

    decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, true);

    assert_eq!(decision_of(&db, "STREAMFLIX WIEN", 9), before);
}

#[test]
fn a_new_rule_runs_after_the_existing_ones() {
    let (_dir, db) = imported(&[october()]);
    decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, true);

    decide_line(&db, "CAFE LINDE DORNBIRN", 10, Action::Ignore, None, true);

    let positions: Vec<(i64, Vec<String>)> = rules(&db)
        .iter()
        .map(|r| (r.position, r.tokens()))
        .collect();
    assert_eq!(
        positions,
        [
            (0, vec!["STREAMFLIX".to_string()]),
            (1, vec!["CAFE".to_string()])
        ]
    );
}

#[test]
fn undoing_a_decision_with_a_rule_reverts_the_decision_the_rule_and_its_lines_in_one_step() {
    let (_dir, db) = imported(&[september(), october()]);

    decide_line(&db, "STREAMFLIX WIEN", 10, Action::Ignore, None, true);
    let undone = db.undo().unwrap().unwrap();

    assert_eq!(undone.label, "Ignore: STREAMFLIX WIEN · rule created");
    assert_eq!(rules(&db), vec![]);
    assert_eq!(decisions(&db), HashMap::new());
    assert_eq!(queue_of(&db).lines.len(), 14);
}

#[test]
fn a_subscription_decision_needs_an_existing_subscription() {
    let (_dir, db) = imported(&[october()]);
    let id = line_id(&db, "STREAMFLIX WIEN", 10);

    for subscription in [
        None,
        Some("no-such-subscription".to_string()),
        Some(String::new()),
    ] {
        let result = decide(
            &db,
            &id,
            Action::Subscription,
            None,
            subscription.clone(),
            false,
            NOW,
            Source::Ui,
        );
        assert_eq!(
            invalid_field(result),
            "review.subscription",
            "{subscription:?}"
        );
    }

    assert_eq!(decisions(&db), HashMap::new());
}

#[test]
fn a_subscription_decision_links_the_line_and_never_creates_a_rule() {
    let (_dir, db) = imported(&[september(), october()]);
    let subscription_id = hand_linked_subscription(&db);

    let result = decide(
        &db,
        &line_id(&db, "STREAMFLIX WIEN", 10),
        Action::Subscription,
        None,
        Some(subscription_id.clone()),
        true,
        NOW + 5,
        Source::Ui,
    )
    .unwrap();

    assert_eq!(
        result,
        written("Subscription: STREAMFLIX WIEN", &["decisions"])
    );
    assert_eq!(
        decision_of(&db, "STREAMFLIX WIEN", 10),
        Some(DecisionRow {
            id: line_id(&db, "STREAMFLIX WIEN", 10),
            kind: Action::Subscription,
            item_key: None,
            subscription_id: Some(subscription_id),
            document_id: None,
            rule_id: None,
            decided_at: NOW + 5,
        })
    );
    assert_eq!(rules(&db), vec![]);
    assert_eq!(decision_of(&db, "STREAMFLIX WIEN", 9), None);
}

#[test]
fn a_decision_other_than_subscription_links_no_subscription() {
    let (_dir, db) = imported(&[october()]);
    let subscription_id = hand_linked_subscription(&db);

    decide(
        &db,
        &line_id(&db, "CAFE LINDE DORNBIRN", 10),
        Action::Ignore,
        None,
        Some(subscription_id),
        false,
        NOW,
        Source::Ui,
    )
    .unwrap();

    let decision = decision_of(&db, "CAFE LINDE DORNBIRN", 10).unwrap();
    assert_eq!(decision.subscription_id, None);
}

#[test]
fn the_queue_offers_only_active_subscriptions() {
    let (_dir, db) = imported(&[october()]);
    let mut ended = subscription_form("Tonfeld Audiobooks", "TONFELD", "9.95");
    ended.active = false;
    ended.ended_on = Some("2026-06-30".into());
    add_subscription(&db, &ended);
    add_subscription(
        &db,
        &subscription_form("Streamflix", "NO SUCH TEXT", "12.99"),
    );

    let options: Vec<_> = queue_of(&db)
        .subscriptions
        .into_iter()
        .map(|s| (s.name, s.letter, s.tone))
        .collect();

    assert_eq!(options, [("Streamflix".to_string(), "S".to_string(), 1)]);
}

#[test]
fn a_line_matching_a_subscription_at_another_amount_asks_whether_the_plan_changed() {
    let (_dir, db) = imported(&[october()]);
    let mut form = subscription_form("Lumen AI", "LUMEN AI", "20.00");
    form.currency = "USD".into();
    let id = add_subscription(&db, &form);

    let queue = queue_of(&db);

    let lumen = &waiting(&queue, "LUMEN AI SAN FRANCISCO")[0];
    assert_eq!(
        lumen.plan_changed,
        Some(PlanChanged {
            subscription_id: id,
            name: "Lumen AI".into(),
            text: "Lumen AI usually charges USD 20.00; this charge is USD 24.00.".into(),
            amount: "24.00".into(),
            currency: "USD".into(),
            date: "2026-10-10".into(),
        })
    );
    assert_eq!(waiting(&queue, "CAFE LINDE DORNBIRN")[0].plan_changed, None);
}

#[test]
fn a_line_at_the_planned_amount_is_decided_and_leaves_the_queue() {
    let (_dir, db) = imported(&[october()]);
    let id = add_subscription(&db, &subscription_form("Streamflix", "STREAMFLIX", "12.99"));

    let decision = decision_of(&db, "STREAMFLIX WIEN", 10).unwrap();

    assert_eq!(
        (decision.kind, decision.subscription_id, decision.rule_id),
        (Action::Subscription, Some(id.clone()), Some(id))
    );
    assert!(waiting(&queue_of(&db), "STREAMFLIX WIEN").is_empty());
}
