//! The one note line per Kennzahl in the FinanzOnline view, from the item's estimate rules. The copy value stays the
//! full amount, because the tax office applies these rules itself; the note works out what that leaves. Applied
//! in order (requires, reductions, cap); the first that speaks wins.

use std::collections::HashMap;

use crate::money::{Money, format_money};
use crate::settings::NumberFormat;
use crate::taxrules::model::{EstimateRule, Item, Question};

/// `Telearbeitstage` from `Telearbeitstage (Homeoffice days)`: the label before any explanation.
fn short(label: &str) -> &str {
    label.split(" (").next().unwrap_or(label).trim()
}

fn answer_first(question: &str) -> String {
    crate::tr!(
        "Enter your {question} first.",
        "Trag zuerst deine {question} ein."
    )
}

fn amount(cents: i64, format: NumberFormat) -> String {
    format_money(&Money::eur(cents), format)
}

fn what_counts(copy: i64, subtracted: i64, why: &str, format: NumberFormat) -> String {
    let (counts, subtracted) = (
        amount((copy - subtracted).max(0), format),
        amount(subtracted, format),
    );
    crate::tr!(
        "About {counts} counts: the tax office subtracts {subtracted} ({why}).",
        "Etwa {counts} zählen: Das Finanzamt zieht {subtracted} ab ({why})."
    )
}

