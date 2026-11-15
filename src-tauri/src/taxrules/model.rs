//! A rules file as typed data (`rules.schema.json`). Unknown fields and rule types fail to parse, so a typo
//! never changes a result silently.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Whole euro cents read from a JSON amount (`1000.00`); negative values and fractions of a cent are refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cents(pub i64);

impl<'de> Deserialize<'de> for Cents {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        let cents = (value * 100.0).round();
        if value < 0.0 || (value * 100.0 - cents).abs() > 1e-6 || cents > i64::MAX as f64 {
            return Err(serde::de::Error::custom(crate::tr!(
                "{value} isn't an amount of at least 0 with at most two decimals",
                "{value} ist kein Betrag ab 0 mit höchstens zwei Nachkommastellen"
            )));
        }
        Ok(Cents(cents as i64))
    }
}

impl Serialize for Cents {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0 as f64 / 100.0)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ruleset {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub ruleset: String,
    pub year: i32,
    #[serde(default)]
    pub based_on: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceRef>,
    pub settings: YearSettings,
    #[serde(default)]
    pub useful_life: BTreeMap<String, u32>,
    #[serde(default)]
    pub rates: Rates,
    #[serde(default)]
    pub questions: Vec<Question>,
    pub sections: Vec<Section>,
    pub items: Vec<Item>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub label: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct YearSettings {
    pub gwg_limit: Cents,
    /// `MM-DD`.
    pub half_year_from: String,
    #[serde(default)]
    pub werbungskostenpauschale: Option<Cents>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rates {
    #[serde(default)]
    pub kilometergeld: Vec<KmRate>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Vehicle {
    Car,
    Motorbike,
    Bicycle,
}

impl Vehicle {
    pub fn as_str(self) -> &'static str {
        match self {
            Vehicle::Car => "car",
            Vehicle::Motorbike => "motorbike",
            Vehicle::Bicycle => "bicycle",
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KmRate {
    pub vehicle: Vehicle,
    pub per_km: Cents,
    pub from: String,
    pub to: String,
    #[serde(default = "default_true")]
    pub verified: bool,
    #[serde(default)]
    pub source: Option<SourceRef>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    Integer,
    Amount,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(rename = "type")]
    pub kind: QuestionType,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<Ask>,
}

/// How the year check asks about an item or question.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ask {
    /// Lower comes first; equal orders keep the file's order.
    pub order: u32,
    pub question: String,
    #[serde(default)]
    pub hint: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Documents,
    Trips,
    Amount,
    ReportedByOthers,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Evidence {
    #[default]
    Required,
    Optional,
    None,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Depreciation {
    pub asset_class: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub key: String,
    pub kz: String,
    pub name: String,
    pub section: String,
    pub kind: ItemKind,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub evidence: Option<Evidence>,
    #[serde(default)]
    pub private_share: Option<u32>,
    #[serde(default)]
    pub depreciation: Option<Depreciation>,
    #[serde(default)]
    pub vehicle: Option<Vehicle>,
    #[serde(default)]
    pub estimate: Vec<EstimateRule>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default = "default_true")]
    pub verified: bool,
    #[serde(default)]
    pub source: Option<SourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<Ask>,
}

impl Item {
    /// What the entry needs before it counts; information-only items never need a PDF.
    pub fn evidence(&self) -> Evidence {
        match self.kind {
            ItemKind::ReportedByOthers => Evidence::None,
            _ => self.evidence.unwrap_or_default(),
        }
    }
}

/// What the tax office applies after the value is copied; it only adds a note (ADR 0040).
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum EstimateRule {
    #[serde(rename_all = "camelCase")]
    Cap {
        amount: Cents,
        #[serde(default)]
        carry_over: bool,
    },
    Requires {
        question: String,
        min: f64,
    },
    ReduceBy {
        question: String,
    },
}

fn default_true() -> bool {
    true
}

impl Ruleset {
    /// Rates and items marked `verified: false`; Settings shows the count until they are checked.
    pub fn unverified_count(&self) -> usize {
        let rates = self.rates.kilometergeld.iter().filter(|r| !r.verified);
        let items = self.items.iter().filter(|i| !i.verified);
        rates.count() + items.count()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_support::rules::{minimal_rules, set};

    fn cents(value: serde_json::Value) -> Result<Cents, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn item_with(kind: &str, evidence: Option<&str>) -> Item {
        let mut item = json!({
            "key": "books", "kz": "720", "name": "Books", "section": "main", "kind": kind
        });
        if let Some(evidence) = evidence {
            item["evidence"] = json!(evidence);
        }
        serde_json::from_value(item).unwrap()
    }

    #[test]
    fn amounts_read_as_whole_cents() {
        for (input, expected) in [
            (json!(1000.00), 100_000),
            (json!(1000), 100_000),
            (json!(0), 0),
            (json!(0.07), 7),
            (json!(19.99), 1999),
            (json!(0.5), 50),
            (json!(132.0), 13_200),
        ] {
            assert_eq!(cents(input.clone()).unwrap(), Cents(expected), "{input}");
        }
    }

    #[test]
    fn negative_amounts_and_fractions_of_a_cent_are_refused() {
        for input in [
            json!(-0.01),
            json!(-5),
            json!(1.005),
            json!(0.001),
            json!("12"),
        ] {
            assert!(cents(input.clone()).is_err(), "{input}");
        }
    }

    #[test]
    fn amounts_write_back_as_euros() {
        assert_eq!(serde_json::to_value(Cents(1999)).unwrap(), json!(19.99));
        assert_eq!(serde_json::to_value(Cents(50)).unwrap(), json!(0.5));
    }

    #[test]
    fn evidence_defaults_to_required() {
        assert_eq!(item_with("documents", None).evidence(), Evidence::Required);
        assert_eq!(item_with("amount", None).evidence(), Evidence::Required);
    }

    #[test]
    fn evidence_follows_the_file_for_ordinary_items() {
        for (text, expected) in [
            ("required", Evidence::Required),
            ("optional", Evidence::Optional),
            ("none", Evidence::None),
        ] {
            assert_eq!(
                item_with("trips", Some(text)).evidence(),
                expected,
                "{text}"
            );
        }
    }

    #[test]
    fn an_item_reported_by_others_never_needs_evidence() {
        assert_eq!(
            item_with("reportedByOthers", None).evidence(),
            Evidence::None
        );
        assert_eq!(
            item_with("reportedByOthers", Some("required")).evidence(),
            Evidence::None
        );
    }

    #[test]
    fn items_are_visible_and_verified_unless_the_file_says_otherwise() {
        let item = item_with("documents", None);
        assert!(item.visible);
        assert!(item.verified);
    }

    #[test]
    fn unverified_items_and_rates_are_counted_together() {
        let mut rules = minimal_rules(2026);
        set(&mut rules, "/items/0/verified", json!(false));
        set(
            &mut rules,
            "/rates",
            json!({ "kilometergeld": [
                { "vehicle": "car", "perKm": 0.5, "from": "2026-01-01", "to": "2026-06-30", "verified": false },
                { "vehicle": "car", "perKm": 0.5, "from": "2026-07-01", "to": "2026-12-31" }
            ]}),
        );
        let ruleset: Ruleset = serde_json::from_value(rules).unwrap();
        assert_eq!(ruleset.unverified_count(), 2);

        let verified: Ruleset = serde_json::from_value(minimal_rules(2026)).unwrap();
        assert_eq!(verified.unverified_count(), 0);
    }
}
