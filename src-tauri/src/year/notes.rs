//! The one note line per Kennzahl from the item's estimate rules: they never change the copy value, only say
//! what the tax office will do with it. Applied in order (requires, reduceBy, cap); the first that speaks wins.

use std::collections::HashMap;

use crate::money::{Money, format_money};
use crate::settings::NumberFormat;
use crate::taxrules::model::{EstimateRule, Item, Question, QuestionType};

/// `Telearbeitstage` from `Telearbeitstage (Homeoffice days)`: the label before any explanation.
fn short(label: &str) -> &str {
    label.split(" (").next().unwrap_or(label).trim()
}

fn answer_first(question: &str) -> String {
    crate::tr!(
        "Answer {question} under Questions first.",
        "Beantworte zuerst {question} unter Fragen."
    )
}

fn amount(cents: i64, format: NumberFormat) -> String {
    format_money(&Money::eur(cents), format)
}

/// The note for `item`; None when no rule has anything to say.
pub fn note(
    item: &Item,
    questions: &[Question],
    answers: &HashMap<String, i64>,
    format: NumberFormat,
) -> Option<String> {
    let label = |id: &str| {
        questions
            .iter()
            .find(|q| q.id == id)
            .map_or(id.to_string(), |q| short(q.display_label()).to_string())
    };
    let mut rules: Vec<&EstimateRule> = item.estimate.iter().collect();
    rules.sort_by_key(|r| match r {
        EstimateRule::Requires { .. } => 0,
        EstimateRule::ReduceBy { .. } => 1,
        EstimateRule::Cap { .. } => 2,
    });
    for rule in rules {
        match rule {
            EstimateRule::Requires { question, min } => match answers.get(question) {
                None => return Some(answer_first(&label(question))),
                Some(value) if (*value as f64) < *min => {
                    let label = label(question);
                    return Some(crate::tr!(
                        "Fewer than {min} {label}: the tax office won't accept this.",
                        "Weniger als {min} {label}: Das Finanzamt erkennt das nicht an."
                    ));
                }
                Some(_) => {}
            },
            EstimateRule::ReduceBy { question } => {
                let is_amount = questions
                    .iter()
                    .any(|q| &q.id == question && q.kind == QuestionType::Amount);
                return Some(match answers.get(question) {
                    None => answer_first(&label(question)),
                    Some(cents) if is_amount => {
                        let (amount, label) = (amount(*cents, format), label(question));
                        crate::tr!(
                            "The tax office subtracts the {amount} {label} you received.",
                            "Das Finanzamt zieht {amount} ({label}) ab."
                        )
                    }
                    Some(_) => {
                        let label = label(question);
                        crate::tr!(
                            "The tax office subtracts the {label} you received.",
                            "Das Finanzamt zieht {label} ab."
                        )
                    }
                });
            }
            EstimateRule::Cap { amount: cap, .. } => {
                let cap = amount(cap.0, format);
                return Some(crate::tr!(
                    "Enter the full amount. The tax office counts up to {cap} per year.",
                    "Trag den vollen Betrag ein. Das Finanzamt zählt bis zu {cap} pro Jahr."
                ));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn questions() -> Vec<Question> {
        serde_json::from_value(json!([
            { "id": "telearbeitstage", "label": "Telearbeitstage (Homeoffice days)", "type": "integer" },
            { "id": "pauschale", "label": "Telearbeitspauschale (from the employer)", "type": "amount" }
        ]))
        .unwrap()
    }

    fn item(estimate: Value) -> Item {
        serde_json::from_value(json!({
            "key": "home", "kz": "723", "name": "Home office", "section": "main",
            "kind": "amount", "estimate": estimate
        }))
        .unwrap()
    }

    fn answered(pairs: &[(&str, i64)]) -> HashMap<String, i64> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    fn note_of(estimate: Value, answers: &[(&str, i64)]) -> Option<String> {
        note(
            &item(estimate),
            &questions(),
            &answered(answers),
            NumberFormat::DotDecimal,
        )
    }

    fn requires() -> Value {
        json!({ "type": "requires", "question": "telearbeitstage", "min": 26 })
    }

    fn reduce_by() -> Value {
        json!({ "type": "reduceBy", "question": "pauschale" })
    }

    fn cap() -> Value {
        json!({ "type": "cap", "amount": 300.0 })
    }

    const CAP_NOTE: &str = "Enter the full amount. The tax office counts up to €300.00 per year.";

    #[test]
    fn an_item_without_estimate_rules_has_no_note() {
        assert_eq!(note_of(json!([]), &[("telearbeitstage", 5)]), None);
    }

    #[test]
    fn an_unanswered_requirement_asks_for_the_answer() {
        assert_eq!(
            note_of(json!([requires()]), &[]),
            Some("Answer Telearbeitstage under Questions first.".into())
        );
    }

    #[test]
    fn an_answer_below_the_minimum_names_the_question_without_its_explanation() {
        assert_eq!(
            note_of(json!([requires()]), &[("telearbeitstage", 25)]),
            Some("Fewer than 26 Telearbeitstage: the tax office won't accept this.".into())
        );
    }

    #[test]
    fn a_met_requirement_alone_says_nothing_and_lets_later_rules_speak() {
        assert_eq!(
            note_of(json!([requires()]), &[("telearbeitstage", 26)]),
            None
        );
        assert_eq!(
            note_of(json!([requires(), cap()]), &[("telearbeitstage", 26)]),
            Some(CAP_NOTE.into())
        );
    }

    #[test]
    fn a_reduction_asks_for_its_answer_or_names_the_amount_received() {
        assert_eq!(
            note_of(json!([reduce_by()]), &[]),
            Some("Answer Telearbeitspauschale under Questions first.".into())
        );
        assert_eq!(
            note_of(json!([reduce_by()]), &[("pauschale", 12_000)]),
            Some("The tax office subtracts the €120.00 Telearbeitspauschale you received.".into())
        );
    }

    #[test]
    fn a_reduction_by_a_count_names_no_amount() {
        let estimate = json!([{ "type": "reduceBy", "question": "telearbeitstage" }]);

        assert_eq!(
            note_of(estimate, &[("telearbeitstage", 30)]),
            Some("The tax office subtracts the Telearbeitstage you received.".into())
        );
    }

    #[test]
    fn notes_read_in_german_when_the_language_is_german() {
        use crate::i18n::with_lang;
        use crate::settings::Language;
        let german = |estimate: Value, answers: &[(&str, i64)]| {
            with_lang(Language::De, || note_of(estimate, answers))
        };

        assert_eq!(
            german(json!([requires()]), &[]),
            Some("Beantworte zuerst Telearbeitstage unter Fragen.".into())
        );
        assert_eq!(
            german(json!([requires()]), &[("telearbeitstage", 25)]),
            Some("Weniger als 26 Telearbeitstage: Das Finanzamt erkennt das nicht an.".into())
        );
        assert_eq!(
            german(json!([reduce_by()]), &[("pauschale", 12_000)]),
            Some("Das Finanzamt zieht €120.00 (Telearbeitspauschale) ab.".into())
        );
        assert_eq!(
            german(json!([cap()]), &[]),
            Some("Trag den vollen Betrag ein. Das Finanzamt zählt bis zu €300.00 pro Jahr.".into())
        );
    }

    #[test]
    fn a_cap_names_the_limit_per_year() {
        assert_eq!(note_of(json!([cap()]), &[]), Some(CAP_NOTE.into()));
    }

    #[test]
    fn amounts_in_notes_follow_the_number_format() {
        let note = note(
            &item(json!([reduce_by()])),
            &questions(),
            &answered(&[("pauschale", 123_456)]),
            NumberFormat::CommaDecimal,
        );

        assert_eq!(
            note.as_deref(),
            Some("The tax office subtracts the €1.234,56 Telearbeitspauschale you received.")
        );
    }

    #[test]
    fn rules_apply_as_requires_then_reduce_by_then_cap_whatever_the_file_order() {
        let orders = [
            json!([cap(), reduce_by(), requires()]),
            json!([reduce_by(), cap(), requires()]),
            json!([requires(), cap(), reduce_by()]),
            json!([cap(), requires(), reduce_by()]),
        ];
        for estimate in orders {
            assert_eq!(
                note_of(estimate.clone(), &[]),
                Some("Answer Telearbeitstage under Questions first.".into()),
                "{estimate}"
            );
            assert_eq!(
                note_of(
                    estimate.clone(),
                    &[("telearbeitstage", 10), ("pauschale", 12_000)]
                ),
                Some("Fewer than 26 Telearbeitstage: the tax office won't accept this.".into()),
                "{estimate}"
            );
            assert_eq!(
                note_of(
                    estimate.clone(),
                    &[("telearbeitstage", 30), ("pauschale", 12_000)]
                ),
                Some(
                    "The tax office subtracts the €120.00 Telearbeitspauschale you received."
                        .into()
                ),
                "{estimate}"
            );
        }
    }

    #[test]
    fn a_reduction_speaks_before_a_cap() {
        assert_eq!(
            note_of(json!([cap(), reduce_by()]), &[("pauschale", 5_000)]),
            Some("The tax office subtracts the €50.00 Telearbeitspauschale you received.".into())
        );
    }
}
