use serde_json::json;
use tempfile::TempDir;

use super::store::Action;
use super::tests::{
    NOW, add, assign, household_transfer, ignore, import_statement, insert_rule, rule_row, setup,
};
use super::view::{self, RulesView};
use super::{ItemOption, RuleForm, ensure_builtins};
use crate::date::DateOrder;
use crate::db::{Db, Source};
use crate::settings::NumberFormat;
use crate::statements::import_tests::october;

const DOT: NumberFormat = NumberFormat::DotDecimal;
const COMMA: NumberFormat = NumberFormat::CommaDecimal;

/// The built-in rules, three owner rules (one typed with comma amounts, one assigning `books`, one that never
/// matches) and an imported statement with three transfers to Household.
fn scene() -> (TempDir, Db) {
    let (dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    add(
        &db,
        RuleForm {
            contains: vec!["Cloudbox".into(), "Lumen".into()],
            amount: Some("25,95".into()),
            tolerance: Some("1,00".into()),
            account_id: Some("account-1".into()),
            own_counterparty: true,
            ..ignore(&[])
        },
    );
    add(&db, assign(&["Seitenweise"], "books"));
    add(&db, ignore(&["Nothing"]));
    let mut statement = october();
    for day in [2, 9, 20] {
        statement.lines.push(household_transfer(day));
    }
    statement.lines.sort_by_key(|l| l.value_date);
    import_statement(&db, dir.path(), &statement);
    (dir, db)
}

fn view_of(db: &Db, format: NumberFormat, order: DateOrder) -> RulesView {
    view::rules(db, format, order).unwrap()
}

fn shown(view: &RulesView) -> Vec<(&str, Vec<(&str, &str)>)> {
    view.groups
        .iter()
        .map(|g| {
            (
                g.title.as_str(),
                g.rules
                    .iter()
                    .map(|r| (r.label.as_str(), r.detail.as_str()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn rules_are_grouped_by_action_and_described_in_words_with_their_matches() {
    let (_dir, db) = scene();

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(
        shown(&view),
        vec![
            (
                "Ignore",
                vec![
                    (
                        "Counterparty is one of my accounts",
                        "Transfers · built in · 3 matches · last 20 Oct"
                    ),
                    (
                        "Bank text contains Kontoführung or Debitkarte",
                        "Bank fees · built in · 0 matches"
                    ),
                    (
                        "Bank text contains Cloudbox or Lumen and amount 25.95 ± 1.00 and account is Private and counterparty is one of my accounts",
                        "0 matches"
                    ),
                    ("Bank text contains Nothing", "0 matches"),
                ]
            ),
            (
                "ANV",
                vec![(
                    "Bank text contains Seitenweise",
                    "→ assign item KZ 720 · 1 match · last 14 Oct"
                )]
            ),
        ]
    );
}

#[test]
fn built_in_rules_are_described_in_german_when_the_language_is_german() {
    let (_dir, db) = setup();
    ensure_builtins(&db, NOW, Source::App).unwrap();
    add(&db, ignore(&["Nothing", "Other"]));

    let view = crate::i18n::with_lang(crate::settings::Language::De, || {
        view_of(&db, DOT, DateOrder::DayFirst)
    });

    assert_eq!(
        shown(&view),
        vec![(
            "Ignorieren",
            vec![
                (
                    "Gegenkonto ist eines meiner Konten",
                    "Umbuchungen · integriert · 0 Treffer"
                ),
                (
                    "Banktext enthält Kontoführung oder Debitkarte",
                    "Bankgebühren · integriert · 0 Treffer"
                ),
                ("Banktext enthält Nothing oder Other", "0 Treffer"),
            ]
        )]
    );
}

#[test]
fn amounts_and_dates_follow_the_number_format_and_the_date_order() {
    let (_dir, db) = scene();

    let view = view_of(&db, COMMA, DateOrder::MonthFirst);

    let shown = shown(&view);
    assert_eq!(
        shown[0].1[0],
        (
            "Counterparty is one of my accounts",
            "Transfers · built in · 3 matches · last Oct 20"
        )
    );
    assert_eq!(
        shown[0].1[2].0,
        "Bank text contains Cloudbox or Lumen and amount 25,95 ± 1,00 and account is Private and counterparty is one of my accounts"
    );
    assert_eq!(
        shown[1].1[0].1,
        "→ assign item KZ 720 · 1 match · last Oct 14"
    );
}

#[test]
fn a_rules_form_shows_its_amounts_in_the_users_number_format() {
    let (_dir, db) = scene();
    let typed_with_a_comma = |view: &RulesView| view.groups[0].rules[2].form.clone();

    let dot = typed_with_a_comma(&view_of(&db, DOT, DateOrder::DayFirst));
    let comma = typed_with_a_comma(&view_of(&db, COMMA, DateOrder::DayFirst));

    let expected = RuleForm {
        contains: vec!["Cloudbox".into(), "Lumen".into()],
        amount: Some("25.95".into()),
        tolerance: Some("1.00".into()),
        account_id: Some("account-1".into()),
        own_counterparty: true,
        action: Action::Ignore,
        item_key: None,
    };
    assert_eq!(dot, expected);
    assert_eq!(
        comma,
        RuleForm {
            amount: Some("25,95".into()),
            tolerance: Some("1,00".into()),
            ..expected
        }
    );
}

#[test]
fn a_rule_without_an_amount_or_tolerance_has_none_in_its_form() {
    let (_dir, db) = scene();

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(
        view.groups[1].rules[0].form,
        assign(&["Seitenweise"], "books")
    );
    assert_eq!(
        view.groups[0].rules[0].form,
        RuleForm {
            own_counterparty: true,
            ..ignore(&[])
        }
    );
}

#[test]
fn built_in_rules_are_flagged_and_keep_their_ids() {
    let (_dir, db) = scene();

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    let flags: Vec<(String, bool)> = view.groups[0]
        .rules
        .iter()
        .map(|r| (r.id.clone(), r.builtin))
        .collect();
    assert_eq!(flags[0], ("builtin-transfers".to_string(), true));
    assert_eq!(flags[1], ("builtin-bank-fees".to_string(), true));
    assert!(!flags[2].1 && !flags[3].1);
}

#[test]
fn the_items_list_offers_the_visible_items_with_their_labels() {
    let (_dir, db) = scene();

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(
        view.items,
        vec![ItemOption {
            key: "books".into(),
            label: "KZ 720 · Fachliteratur".into(),
            kz: "720".into(),
        }]
    );
}

#[test]
fn groups_come_in_the_order_ignore_subscription_item_and_empty_ones_are_left_out() {
    let (_dir, db) = setup();
    assert!(
        view_of(&db, DOT, DateOrder::DayFirst).groups.is_empty(),
        "no rules, no groups"
    );

    insert_rule(&db, &rule_row("rule-item", 0, &["a"], Action::Anv));
    assert_eq!(
        shown(&view_of(&db, DOT, DateOrder::DayFirst))
            .iter()
            .map(|g| g.0)
            .collect::<Vec<_>>(),
        vec!["ANV"]
    );

    insert_rule(&db, &rule_row("rule-sub", 1, &["b"], Action::Subscription));
    insert_rule(&db, &rule_row("rule-ignore", 2, &["c"], Action::Ignore));
    let view = view_of(&db, DOT, DateOrder::DayFirst);
    assert_eq!(
        shown(&view).iter().map(|g| g.0).collect::<Vec<_>>(),
        vec!["Ignore", "Subscription", "ANV"]
    );
}

#[test]
fn a_rule_naming_a_deleted_account_or_a_retired_item_still_reads_clearly() {
    let (_dir, db) = setup();
    let mut gone = rule_row("rule-gone", 0, &["x"], Action::Ignore);
    gone.account_id = Some("account-9".into());
    let mut retired = rule_row("rule-retired", 1, &["y"], Action::Anv);
    retired.item_key = Some("retired".into());
    insert_rule(&db, &gone);
    insert_rule(&db, &retired);

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(
        shown(&view),
        vec![
            (
                "Ignore",
                vec![(
                    "Bank text contains x and account is a deleted account",
                    "0 matches"
                )]
            ),
            (
                "ANV",
                vec![("Bank text contains y", "→ assign item retired · 0 matches")]
            ),
        ]
    );
}

#[test]
fn a_rule_with_only_an_account_starts_its_label_with_a_capital() {
    let (_dir, db) = setup();
    let mut rule = rule_row("rule-account", 0, &[], Action::Ignore);
    rule.account_id = Some("account-2".into());
    insert_rule(&db, &rule);

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(view.groups[0].rules[0].label, "Account is Household");
}

#[test]
fn a_rules_view_serializes_with_its_groups_and_items() {
    let (_dir, db) = setup();
    insert_rule(&db, &rule_row("rule-item", 0, &["a"], Action::Anv));

    let view = view_of(&db, DOT, DateOrder::DayFirst);

    assert_eq!(
        serde_json::to_value(&view).unwrap(),
        json!({
            "groups": [{
                "title": "ANV",
                "rules": [{
                    "id": "rule-item",
                    "label": "Bank text contains a",
                    "detail": "→ assign item KZ 720 · 0 matches",
                    "builtin": false,
                    "form": {
                        "contains": ["a"],
                        "amount": null,
                        "tolerance": null,
                        "accountId": null,
                        "ownCounterparty": false,
                        "action": "anv",
                        "itemKey": "books"
                    }
                }]
            }],
            "items": [{ "key": "books", "label": "KZ 720 · Fachliteratur", "kz": "720" }]
        })
    );
}
