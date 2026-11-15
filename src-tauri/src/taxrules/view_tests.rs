use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::check;
use super::view::*;
use crate::date::DateOrder;
use crate::settings::NumberFormat;
use crate::taxyears::{RulesSource, TaxYearRow, YearState};
use crate::test_support::rules::{minimal_rules, rich_rules, rules_text, set};

fn view_of(rules: &Value, mutate: impl FnOnce(&mut TaxYearRow)) -> RulesYearView {
    let text = rules_text(rules);
    let mut row = TaxYearRow::new(2026, "2026 v1".into(), text.clone(), 1);
    mutate(&mut row);
    let ruleset = check::read(&text, &BTreeSet::new()).unwrap();
    year_view(
        &row,
        &ruleset,
        NumberFormat::DotDecimal,
        DateOrder::DayFirst,
    )
}

fn view(rules: &Value) -> RulesYearView {
    view_of(rules, |_| {})
}

fn group<'a>(view: &'a RulesYearView, title: &str) -> &'a RulesGroup {
    view.groups
        .iter()
        .find(|g| g.title == title)
        .unwrap_or_else(|| panic!("no group {title}"))
}

fn field(path: &str, label: &str, value: &str, unit: &str, kind: FieldKind) -> RulesField {
    RulesField {
        path: path.into(),
        label: label.into(),
        value: value.into(),
        unit: Some(unit.into()),
        kind,
    }
}

#[test]
fn groups_come_in_the_order_general_useful_life_kilometergeld_then_one_per_editable_item() {
    let titles: Vec<String> = view(&rich_rules(2026))
        .groups
        .into_iter()
        .map(|g| g.title)
        .collect();

    assert_eq!(
        titles,
        [
            "General",
            "Useful life",
            "Kilometergeld",
            "Books · KZ 720",
            "Dues · KZ 723"
        ]
    );
}

#[test]
fn the_general_group_holds_the_purchase_limit_and_the_half_year_fact() {
    let view = view(&rich_rules(2026));

    assert_eq!(
        group(&view, "General"),
        &RulesGroup {
            title: "General".into(),
            fields: vec![field(
                "settings/gwgLimit",
                "Purchases above this are spread over their useful life",
                "1,000.00",
                "€",
                FieldKind::Amount
            )],
            facts: vec!["Bought from 1 Jul on: the first year counts half".into()],
        }
    );
}

#[test]
fn the_flat_allowance_comes_first_when_the_rules_have_one() {
    let mut rules = rich_rules(2026);
    set(
        &mut rules,
        "/settings/werbungskostenpauschale",
        json!(132.00),
    );

    let view = view(&rules);

    assert_eq!(
        group(&view, "General").fields[0],
        field(
            "settings/werbungskostenpauschale",
            "Flat allowance the tax office grants anyway",
            "132.00",
            "€",
            FieldKind::Amount
        )
    );
}

#[test]
fn amounts_follow_the_number_format() {
    let text = rules_text(&rich_rules(2026));
    let row = TaxYearRow::new(2026, "2026 v1".into(), text.clone(), 1);
    let ruleset = check::read(&text, &BTreeSet::new()).unwrap();

    let view = year_view(
        &row,
        &ruleset,
        NumberFormat::CommaDecimal,
        DateOrder::DayFirst,
    );

    assert_eq!(group(&view, "General").fields[0].value, "1.000,00");
}

#[test]
fn useful_life_lists_each_asset_class_in_years() {
    let view = view(&rich_rules(2026));

    assert_eq!(
        group(&view, "Useful life").fields,
        vec![field(
            "usefulLife/computer",
            "Computers",
            "3",
            "years",
            FieldKind::Count
        )]
    );
}

#[test]
fn kilometergeld_lists_each_rate_by_vehicle_and_range() {
    let view = view(&rich_rules(2026));

    assert_eq!(
        group(&view, "Kilometergeld").fields,
        vec![field(
            "rates/kilometergeld/0/perKm",
            "Car, 1 Jan – 31 Dec",
            "0.42",
            "€ / km",
            FieldKind::Amount
        )]
    );
}

#[test]
fn an_item_group_lists_its_private_share_and_each_estimate_value() {
    let view = view(&rich_rules(2026));

    assert_eq!(
        group(&view, "Books · KZ 720").fields,
        vec![
            field(
                "items/books/privateShare",
                "Private share, preset for new documents",
                "20",
                "%",
                FieldKind::Percent
            ),
            field(
                "items/books/estimate/0/amount",
                "Counted at most",
                "300.00",
                "€",
                FieldKind::Amount
            ),
            field(
                "items/books/estimate/1/min",
                "Needed at least",
                "5",
                "days",
                FieldKind::Count
            ),
            field(
                "items/books/estimate/2/perUnit",
                "The tax office subtracts per days",
                "3.00",
                "€",
                FieldKind::Amount
            ),
            field(
                "items/books/estimate/2/maxUnits",
                "Counted at most",
                "100",
                "days",
                FieldKind::Count
            ),
        ]
    );
}

