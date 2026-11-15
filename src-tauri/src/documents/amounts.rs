//! What a payment counts as: the work share of the item, the ANV amount and the GWG hint, in words.

use serde::Serialize;
use ts_rs::TS;

use crate::money::{Money, format_money};
use crate::settings::NumberFormat;
use crate::taxrules::model::Ruleset;

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
    /// `€121.23 is below the GWG limit (€1,000.00): counts fully in 2026.`
    pub gwg: Option<String>,
}

pub fn estimate(
    ruleset: Option<&Ruleset>,
    paid_cents: i64,
    item_key: Option<&str>,
    work_share: i64,
    year: i32,
    format: NumberFormat,
) -> Estimate {
    let eur = |cents: i64| format_money(&Money::eur(cents), format);
    let item = ruleset
        .zip(item_key)
        .and_then(|(r, key)| r.items.iter().find(|i| i.key == key));
    let share_hint = item
        .and_then(|i| i.private_share)
        .map(|p| format!("from the item: {p} % private"));
    let anv = anv_cents(paid_cents, work_share);
    let gwg = ruleset.zip(item).map(|(r, i)| {
        let limit = r.settings.gwg_limit.0;
        if anv <= limit {
            format!(
                "{} is below the GWG limit ({}): counts fully in {year}.",
                eur(anv),
                eur(limit)
            )
        } else if i.depreciation.is_none() {
            format!(
                "{} counts fully in {year}: this item is never depreciated.",
                eur(anv)
            )
        } else {
            format!(
                "{} is above the GWG limit ({}): it is depreciated over several years.",
                eur(anv),
                eur(limit)
            )
        }
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
        estimate(Some(&ruleset()), paid, item, share, 2026, format)
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
                gwg: Some(
                    "€72.74 is below the GWG limit (€1,000.00): counts fully in 2026.".into()
                ),
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
            estimate(
                None,
                12_123,
                Some("phone"),
                100,
                2026,
                NumberFormat::DotDecimal,
            ),
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
        assert_eq!(
            estimate.gwg.as_deref(),
            Some("€20.00 is below the GWG limit (€1,000.00): counts fully in 2026.")
        );
    }

    #[test]
    fn a_depreciable_item_above_the_limit_is_spread_over_several_years() {
        let estimate = estimate_of(Some("laptop"), 150_000, 100, NumberFormat::DotDecimal);

        assert_eq!(
            estimate.gwg.as_deref(),
            Some(
                "€1,500.00 is above the GWG limit (€1,000.00): it is depreciated over several years."
            )
        );
    }

    #[test]
    fn a_depreciable_item_at_the_limit_counts_fully() {
        let estimate = estimate_of(Some("laptop"), 100_000, 100, NumberFormat::DotDecimal);

        assert_eq!(
            estimate.gwg.as_deref(),
            Some("€1,000.00 is below the GWG limit (€1,000.00): counts fully in 2026.")
        );
    }

    #[test]
    fn an_item_without_depreciation_above_the_limit_counts_fully_as_it_is_never_depreciated() {
        let estimate = estimate_of(Some("books"), 150_000, 100, NumberFormat::DotDecimal);

        assert_eq!(
            estimate.gwg.as_deref(),
            Some("€1,500.00 counts fully in 2026: this item is never depreciated.")
        );
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
                "gwg": "€72.74 is below the GWG limit (€1,000.00): counts fully in 2026."
            })
        );
    }
}
