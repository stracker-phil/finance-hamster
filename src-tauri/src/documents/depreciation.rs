//! Depreciation (AfA): a purchase whose counted amount is above the purchase year's GWG limit, on an item with
//! a depreciation class, spreads over the class's useful life (changeable per document). Bought on or after
//! `halfYearFrom`, the first and the last slice count half, so the schedule runs one year longer.

use chrono::{Datelike, NaiveDate};

use crate::taxrules::model::{Item, Ruleset};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    pub year: i32,
    pub cents: i64,
    /// A half-year slice: the first or the last of a purchase in the second half of the year.
    pub half: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// Counts in full in the purchase year: at or below the GWG limit, or an item without a depreciation class.
    Full,
    Spread {
        asset_class: String,
        rule_years: u32,
        /// The years applied: the document's own, else the rule's.
        years: u32,
        half_year: bool,
        slices: Vec<Slice>,
    },
}

/// Bought on or after `MM-DD` of its year.
fn second_half(day: NaiveDate, half_year_from: &str) -> bool {
    let mut parts = half_year_from
        .split('-')
        .filter_map(|p| p.parse::<u32>().ok());
    match (parts.next(), parts.next()) {
        (Some(month), Some(from)) => (day.month(), day.day()) >= (month, from),
        _ => false,
    }
}

/// Spreads `cost` cents over `years`; the last slice takes what rounding leaves.
fn slices(cost: i64, start: i32, years: u32, half_year: bool) -> Vec<Slice> {
    let annual = cost / i64::from(years);
    let count = years + u32::from(half_year);
    let mut out: Vec<Slice> = (0..count)
        .map(|i| {
            let half = half_year && (i == 0 || i == count - 1);
            Slice {
                year: start + i as i32,
                cents: match (half, i == 0) {
                    (true, true) => annual / 2,
                    (true, false) => 0,
                    (false, _) => annual,
                },
                half,
            }
        })
        .collect();
    let counted: i64 = out.iter().map(|s| s.cents).sum();
    if let Some(last) = out.last_mut() {
        last.cents += cost - counted;
    }
    out
}