#[test]
fn a_cap_that_carries_over_says_so_in_its_label() {
    let view = view(&rich_rules(2026));

    assert_eq!(
        group(&view, "Dues · KZ 723").fields,
        vec![field(
            "items/dues/estimate/0/amount",
            "Counted per year at most; the rest in later years",
            "120.00",
            "€",
            FieldKind::Amount
        )]
    );
}

#[test]
fn hidden_items_and_items_without_editable_values_get_no_group() {
    let titles: Vec<String> = view(&rich_rules(2026))
        .groups
        .into_iter()
        .map(|g| g.title)
        .collect();

    assert!(!titles.iter().any(|t| t.starts_with("Notes")), "{titles:?}");
    assert!(!titles.iter().any(|t| t.starts_with("Plain")), "{titles:?}");
}

#[test]
fn rules_without_useful_life_or_rates_have_only_the_general_group() {
    let titles: Vec<String> = view(&minimal_rules(2026))
        .groups
        .into_iter()
        .map(|g| g.title)
        .collect();

    assert_eq!(titles, ["General"]);
}

#[test]
fn only_an_open_year_is_editable() {
    for (state, closed_at, expected) in [
        (YearState::Open, None, true),
        (YearState::Closed, Some(5), false),
    ] {
        let view = view_of(&rich_rules(2026), |row| {
            row.state = state;
            row.closed_at = closed_at;
        });
        assert_eq!(view.editable, expected, "{state:?}");
    }
}

#[test]
fn the_view_names_the_year_its_source_and_where_the_rules_came_from() {
    let mut rules = rich_rules(2026);
    set(&mut rules, "/basedOn", json!("2026 v0"));
    set(&mut rules, "/notes", json!("Check the limits"));
    set(
        &mut rules,
        "/sources",
        json!([{ "label": "Example guide", "url": "https://example.com" }]),
    );

    let view = view_of(&rules, |row| row.rules_source = RulesSource::Own);

    assert_eq!(view.year, 2026);
    assert_eq!(view.source, RulesSource::Own);
    assert_eq!(view.ruleset, "2026 v1");
    assert_eq!(view.based_on.as_deref(), Some("2026 v0"));
    assert_eq!(view.notes.as_deref(), Some("Check the limits"));
    assert_eq!(view.sources, ["Example guide"]);
}

#[test]
fn a_field_path_decides_its_kind() {
    for (path, expected) in [
        ("settings/gwgLimit", FieldKind::Amount),
        ("rates/kilometergeld/0/perKm", FieldKind::Amount),
        ("usefulLife/computer", FieldKind::Count),
        ("items/books/privateShare", FieldKind::Percent),
        ("items/books/estimate/0/amount", FieldKind::Amount),
        ("items/books/estimate/1/min", FieldKind::Count),
        ("items/books/estimate/2/perUnit", FieldKind::Amount),
        ("items/books/estimate/2/maxUnits", FieldKind::Count),
    ] {
        assert_eq!(kind_of(path), expected, "{path}");
    }
}

#[test]
fn a_new_value_reads_as_money_percent_or_count_with_its_unit() {
    for (kind, value, unit, format, expected) in [
        (
            FieldKind::Amount,
            42,
            None,
            NumberFormat::DotDecimal,
            "€0.42",
        ),
        (
            FieldKind::Amount,
            123_450,
            None,
            NumberFormat::CommaDecimal,
            "€1.234,50",
        ),
        (
            FieldKind::Percent,
            40,
            Some("%"),
            NumberFormat::DotDecimal,
            "40 %",
        ),
        (
            FieldKind::Count,
            5,
            Some("years"),
            NumberFormat::DotDecimal,
            "5 years",
        ),
        (FieldKind::Count, 5, None, NumberFormat::DotDecimal, "5"),
    ] {
        assert_eq!(
            shown_value(kind, value, unit, format),
            expected,
            "{kind:?} {value}"
        );
    }
}

#[test]
fn the_view_serializes_with_camel_case_names_and_snake_case_kinds() {
    let view = view(&minimal_rules(2026));

    assert_eq!(
        serde_json::to_value(&view).unwrap(),
        json!({
            "year": 2026,
            "editable": true,
            "source": "builtin",
            "ruleset": "2026 v1",
            "basedOn": null,
            "groups": [{
                "title": "General",
                "fields": [{
                    "path": "settings/gwgLimit",
                    "label": "Purchases above this are spread over their useful life",
                    "value": "1,000.00",
                    "unit": "€",
                    "kind": "amount"
                }],
                "facts": ["Bought from 1 Jul on: the first year counts half"]
            }],
            "notes": null,
            "sources": []
        })
    );
}
