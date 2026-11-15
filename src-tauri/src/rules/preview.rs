//! What a rule in the sheet would match, before it is saved: the count and a few of the newest lines.

use serde::Serialize;
use ts_rs::TS;

use super::engine::{Line, matches};
use super::{RuleForm, apply, blank, encode_tokens, store};
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::money::{Money, format_signed, parse_cents};
use crate::review::title_of;
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::{tr, tr_plural};

const EXAMPLES: usize = 3;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct RulePreview {
    pub count: u32,
    /// `Matches 12 lines; 2 decided by hand keep your choice.`
    pub text: String,
    /// `Tue, 10 Nov · BILLA · −€12.30`, newest first.
    pub examples: Vec<String>,
}

fn cents(text: &Option<String>) -> Option<i64> {
    text.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .and_then(parse_cents)
        .map(i64::abs)
}

pub fn preview(
    db: &Db,
    form: &RuleForm,
    number: NumberFormat,
    order: DateOrder,
) -> AppResult<RulePreview> {
    let tokens: Vec<&str> = form
        .contains
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();
    let amount_cents = cents(&form.amount);
    if tokens.is_empty()
        && amount_cents.is_none()
        && form.account_id.is_none()
        && !form.own_counterparty
    {
        return Ok(RulePreview {
            count: 0,
            text: tr!(
                "Add a condition to see which lines the rule matches.",
                "Füge eine Bedingung hinzu, um zu sehen, zu welchen Buchungen die Regel passt."
            ),
            examples: Vec::new(),
        });
    }
    let rule = store::RuleRow {
        contains: encode_tokens(&tokens),
        amount_cents,
        tolerance_cents: amount_cents.and(cents(&form.tolerance)).unwrap_or(0),
        account_id: form.account_id.clone(),
        own_counterparty: form.own_counterparty,
        ..blank("preview".into(), 0, 0)
    };
    let own = db.read(apply::own_accounts)?;
    let decided = db.read(store::decisions)?;
    let mut lines: Vec<_> = db
        .read(statements::all_transactions)?
        .into_iter()
        .filter(|t| matches(&rule, &Line::of(t, &own)))
        .collect();
    lines.sort_by(|a, b| b.value_date.cmp(&a.value_date));
    let by_hand = lines
        .iter()
        .filter(|t| decided.get(&t.id).is_some_and(store::DecisionRow::by_hand))
        .count();
    let count = lines.len();
    let matching = tr_plural!(
        count,
        "Matches {count} line",
        "Matches {count} lines",
        "Passt zu {count} Buchung",
        "Passt zu {count} Buchungen"
    );
    let text = match (count, by_hand) {
        (0, _) => tr!("Matches no line yet.", "Passt noch zu keiner Buchung."),
        (_, 0) => format!("{matching}."),
        (_, h) => {
            let kept = tr_plural!(
                h,
                "{h} decided by hand keeps your choice.",
                "{h} decided by hand keep your choice.",
                "{h} von Hand entschiedene behält deine Wahl.",
                "{h} von Hand entschiedene behalten deine Wahl."
            );
            format!("{matching}; {kept}")
        }
    };
    let examples = lines
        .iter()
        .take(EXAMPLES)
        .map(|t| {
            let day = date::parse(&t.value_date)
                .map(|d| date::weekday_short(d, order))
                .unwrap_or_default();
            format!(
                "{day} · {} · {}",
                title_of(t),
                format_signed(&Money::eur(t.cents), number)
            )
        })
        .collect();
    Ok(RulePreview {
        count: count as u32,
        text,
        examples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::{
        add, decide_by_hand, household_transfer, ignore, import_statement, setup,
    };
    use crate::statements::import_tests::october;
    use tempfile::TempDir;

    const DOT: NumberFormat = NumberFormat::DotDecimal;
    const DAY_FIRST: DateOrder = DateOrder::DayFirst;

    /// The demo October statement on `account-1`, imported.
    fn october_db() -> (TempDir, Db) {
        let (dir, db) = setup();
        import_statement(&db, dir.path(), &october());
        (dir, db)
    }

    fn run(db: &Db, form: &RuleForm) -> RulePreview {
        preview(db, form, DOT, DAY_FIRST).unwrap()
    }

    fn examples(list: &[&str]) -> Vec<String> {
        list.iter().map(|e| (*e).to_string()).collect()
    }

    #[test]
    fn a_rule_without_a_condition_asks_for_one() {
        let (_dir, db) = october_db();
        let waiting = RulePreview {
            count: 0,
            text: tr!(
                "Add a condition to see which lines the rule matches.",
                "Füge eine Bedingung hinzu, um zu sehen, zu welchen Buchungen die Regel passt."
            ),
            examples: Vec::new(),
        };
        let blank_conditions = RuleForm {
            contains: vec!["".into(), "   ".into()],
            amount: Some("not an amount".into()),
            tolerance: Some("5,00".into()),
            ..ignore(&[])
        };
        assert_eq!(run(&db, &ignore(&[])), waiting);
        assert_eq!(run(&db, &blank_conditions), waiting);
    }

    #[test]
    fn a_text_condition_counts_the_matching_line_and_shows_it() {
        let (_dir, db) = october_db();
        assert_eq!(
            run(&db, &ignore(&["streamflix"])),
            RulePreview {
                count: 1,
                text: "Matches 1 line.".into(),
                examples: examples(&["Wed, 7 Oct · STREAMFLIX WIEN 1010 · −€12.99"]),
            }
        );
    }

    #[test]
    fn text_tokens_match_any_of_them_and_examples_come_newest_first() {
        let (_dir, db) = october_db();
        assert_eq!(
            run(&db, &ignore(&["tankstelle", "cafe linde"])),
            RulePreview {
                count: 2,
                text: "Matches 2 lines.".into(),
                examples: examples(&[
                    "Wed, 21 Oct · CAFE LINDE DORNBIRN 6850 · −€6.21",
                    "Mon, 5 Oct · TANKSTELLE RHEIN LUSTENAU 6890 · −€58.30",
                ]),
            }
        );
    }

    #[test]
    fn a_text_nothing_contains_matches_no_line_yet() {
        let (_dir, db) = october_db();
        assert_eq!(
            run(&db, &ignore(&["no such merchant"])),
            RulePreview {
                count: 0,
                text: "Matches no line yet.".into(),
                examples: Vec::new(),
            }
        );
    }

    #[test]
    fn at_most_three_examples_are_shown_while_the_count_covers_every_line() {
        let (_dir, db) = october_db();
        let form = RuleForm {
            account_id: Some("account-1".into()),
            ..ignore(&[])
        };
        assert_eq!(
            run(&db, &form),
            RulePreview {
                count: 7,
                text: "Matches 7 lines.".into(),
                examples: examples(&[
                    "Wed, 21 Oct · CAFE LINDE DORNBIRN 6850 · −€6.21",
                    "Wed, 14 Oct · BUCHHANDLUNG SEITENWEISE · −€46.90",
                    "Sat, 10 Oct · LUMEN AI SAN FRANCISCO · −€22.48",
                ]),
            }
        );
    }

    #[test]
    fn an_account_condition_matches_only_that_accounts_lines() {
        let (_dir, db) = october_db();
        let other = RuleForm {
            account_id: Some("account-2".into()),
            ..ignore(&[])
        };
        assert_eq!(run(&db, &other).count, 0);
    }

    #[test]
    fn an_amount_matches_a_debit_or_credit_of_that_size_without_its_sign() {
        let (_dir, db) = october_db();
        for amount in ["12,99", "-12.99", "€12.99"] {
            let form = RuleForm {
                amount: Some(amount.into()),
                ..ignore(&[])
            };
            assert_eq!(
                run(&db, &form).examples,
                examples(&["Wed, 7 Oct · STREAMFLIX WIEN 1010 · −€12.99"]),
                "{amount}"
            );
        }
    }

    #[test]
    fn the_tolerance_widens_the_amount_condition() {
        let (_dir, db) = october_db();
        let form = RuleForm {
            amount: Some("20,00".into()),
            tolerance: Some("10,00".into()),
            ..ignore(&[])
        };
        assert_eq!(
            run(&db, &form),
            RulePreview {
                count: 2,
                text: "Matches 2 lines.".into(),
                examples: examples(&[
                    "Sat, 10 Oct · LUMEN AI SAN FRANCISCO · −€22.48",
                    "Wed, 7 Oct · STREAMFLIX WIEN 1010 · −€12.99",
                ]),
            }
        );
    }

    #[test]
    fn an_invalid_amount_is_ignored_so_the_other_conditions_decide() {
        let (_dir, db) = october_db();
        let form = RuleForm {
            amount: Some("abc".into()),
            tolerance: Some("1000,00".into()),
            ..ignore(&["cafe"])
        };
        assert_eq!(run(&db, &form).count, 1);
    }

    #[test]
    fn a_tolerance_alone_is_no_condition() {
        let (_dir, db) = october_db();
        let form = RuleForm {
            tolerance: Some("1,00".into()),
            ..ignore(&[])
        };
        assert_eq!(
            run(&db, &form).text,
            "Add a condition to see which lines the rule matches."
        );
    }

    #[test]
    fn an_own_counterparty_condition_matches_transfers_between_own_accounts() {
        let (dir, db) = setup();
        let mut statement = october();
        for day in [2, 9, 20] {
            statement.lines.push(household_transfer(day));
        }
        statement.lines.sort_by_key(|l| l.value_date);
        import_statement(&db, dir.path(), &statement);
        let form = RuleForm {
            own_counterparty: true,
            ..ignore(&[])
        };
        assert_eq!(
            run(&db, &form),
            RulePreview {
                count: 3,
                text: "Matches 3 lines.".into(),
                examples: examples(&[
                    "Tue, 20 Oct · HOUSEHOLD · −€500.00",
                    "Fri, 9 Oct · HOUSEHOLD · −€500.00",
                    "Fri, 2 Oct · HOUSEHOLD · −€500.00",
                ]),
            }
        );
    }

    #[test]
    fn lines_decided_by_hand_are_counted_and_keep_their_choice() {
        let (_dir, db) = october_db();
        decide_by_hand(&db, "STREAMFLIX");
        let one = run(&db, &ignore(&["streamflix", "lumen"]));
        assert_eq!(
            one.text,
            "Matches 2 lines; 1 decided by hand keeps your choice."
        );
        decide_by_hand(&db, "LUMEN");
        let both = run(&db, &ignore(&["streamflix", "lumen"]));
        assert_eq!(
            both.text,
            "Matches 2 lines; 2 decided by hand keep your choice."
        );
        assert_eq!(both.count, 2);
    }

    #[test]
    fn lines_a_rule_decided_are_not_counted_as_decided_by_hand() {
        let (_dir, db) = october_db();
        add(&db, ignore(&["streamflix"]));
        assert_eq!(run(&db, &ignore(&["streamflix"])).text, "Matches 1 line.");
    }

    #[test]
    fn examples_follow_the_number_and_date_format() {
        let (_dir, db) = october_db();
        let shown = preview(
            &db,
            &ignore(&["streamflix"]),
            NumberFormat::CommaDecimal,
            DateOrder::MonthFirst,
        )
        .unwrap();
        assert_eq!(
            shown.examples,
            examples(&["Wed, Oct 7 · STREAMFLIX WIEN 1010 · −€12,99"])
        );
    }

    #[test]
    fn the_preview_serializes_with_camel_case_fields() {
        let preview = RulePreview {
            count: 1,
            text: "Matches 1 line.".into(),
            examples: examples(&["Wed, 7 Oct · STREAMFLIX WIEN 1010 · −€12.99"]),
        };
        assert_eq!(
            serde_json::to_value(&preview).unwrap(),
            serde_json::json!({
                "count": 1,
                "text": "Matches 1 line.",
                "examples": ["Wed, 7 Oct · STREAMFLIX WIEN 1010 · −€12.99"]
            })
        );
    }
}