/// The note for `item` whose copy value is `copy` cents; None when no rule has anything to say.
pub fn note(
    item: &Item,
    questions: &[Question],
    answers: &HashMap<String, i64>,
    copy: i64,
    format: NumberFormat,
) -> Option<String> {
    let label = |id: &str| {
        questions
            .iter()
            .find(|q| q.id == id)
            .map_or(id.to_string(), |q| short(q.display_label()).to_string())
    };
    let unit = |id: &str| {
        questions
            .iter()
            .find(|q| q.id == id)
            .and_then(Question::display_unit)
            .map_or_else(|| label(id), str::to_string)
    };
    let mut rules: Vec<&EstimateRule> = item.estimate.iter().collect();
    rules.sort_by_key(|r| match r {
        EstimateRule::Requires { .. } => 0,
        EstimateRule::ReduceBy { .. } | EstimateRule::ReduceByPerUnit { .. } => 1,
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
                return Some(match answers.get(question) {
                    None => answer_first(&label(question)),
                    Some(cents) => what_counts(copy, *cents, &label(question), format),
                });
            }
            EstimateRule::ReduceByPerUnit {
                question,
                per_unit,
                max_units,
            } => match answers.get(question) {
                None => return Some(answer_first(&label(question))),
                Some(units) if *units > 0 => {
                    let counted = (*units).min(i64::from(*max_units));
                    let per = amount(per_unit.0, format);
                    let why = format!("{counted} {} × {per}", unit(question));
                    return Some(what_counts(copy, counted * per_unit.0, &why, format));
                }
                Some(_) => {}
            },
            EstimateRule::Cap {
                amount: cap,
                carry_over,
            } => {
                if copy <= cap.0 {
                    return None;
                }
                let (rest, cap) = (amount(copy - cap.0, format), amount(cap.0, format));
                return Some(if *carry_over {
                    crate::tr!(
                        "Counts {cap} this year; the other {rest} count in later years.",
                        "Zählt dieses Jahr {cap}; die übrigen {rest} zählen in späteren Jahren."
                    )
                } else {
                    crate::tr!(
                        "Counts at most {cap}; the other {rest} don't count.",
                        "Zählt höchstens {cap}; die übrigen {rest} zählen nicht."
                    )
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::i18n::with_lang;
    use crate::settings::Language;

    fn questions() -> Vec<Question> {
        serde_json::from_value(json!([
            {
                "id": "telearbeitstage", "label": "Telearbeitstage (Homeoffice days)", "type": "integer",
                "unit": "days",
                "de": { "label": "Homeoffice-Tage (laut Lohnzettel)", "unit": "Tage" }
            },
            {
                "id": "pauschale", "label": "Telearbeitspauschale (from the employer)", "type": "amount"
            },
            { "id": "trips", "label": "Trips (this year)", "type": "integer" }
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

    fn note_of(estimate: Value, answers: &[(&str, i64)], copy: i64) -> Option<String> {
        note(
            &item(estimate),
            &questions(),
            &answered(answers),
            copy,
            NumberFormat::DotDecimal,
        )
    }

    fn requires() -> Value {
        json!({ "type": "requires", "question": "telearbeitstage", "min": 26 })
    }

    fn reduce_by() -> Value {
        json!({ "type": "reduceBy", "question": "pauschale" })
    }

    fn per_day() -> Value {
        json!({ "type": "reduceByPerUnit", "question": "telearbeitstage", "perUnit": 3.0, "maxUnits": 100 })
    }

    fn cap(carry_over: bool) -> Value {
        json!({ "type": "cap", "amount": 300.0, "carryOver": carry_over })
    }

    #[test]
    fn an_item_without_estimate_rules_has_no_note() {
        assert_eq!(note_of(json!([]), &[("telearbeitstage", 5)], 50_000), None);
    }

    #[test]
    fn an_unanswered_requirement_asks_to_enter_the_answer() {
        assert_eq!(
            note_of(json!([requires()]), &[], 50_000),
            Some("Enter your Telearbeitstage first.".into())
        );
    }

    #[test]
    fn an_answer_below_the_minimum_names_the_question_without_its_explanation() {
        assert_eq!(
            note_of(json!([requires()]), &[("telearbeitstage", 25)], 50_000),
            Some("Fewer than 26 Telearbeitstage: the tax office won't accept this.".into())
        );
    }

    #[test]
    fn a_met_requirement_alone_says_nothing_and_lets_later_rules_speak() {
        assert_eq!(
            note_of(json!([requires()]), &[("telearbeitstage", 26)], 50_000),
            None
        );
        assert_eq!(
            note_of(
                json!([requires(), cap(false)]),
                &[("telearbeitstage", 26)],
                50_000
            ),
            Some("Counts at most €300.00; the other €200.00 don't count.".into())
        );
    }

    #[test]
    fn an_unanswered_amount_reduction_asks_to_enter_the_answer() {
        assert_eq!(
            note_of(json!([reduce_by()]), &[], 50_000),
            Some("Enter your Telearbeitspauschale first.".into())
        );
    }

    #[test]
    fn an_amount_reduction_subtracts_the_answer_from_the_copy_value() {
        assert_eq!(
            note_of(json!([reduce_by()]), &[("pauschale", 12_000)], 50_000),
            Some(
                "About €380.00 counts: the tax office subtracts €120.00 (Telearbeitspauschale)."
                    .into()
            )
        );
    }

    #[test]
    fn a_reduction_larger_than_the_copy_value_leaves_nothing_but_never_less() {
        assert_eq!(
            note_of(json!([reduce_by()]), &[("pauschale", 12_000)], 5_000),
            Some(
                "About €0.00 counts: the tax office subtracts €120.00 (Telearbeitspauschale)."
                    .into()
            )
        );
    }

    #[test]
    fn a_per_unit_reduction_asks_for_its_answer_first() {
        assert_eq!(
            note_of(json!([per_day()]), &[], 50_000),
            Some("Enter your Telearbeitstage first.".into())
        );
    }

    #[test]
    fn a_per_unit_reduction_subtracts_the_units_times_the_rate_up_to_the_maximum() {
        for (days, counted, counts, subtracted) in [
            (26, 26, "€422.00", "€78.00"),
            (100, 100, "€200.00", "€300.00"),
            (159, 100, "€200.00", "€300.00"),
        ] {
            assert_eq!(
                note_of(json!([per_day()]), &[("telearbeitstage", days)], 50_000),
                Some(format!(
                    "About {counts} counts: the tax office subtracts {subtracted} ({counted} days × €3.00)."
                )),
                "{days} days"
            );
        }
    }

    #[test]
    fn a_per_unit_reduction_larger_than_the_copy_value_floors_at_zero() {
        assert_eq!(
            note_of(json!([per_day()]), &[("telearbeitstage", 159)], 10_000),
            Some("About €0.00 counts: the tax office subtracts €300.00 (100 days × €3.00).".into())
        );
    }

    #[test]
    fn a_per_unit_reduction_names_a_question_without_unit_by_its_short_label() {
        let estimate = json!([
            { "type": "reduceByPerUnit", "question": "trips", "perUnit": 3.0, "maxUnits": 100 }
        ]);

        assert_eq!(
            note_of(estimate, &[("trips", 10)], 50_000),
            Some(
                "About €470.00 counts: the tax office subtracts €30.00 (10 Trips × €3.00).".into()
            )
        );
    }

    #[test]
    fn a_per_unit_reduction_answered_with_zero_says_nothing_and_lets_later_rules_speak() {
        assert_eq!(
            note_of(json!([per_day()]), &[("telearbeitstage", 0)], 50_000),
            None
        );
        assert_eq!(
            note_of(
                json!([per_day(), cap(true)]),
                &[("telearbeitstage", 0)],
                50_000
            ),
            Some("Counts €300.00 this year; the other €200.00 count in later years.".into())
        );
    }

    #[test]
    fn a_cap_says_nothing_while_the_copy_value_stays_within_it() {
        for copy in [0, 29_999, 30_000] {
            assert_eq!(note_of(json!([cap(false)]), &[], copy), None, "{copy}");
            assert_eq!(note_of(json!([cap(true)]), &[], copy), None, "{copy}");
        }
    }

    #[test]
    fn a_cap_names_what_counts_and_what_the_rest_does() {
        assert_eq!(
            note_of(json!([cap(true)]), &[], 50_000),
            Some("Counts €300.00 this year; the other €200.00 count in later years.".into())
        );
        assert_eq!(
            note_of(json!([cap(false)]), &[], 50_000),
            Some("Counts at most €300.00; the other €200.00 don't count.".into())
        );
    }

    #[test]
    fn amounts_in_notes_follow_the_number_format() {
        let note = note(
            &item(json!([reduce_by()])),
            &questions(),
            &answered(&[("pauschale", 123_456)]),
            200_000,
            NumberFormat::CommaDecimal,
        );

        assert_eq!(
            note.as_deref(),
            Some(
                "About €765,44 counts: the tax office subtracts €1.234,56 (Telearbeitspauschale)."
            )
        );
    }

    #[test]
    fn rules_apply_as_requires_then_reductions_then_cap_whatever_the_file_order() {
        let rules = [requires(), reduce_by(), per_day(), cap(false)];
        let orders: [[usize; 4]; 4] = [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1]];
        for order in orders {
            let estimate = Value::Array(order.iter().map(|i| rules[*i].clone()).collect());
            assert_eq!(
                note_of(estimate.clone(), &[], 50_000),
                Some("Enter your Telearbeitstage first.".into()),
                "{estimate}"
            );
            assert_eq!(
                note_of(
                    estimate.clone(),
                    &[("telearbeitstage", 10), ("pauschale", 12_000)],
                    50_000
                ),
                Some("Fewer than 26 Telearbeitstage: the tax office won't accept this.".into()),
                "{estimate}"
            );
        }
    }

    #[test]
    fn a_reduction_speaks_before_a_cap() {
        assert_eq!(
            note_of(
                json!([cap(false), reduce_by()]),
                &[("pauschale", 5_000)],
                50_000
            ),
            Some(
                "About €450.00 counts: the tax office subtracts €50.00 (Telearbeitspauschale)."
                    .into()
            )
        );
        assert_eq!(
            note_of(
                json!([cap(false), per_day()]),
                &[("telearbeitstage", 26)],
                50_000
            ),
            Some("About €422.00 counts: the tax office subtracts €78.00 (26 days × €3.00).".into())
        );
    }

    #[test]
    fn notes_read_in_german_when_the_language_is_german() {
        let german = |estimate: Value, answers: &[(&str, i64)]| {
            with_lang(Language::De, || note_of(estimate, answers, 50_000))
        };

        assert_eq!(
            german(json!([requires()]), &[]),
            Some("Trag zuerst deine Homeoffice-Tage ein.".into())
        );
        assert_eq!(
            german(json!([requires()]), &[("telearbeitstage", 25)]),
            Some("Weniger als 26 Homeoffice-Tage: Das Finanzamt erkennt das nicht an.".into())
        );
        assert_eq!(
            german(json!([reduce_by()]), &[("pauschale", 12_000)]),
            Some(
                "Etwa €380.00 zählen: Das Finanzamt zieht €120.00 ab (Telearbeitspauschale)."
                    .into()
            )
        );
        assert_eq!(
            german(json!([per_day()]), &[("telearbeitstage", 159)]),
            Some("Etwa €200.00 zählen: Das Finanzamt zieht €300.00 ab (100 Tage × €3.00).".into())
        );
        assert_eq!(
            german(json!([cap(true)]), &[]),
            Some(
                "Zählt dieses Jahr €300.00; die übrigen €200.00 zählen in späteren Jahren.".into()
            )
        );
        assert_eq!(
            german(json!([cap(false)]), &[]),
            Some("Zählt höchstens €300.00; die übrigen €200.00 zählen nicht.".into())
        );
    }
}
