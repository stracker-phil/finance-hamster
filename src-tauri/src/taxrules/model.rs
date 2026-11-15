//! A rules file as typed data (`rules.schema.json`). Unknown fields and rule types fail to parse, so a typo
//! never changes a result silently.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::i18n::lang;
use crate::settings::Language;

/// The German text when the app is German and the file has one, else the English text (ADR 0050).
fn shown<'a>(en: &'a str, de: Option<&'a String>) -> &'a str {
    match (lang(), de) {
        (Language::De, Some(de)) => de,
        _ => en,
    }
}

fn shown_opt<'a>(en: Option<&'a String>, de: Option<&'a String>) -> Option<&'a str> {
    match (lang(), de) {
        (Language::De, Some(de)) => Some(de),
        _ => en.map(String::as_str),
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<QuestionDe>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuestionDe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl Question {
    pub fn display_label(&self) -> &str {
        shown(&self.label, self.de.as_ref().and_then(|d| d.label.as_ref()))
    }

    pub fn display_help(&self) -> Option<&str> {
        shown_opt(
            self.help.as_ref(),
            self.de.as_ref().and_then(|d| d.help.as_ref()),
        )
    }

    pub fn display_unit(&self) -> Option<&str> {
        shown_opt(
            self.unit.as_ref(),
            self.de.as_ref().and_then(|d| d.unit.as_ref()),
        )
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<AskDe>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AskDe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Ask {
    pub fn display_question(&self) -> &str {
        shown(
            &self.question,
            self.de.as_ref().and_then(|d| d.question.as_ref()),
        )
    }

    pub fn display_hint(&self) -> Option<&str> {
        shown_opt(
            self.hint.as_ref(),
            self.de.as_ref().and_then(|d| d.hint.as_ref()),
        )
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<SectionDe>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SectionDe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Section {
    pub fn display_name(&self) -> &str {
        shown(&self.name, self.de.as_ref().and_then(|d| d.name.as_ref()))
    }
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
    /// The legal name, as FinanzOnline shows it.
    pub name: String,
    /// The plain name shown while recording; `name` when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// What belongs here, as examples ("Laptop, monitor, …").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<ItemDe>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemDe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Item {
    pub fn display_name(&self) -> &str {
        shown(&self.name, self.de.as_ref().and_then(|d| d.name.as_ref()))
    }

    pub fn display_label(&self) -> &str {
        let de = self.de.as_ref();
        match self.label.as_ref() {
            Some(label) => shown(label, de.and_then(|d| d.label.as_ref())),
            None => self.display_name(),
        }
    }

    /// The examples; older files that predate them fall back to their year check hint.
    pub fn display_examples(&self) -> Option<&str> {
        let de = self.de.as_ref();
        shown_opt(self.examples.as_ref(), de.and_then(|d| d.examples.as_ref()))
            .or_else(|| self.ask.as_ref().and_then(Ask::display_hint))
    }

    pub fn display_note(&self) -> Option<&str> {
        shown_opt(
            self.note.as_ref(),
            self.de.as_ref().and_then(|d| d.note.as_ref()),
        )
    }

    /// What the entry needs before it counts; information-only items never need a PDF.
    pub fn evidence(&self) -> Evidence {
        match self.kind {
            ItemKind::ReportedByOthers => Evidence::None,
            _ => self.evidence.unwrap_or_default(),
        }
    }
}

/// What the tax office applies after the value is copied; it only adds a note in the FinanzOnline view.
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
    /// `perUnit` for each unit the integer question counts, at most `maxUnits` of them.
    #[serde(rename_all = "camelCase")]
    ReduceByPerUnit {
        question: String,
        per_unit: Cents,
        max_units: u32,
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

    fn german_ruleset() -> Ruleset {
        let mut rules = minimal_rules(2026);
        set(&mut rules, "/sections/0/de", json!({ "name": "Hauptteil" }));
        set(
            &mut rules,
            "/questions",
            json!([
                {
                    "id": "days", "label": "Days", "help": "Days at home", "type": "integer",
                    "unit": "days",
                    "ask": {
                        "order": 1, "question": "How many days?", "hint": "See the calendar",
                        "de": { "question": "Wie viele Tage?", "hint": "Siehe Kalender" }
                    },
                    "de": { "label": "Tage", "help": "Tage zu Hause", "unit": "Tage" }
                },
                { "id": "plain", "label": "Plain", "help": "Only English", "type": "amount", "unit": "EUR" }
            ]),
        );
        set(
            &mut rules,
            "/items/0/de",
            json!({ "name": "Bücher", "note": "Fachbücher" }),
        );
        serde_json::from_value(rules).unwrap()
    }

    #[test]
    fn display_texts_are_german_when_the_language_is_german_and_the_file_has_them() {
        let rules = german_ruleset();
        let (section, item) = (&rules.sections[0], &rules.items[0]);
        let (days, ask) = (
            &rules.questions[0],
            rules.questions[0].ask.as_ref().unwrap(),
        );

        crate::i18n::with_lang(Language::De, || {
            assert_eq!(section.display_name(), "Hauptteil");
            assert_eq!(item.display_name(), "Bücher");
            assert_eq!(days.display_label(), "Tage");
            assert_eq!(days.display_help(), Some("Tage zu Hause"));
            assert_eq!(days.display_unit(), Some("Tage"));
            assert_eq!(ask.display_question(), "Wie viele Tage?");
            assert_eq!(ask.display_hint(), Some("Siehe Kalender"));
        });
    }

    #[test]
    fn display_texts_fall_back_to_english_without_a_german_text() {
        let rules = german_ruleset();
        let plain = &rules.questions[1];

        crate::i18n::with_lang(Language::De, || {
            assert_eq!(plain.display_label(), "Plain");
            assert_eq!(plain.display_help(), Some("Only English"));
            assert_eq!(plain.display_unit(), Some("EUR"));
        });

        let bare: Ruleset = serde_json::from_value(minimal_rules(2026)).unwrap();
        crate::i18n::with_lang(Language::De, || {
            assert_eq!(bare.sections[0].display_name(), "Main");
            assert_eq!(bare.items[0].display_name(), "Books");
        });
    }

    #[test]
    fn a_partial_german_object_shows_german_only_where_it_has_a_text() {
        let mut rules = minimal_rules(2026);
        set(
            &mut rules,
            "/questions",
            json!([{
                "id": "days", "label": "Days", "help": "Days at home", "type": "integer",
                "de": { "label": "Tage" }
            }]),
        );
        let ruleset: Ruleset = serde_json::from_value(rules).unwrap();
        let question = &ruleset.questions[0];

        crate::i18n::with_lang(Language::De, || {
            assert_eq!(question.display_label(), "Tage");
            assert_eq!(question.display_help(), Some("Days at home"));
            assert_eq!(question.display_unit(), None);
        });
    }

    #[test]
    fn display_texts_are_english_when_the_language_is_english() {
        let rules = german_ruleset();
        let (section, item) = (&rules.sections[0], &rules.items[0]);
        let (days, ask) = (
            &rules.questions[0],
            rules.questions[0].ask.as_ref().unwrap(),
        );

        crate::i18n::with_lang(Language::En, || {
            assert_eq!(section.display_name(), "Main");
            assert_eq!(item.display_name(), "Books");
            assert_eq!(days.display_label(), "Days");
            assert_eq!(days.display_help(), Some("Days at home"));
            assert_eq!(days.display_unit(), Some("days"));
            assert_eq!(ask.display_question(), "How many days?");
            assert_eq!(ask.display_hint(), Some("See the calendar"));
        });
    }

    #[test]
    fn german_texts_round_trip_and_are_left_out_when_absent() {
        let rules = german_ruleset();
        let value = serde_json::to_value(&rules).unwrap();
        assert_eq!(
            value["items"][0]["de"],
            json!({ "name": "Bücher", "note": "Fachbücher" })
        );
        assert_eq!(value["questions"][0]["de"]["label"], json!("Tage"));
        assert!(value["questions"][1].get("de").is_none());

        let copy: Ruleset = serde_json::from_value(value).unwrap();
        assert_eq!(copy, rules);
    }

    fn item_json(extra: serde_json::Value) -> Item {
        let mut item = json!({
            "key": "books", "kz": "720", "name": "Fachliteratur", "section": "main", "kind": "documents"
        });
        item.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(item).unwrap()
    }

    #[test]
    fn an_item_label_falls_back_to_its_name() {
        let item = item_json(json!({ "de": { "name": "Fachbücher" } }));

        for (language, expected) in [
            (Language::En, "Fachliteratur"),
            (Language::De, "Fachbücher"),
        ] {
            crate::i18n::with_lang(language, || {
                assert_eq!(item.display_label(), expected, "{language:?}");
            });
        }
    }

    #[test]
    fn an_item_label_is_german_only_when_the_file_has_a_german_label() {
        let with_german = item_json(json!({
            "label": "Books", "de": { "name": "Fachbücher", "label": "Bücher" }
        }));
        let english_only = item_json(json!({ "label": "Books", "de": { "name": "Fachbücher" } }));

        crate::i18n::with_lang(Language::En, || {
            assert_eq!(with_german.display_label(), "Books");
        });
        crate::i18n::with_lang(Language::De, || {
            assert_eq!(with_german.display_label(), "Bücher");
            assert_eq!(english_only.display_label(), "Books");
        });
    }

    #[test]
    fn item_examples_are_german_when_the_language_is_german_and_the_file_has_them() {
        let item = item_json(json!({
            "examples": "Laptop, monitor", "de": { "examples": "Laptop, Bildschirm" }
        }));

        crate::i18n::with_lang(Language::En, || {
            assert_eq!(item.display_examples(), Some("Laptop, monitor"));
        });
        crate::i18n::with_lang(Language::De, || {
            assert_eq!(item.display_examples(), Some("Laptop, Bildschirm"));
        });
    }

    #[test]
    fn item_examples_fall_back_to_the_ask_hint_and_then_to_nothing() {
        let hinted = item_json(json!({
            "ask": {
                "order": 1, "question": "Books?", "hint": "Contract or device",
                "de": { "hint": "Vertrag oder Gerät" }
            }
        }));
        let both = item_json(json!({
            "examples": "Laptop", "ask": { "order": 1, "question": "Books?", "hint": "Contract" }
        }));
        let bare = item_json(json!({}));

        crate::i18n::with_lang(Language::En, || {
            assert_eq!(hinted.display_examples(), Some("Contract or device"));
            assert_eq!(both.display_examples(), Some("Laptop"));
            assert_eq!(bare.display_examples(), None);
        });
        crate::i18n::with_lang(Language::De, || {
            assert_eq!(hinted.display_examples(), Some("Vertrag oder Gerät"));
        });
    }

    #[test]
    fn an_item_note_is_german_when_available_and_english_otherwise() {
        let with_german =
            item_json(json!({ "note": "Keep receipts", "de": { "note": "Belege aufheben" } }));
        let english_only = item_json(json!({ "note": "Keep receipts" }));
        let bare = item_json(json!({}));

        crate::i18n::with_lang(Language::En, || {
            assert_eq!(with_german.display_note(), Some("Keep receipts"));
        });
        crate::i18n::with_lang(Language::De, || {
            assert_eq!(with_german.display_note(), Some("Belege aufheben"));
            assert_eq!(english_only.display_note(), Some("Keep receipts"));
            assert_eq!(bare.display_note(), None);
        });
    }

    #[test]
    fn item_label_and_examples_round_trip_and_are_left_out_when_absent() {
        let item = item_json(json!({
            "label": "Books", "examples": "Novels",
            "de": { "label": "Bücher", "examples": "Romane" }
        }));
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(value["label"], json!("Books"));
        assert_eq!(value["examples"], json!("Novels"));
        assert_eq!(
            value["de"],
            json!({ "label": "Bücher", "examples": "Romane" })
        );
        assert_eq!(serde_json::from_value::<Item>(value).unwrap(), item);

        let bare = serde_json::to_value(item_json(json!({}))).unwrap();
        assert!(bare.get("label").is_none());
        assert!(bare.get("examples").is_none());
    }

    #[test]
    fn a_per_unit_reduction_reads_from_json() {
        let rule: EstimateRule = serde_json::from_value(json!({
            "type": "reduceByPerUnit", "question": "days", "perUnit": 3.00, "maxUnits": 100
        }))
        .unwrap();

        assert_eq!(
            rule,
            EstimateRule::ReduceByPerUnit {
                question: "days".into(),
                per_unit: Cents(300),
                max_units: 100,
            }
        );
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