/// What `cost` (after the work share) of a purchase on `day` counts over the years.
pub fn schedule(
    cost: i64,
    day: NaiveDate,
    item: &Item,
    ruleset: &Ruleset,
    own_years: Option<u32>,
) -> Schedule {
    let Some(depreciation) = &item.depreciation else {
        return Schedule::Full;
    };
    if cost <= ruleset.settings.gwg_limit.0 {
        return Schedule::Full;
    }
    let rule_years = ruleset
        .useful_life
        .get(&depreciation.asset_class)
        .copied()
        .unwrap_or(1)
        .max(1);
    let years = own_years.unwrap_or(rule_years).max(1);
    let half_year = second_half(day, &ruleset.settings.half_year_from);
    Schedule::Spread {
        asset_class: depreciation.asset_class.clone(),
        rule_years,
        years,
        half_year,
        slices: slices(cost, day.year(), years, half_year),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rules::{minimal_rules, set};
    use serde_json::json;

    fn ruleset(gwg: f64) -> Ruleset {
        let mut rules = minimal_rules(2026);
        set(&mut rules, "/settings/gwgLimit", json!(gwg));
        set(&mut rules, "/usefulLife", json!({ "computer": 3 }));
        set(
            &mut rules,
            "/items/0/depreciation",
            json!({ "assetClass": "computer" }),
        );
        serde_json::from_value(rules).unwrap()
    }

    fn day(month: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, d).unwrap()
    }

    fn slice(year: i32, cents: i64, half: bool) -> Slice {
        Slice { year, cents, half }
    }

    fn spread_of(cost: i64, on: NaiveDate, own: Option<u32>) -> Schedule {
        let rules = ruleset(1_000.0);
        schedule(cost, on, &rules.items[0], &rules, own)
    }

    fn slices_of(schedule: Schedule) -> Vec<Slice> {
        match schedule {
            Schedule::Spread { slices, .. } => slices,
            Schedule::Full => panic!("expected a spread schedule"),
        }
    }

    #[test]
    fn a_purchase_counts_fully_up_to_the_gwg_limit_and_spreads_above_it() {
        for (gwg, limit) in [(400.0, 40_000), (800.0, 80_000), (1_000.0, 100_000)] {
            let rules = ruleset(gwg);
            for (cost, spreads) in [
                (limit - 1, false),
                (limit, false),
                (limit + 1, true),
                (limit * 2, true),
            ] {
                let result = schedule(cost, day(3, 10), &rules.items[0], &rules, None);

                assert_eq!(
                    matches!(result, Schedule::Spread { .. }),
                    spreads,
                    "gwg {gwg}, cost {cost}"
                );
                if !spreads {
                    assert_eq!(result, Schedule::Full, "gwg {gwg}, cost {cost}");
                }
            }
        }
    }

    #[test]
    fn an_item_without_a_depreciation_class_counts_fully_whatever_the_cost() {
        let rules = ruleset(400.0);
        let item = Item {
            depreciation: None,
            ..rules.items[0].clone()
        };

        assert_eq!(
            schedule(5_000_000, day(3, 10), &item, &rules, Some(5)),
            Schedule::Full
        );
    }

    #[test]
    fn a_purchase_before_the_half_year_date_spreads_into_equal_slices_from_the_purchase_year() {
        assert_eq!(
            spread_of(150_000, day(6, 30), None),
            Schedule::Spread {
                asset_class: "computer".into(),
                rule_years: 3,
                years: 3,
                half_year: false,
                slices: vec![
                    slice(2026, 50_000, false),
                    slice(2027, 50_000, false),
                    slice(2028, 50_000, false),
                ],
            }
        );
    }

    #[test]
    fn the_last_slice_takes_the_rounding_remainder() {
        assert_eq!(
            slices_of(spread_of(100_001, day(3, 10), None)),
            [
                slice(2026, 33_333, false),
                slice(2027, 33_333, false),
                slice(2028, 33_335, false),
            ]
        );
    }

    #[test]
    fn a_purchase_on_the_half_year_date_gets_half_first_and_last_slices_over_one_more_year() {
        assert_eq!(
            spread_of(150_000, day(7, 1), None),
            Schedule::Spread {
                asset_class: "computer".into(),
                rule_years: 3,
                years: 3,
                half_year: true,
                slices: vec![
                    slice(2026, 25_000, true),
                    slice(2027, 50_000, false),
                    slice(2028, 50_000, false),
                    slice(2029, 25_000, true),
                ],
            }
        );
    }

    #[test]
    fn half_year_slices_give_the_remainder_to_the_last_half_slice() {
        assert_eq!(
            slices_of(spread_of(100_001, day(12, 31), None)),
            [
                slice(2026, 16_666, true),
                slice(2027, 33_333, false),
                slice(2028, 33_333, false),
                slice(2029, 16_669, true),
            ]
        );
    }

    #[test]
    fn slices_always_add_up_to_the_cost() {
        for cost in [100_001, 100_002, 123_457, 999_999, 150_000] {
            for years in [1, 2, 3, 7, 50] {
                for on in [day(3, 10), day(9, 10)] {
                    let total: i64 = slices_of(spread_of(cost, on, Some(years)))
                        .iter()
                        .map(|s| s.cents)
                        .sum();

                    assert_eq!(total, cost, "{cost} over {years} years from {on}");
                }
            }
        }
    }

    #[test]
    fn the_documents_own_years_replace_the_rules_but_the_rules_years_stay_known() {
        let Schedule::Spread {
            rule_years,
            years,
            slices,
            ..
        } = spread_of(150_000, day(3, 10), Some(5))
        else {
            panic!("expected a spread schedule");
        };

        assert_eq!((rule_years, years), (3, 5));
        assert_eq!(slices.len(), 5);
        assert_eq!(slices[0], slice(2026, 30_000, false));
    }

    #[test]
    fn an_own_useful_life_of_zero_counts_as_one_year() {
        let Schedule::Spread { years, slices, .. } = spread_of(150_000, day(3, 10), Some(0)) else {
            panic!("expected a spread schedule");
        };

        assert_eq!(years, 1);
        assert_eq!(slices, [slice(2026, 150_000, false)]);
    }

    #[test]
    fn an_asset_class_the_rules_do_not_list_spreads_over_one_year() {
        let mut rules = ruleset(1_000.0);
        rules.useful_life.clear();

        let Schedule::Spread {
            rule_years, years, ..
        } = schedule(150_000, day(3, 10), &rules.items[0], &rules, None)
        else {
            panic!("expected a spread schedule");
        };

        assert_eq!((rule_years, years), (1, 1));
    }

    #[test]
    fn a_rules_value_of_zero_years_counts_as_one_year() {
        let mut rules = ruleset(1_000.0);
        rules.useful_life.insert("computer".into(), 0);

        let Schedule::Spread { years, .. } =
            schedule(150_000, day(3, 10), &rules.items[0], &rules, None)
        else {
            panic!("expected a spread schedule");
        };

        assert_eq!(years, 1);
    }
}
