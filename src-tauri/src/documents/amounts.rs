//! What a payment counts as: the work share of the item, the ANV amount and the GWG hint, in words.

use serde::Serialize;
use ts_rs::TS;

use crate::money::{Money, format_money};
use crate::settings::NumberFormat;
use crate::taxrules::model::Ruleset;
use crate::tr;

/// The share of the payment that counts: 100 % less the item's `privateShare`.
pub fn default_work_share(ruleset: Option<&Ruleset>, item_key: Option<&str>) -> i64 {
    let private = ruleset
        .zip(item_key)
        .and_then(|(r, key)| r.items.iter().find(|i| i.key == key))
        .and_then(|i| i.private_share)
        .unwrap_or(0);
    100 - i64::from(private.min(100))
}

/// The ANV amount of `paid_cents` at `work_share` percent, rounded half away from zero; a refund stays negative.
pub fn anv_cents(paid_cents: i64, work_share: i64) -> i64 {
    paid_cents.signum() * ((paid_cents.abs() * work_share + 50) / 100)
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Estimate {
    pub work_share: i64,
    /// `from the item: 40 % private`.
    pub share_hint: Option<String>,
    /// `€72.74`.
    pub anv: String,
    /// `€121.23 × 60 %`.
    pub formula: String,
    /// `Counts over several years: it cost more than €1,000.00.`; None when it counts fully this year.
    pub gwg: Option<String>,
}

pub fn estimate(
    ruleset: Option<&Ruleset>,
    paid_cents: i64,
    item_key: Option<&str>,
    work_share: i64,
    format: NumberFormat,
) -> Estimate {
    let eur = |cents: i64| format_money(&Money::eur(cents), format);
    let item = ruleset
        .zip(item_key)
        .and_then(|(r, key)| r.items.iter().find(|i| i.key == key));
    let share_hint = item
        .and_then(|i| i.private_share)
        .map(|p| tr!("from the item: {p} % private", "vom Posten: {p} % privat"));
    let anv = anv_cents(paid_cents, work_share);
    // Said only when the amount doesn't simply count in full this year.
    let gwg = ruleset.zip(item).and_then(|(r, i)| {
        let limit = r.settings.gwg_limit.0;
        (anv > limit && i.depreciation.is_some()).then(|| {
            tr!(
                "Counts over several years: it cost more than {}.",
                "Zählt über mehrere Jahre: Es hat mehr als {} gekostet.",
                eur(limit)
            )
        })
    });
    Estimate {
        work_share,
        share_hint,
        anv: eur(anv),
        formula: format!("{} × {work_share} %", eur(paid_cents)),
        gwg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::test_support::rules::{minimal_rules, set};

    fn ruleset() -> Ruleset {
        let mut rules = minimal_rules(2026);
        set(
            &mut rules,
            "/items",
            json!([
                { "key": "books", "kz": "720", "name": "Books", "section": "main", "kind": "documents" },
                { "key": "phone", "kz": "721", "name": "Phone", "section": "main", "kind": "documents", "privateShare": 40 },
                { "key": "gift", "kz": "722", "name": "Gift", "section": "main", "kind": "documents", "privateShare": 150 },
                {
                    "key": "laptop", "kz": "169", "name": "Laptop", "section": "main", "kind": "documents",
                    "depreciation": { "assetClass": "computer" }
                }
            ]),
        );
        serde_json::from_value(rules).unwrap()
    }

    fn estimate_of(item: Option<&str>, paid: i64, share: i64, format: NumberFormat) -> Estimate {
        estimate(Some(&ruleset()), paid, item, share, format)
    }

    #[test]
    fn the_work_share_is_one_hundred_less_the_items_private_share() {
        let rules = ruleset();
        for (ruleset, item, expected) in [
            (Some(&rules), Some("phone"), 60),
            (Some(&rules), Some("books"), 100),
            (Some(&rules), Some("unknown"), 100),
            (Some(&rules), Some("gift"), 0),
            (Some(&rules), None, 100),
            (None, Some("phone"), 100),
            (None, None, 100),
        ] {
            assert_eq!(default_work_share(ruleset, item), expected, "{item:?}");
        }
    }

    #[test]
    fn the_anv_amount_rounds_half_away_from_zero_and_keeps_the_sign() {
        for (paid, share, expected) in [
            (12_123, 60, 7_274),
            (-12_123, 60, -7_274),
            (-1_000, 60, -600),
            (-1_001, 50, -501),
            (1_001, 50, 501),
            (-1, 50, -1),
            (-1, 49, 0),
            (1_000, 100, 1_000),
            (1, 50, 1),
            (1, 49, 0),
            (999, 33, 330),
            (0, 50, 0),
            (5_000, 0, 0),
        ] {
            assert_eq!(anv_cents(paid, share), expected, "{paid} at {share} %");
        }
    }

    #[test]
    fn an_estimate_names_the_share_the_amount_and_the_formula() {
        assert_eq!(
            estimate_of(Some("phone"), 12_123, 60, NumberFormat::DotDecimal),
            Estimate {
                work_share: 60,
                share_hint: Some("from the item: 40 % private".into()),
                anv: "€72.74".into(),
                formula: "€121.23 × 60 %".into(),
                gwg: None,
            }
        );
    }

    #[test]
    fn an_estimate_reads_in_the_chosen_number_format() {
        let estimate = estimate_of(Some("phone"), 12_123, 60, NumberFormat::CommaDecimal);

        assert_eq!(estimate.anv, "€72,74");
        assert_eq!(estimate.formula, "€121,23 × 60 %");
    }

    #[test]
    fn an_estimate_of_a_net_refund_keeps_the_minus_sign() {
        let estimate = estimate_of(Some("phone"), -12_123, 60, NumberFormat::DotDecimal);

        assert_eq!(estimate.anv, "\u{2212}€72.74");
        assert_eq!(estimate.formula, "\u{2212}€121.23 × 60 %");
    }

    #[test]
    fn without_a_ruleset_or_an_item_there_is_no_hint_and_no_gwg_note() {
        for estimate in [
            estimate(None, 12_123, Some("phone"), 100, NumberFormat::DotDecimal),
            estimate_of(None, 12_123, 100, NumberFormat::DotDecimal),
            estimate_of(Some("unknown"), 12_123, 100, NumberFormat::DotDecimal),
        ] {
            assert_eq!(estimate.share_hint, None);
            assert_eq!(estimate.gwg, None);
            assert_eq!(estimate.anv, "€121.23");
        }
    }

    #[test]
    fn an_item_without_a_private_share_gives_no_share_hint() {
        let estimate = estimate_of(Some("books"), 2_000, 100, NumberFormat::DotDecimal);

        assert_eq!(estimate.share_hint, None);
        assert_eq!(estimate.gwg, None);
    }

    #[test]
    fn a_depreciable_item_above_the_limit_says_it_counts_over_several_years() {
        let estimate = estimate_of(Some("laptop"), 150_000, 100, NumberFormat::DotDecimal);

        assert_eq!(
            estimate.gwg.as_deref(),
            Some("Counts over several years: it cost more than €1,000.00.")
        );
    }

    #[test]
    fn the_several_years_note_reads_in_german() {
        let estimate = crate::i18n::with_lang(crate::settings::Language::De, || {
            estimate_of(Some("laptop"), 150_000, 100, NumberFormat::CommaDecimal)
        });

        assert_eq!(
            estimate.gwg.as_deref(),
            Some("Zählt über mehrere Jahre: Es hat mehr als €1.000,00 gekostet.")
        );
    }

    #[test]
    fn a_depreciable_item_at_or_below_the_limit_has_no_note() {
        for paid in [100_000, 500, 0] {
            let estimate = estimate_of(Some("laptop"), paid, 100, NumberFormat::DotDecimal);

            assert_eq!(estimate.gwg, None, "{paid}");
        }
    }

    #[test]
    fn an_item_without_depreciation_has_no_note_even_above_the_limit() {
        let estimate = estimate_of(Some("books"), 150_000, 100, NumberFormat::DotDecimal);

        assert_eq!(estimate.gwg, None);
    }

    #[test]
    fn the_limit_applies_to_the_counted_amount_not_the_paid_amount() {
        let estimate = estimate_of(Some("laptop"), 150_000, 60, NumberFormat::DotDecimal);

        assert_eq!(estimate.gwg, None);
    }

    #[test]
    fn an_estimate_travels_in_camel_case() {
        let estimate = estimate_of(Some("phone"), 12_123, 60, NumberFormat::DotDecimal);

        assert_eq!(
            serde_json::to_value(&estimate).unwrap(),
            json!({
                "workShare": 60,
                "shareHint": "from the item: 40 % private",
                "anv": "€72.74",
                "formula": "€121.23 × 60 %",
                "gwg": null
            })
        );
    }
}
