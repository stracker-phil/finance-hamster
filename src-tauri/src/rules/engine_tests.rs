use super::builtin_rules;
use super::engine::{Line, decide, line_text, matches};
use super::store::{Action, DecisionRow, RuleRow};
use super::tests::{HOUSEHOLD_IBAN, NOW, PRIVATE_IBAN, line, own_accounts, rule_row};
use crate::statements::store::TransactionRow;

fn rule(tokens: &[&str]) -> RuleRow {
    rule_row("rule-1", 0, tokens, Action::Ignore)
}

fn holds(rule: &RuleRow, t: &TransactionRow) -> bool {
    matches(rule, &Line::of(t, &own_accounts()))
}

#[test]
fn the_searched_text_joins_booking_text_extra_lines_merchant_and_counterparty_in_upper_case() {
    let t = TransactionRow {
        text: "Lastschrift".into(),
        extra: r#"["ref: abc"]"#.into(),
        merchant: Some("Zebra Shop".into()),
        counterparty: Some("Zebra GmbH".into()),
        ..line()
    };

    assert_eq!(
        line_text(&t),
        "LASTSCHRIFT\nREF: ABC\nZEBRA SHOP\nZEBRA GMBH"
    );
}

#[test]
fn any_one_token_is_enough() {
    for (tokens, expected) in [
        (vec!["lumen", "streamflix"], true),
        (vec!["streamflix"], true),
        (vec!["lumen", "cafe"], false),
    ] {
        assert_eq!(holds(&rule(&tokens), &line()), expected, "{tokens:?}");
    }
}

#[test]
fn tokens_ignore_case_and_surrounding_spaces() {
    for (token, expected) in [
        ("streamflix", true),
        ("STREAMFLIX", true),
        ("StreamFlix", true),
        (" streamflix ", true),
        ("netflix", false),
    ] {
        assert_eq!(holds(&rule(&[token]), &line()), expected, "{token:?}");
    }
}

#[test]
fn a_token_matches_the_booking_text_the_extra_lines_the_merchant_and_the_counterparty() {
    let neutral = || TransactionRow {
        text: "Lastschrift".into(),
        ..line()
    };
    let cases = [
        (
            "booking text",
            TransactionRow {
                text: "Zebra purchase".into(),
                ..neutral()
            },
            true,
        ),
        (
            "extra lines",
            TransactionRow {
                extra: r#"["REF: ZEBRA 1"]"#.into(),
                ..neutral()
            },
            true,
        ),
        (
            "merchant",
            TransactionRow {
                merchant: Some("ZEBRA SHOP".into()),
                ..neutral()
            },
            true,
        ),
        (
            "counterparty",
            TransactionRow {
                counterparty: Some("Zebra GmbH".into()),
                ..neutral()
            },
            true,
        ),
        (
            "counterparty iban",
            TransactionRow {
                counterparty_iban: Some("ZEBRA".into()),
                ..neutral()
            },
            false,
        ),
        ("nowhere", neutral(), false),
    ];

    for (place, t, expected) in cases {
        assert_eq!(holds(&rule(&["zebra"]), &t), expected, "{place}");
    }
}

#[test]
fn unreadable_extra_lines_are_skipped_while_the_booking_text_still_matches() {
    let t = TransactionRow {
        extra: "not json".into(),
        ..line()
    };

    assert!(holds(&rule(&["streamflix"]), &t));
    assert!(!holds(&rule(&["json"]), &t));
}

#[test]
fn the_amount_is_compared_without_its_sign_and_within_the_tolerance_inclusive() {
    for (cents, amount, tolerance, expected) in [
        (-1_299, 1_299, 0, true),
        (1_299, 1_299, 0, true),
        (-1_300, 1_299, 0, false),
        (-1_299, -1_299, 0, true),
        (-2_595, 2_595, 100, true),
        (-2_495, 2_595, 100, true),
        (-2_695, 2_595, 100, true),
        (-2_494, 2_595, 100, false),
        (-2_696, 2_595, 100, false),
    ] {
        let rule = RuleRow {
            amount_cents: Some(amount),
            tolerance_cents: tolerance,
            ..rule(&[])
        };
        let t = TransactionRow { cents, ..line() };
        assert_eq!(
            holds(&rule, &t),
            expected,
            "{cents} against {amount} ± {tolerance}"
        );
    }
}

#[test]
fn the_account_condition_needs_the_lines_own_account() {
    let rule = RuleRow {
        account_id: Some("account-1".into()),
        ..rule(&[])
    };

    for (account, expected) in [("account-1", true), ("account-2", false)] {
        let t = TransactionRow {
            account_id: account.into(),
            ..line()
        };
        assert_eq!(holds(&rule, &t), expected, "{account}");
    }
}

#[test]
fn the_own_counterparty_condition_is_true_only_for_another_own_accounts_iban() {
    let rule = RuleRow {
        own_counterparty: true,
        ..rule(&[])
    };

    for (iban, account, expected) in [
        (Some(HOUSEHOLD_IBAN), "account-1", true),
        (Some(PRIVATE_IBAN), "account-2", true),
        (Some(PRIVATE_IBAN), "account-1", false),
        (Some("AT091999900000000077"), "account-1", false),
        (None, "account-1", false),
    ] {
        let t = TransactionRow {
            account_id: account.into(),
            counterparty_iban: iban.map(Into::into),
            ..line()
        };
        assert_eq!(holds(&rule, &t), expected, "{iban:?} on {account}");
    }
}

#[test]
fn every_condition_of_a_rule_must_hold() {
    let rule = RuleRow {
        amount_cents: Some(1_299),
        account_id: Some("account-1".into()),
        own_counterparty: true,
        ..self::rule(&["streamflix"])
    };
    let all = TransactionRow {
        counterparty_iban: Some(HOUSEHOLD_IBAN.into()),
        ..line()
    };
    assert!(holds(&rule, &all));

    for (broken, t) in [
        (
            "text",
            TransactionRow {
                text: "CAFE LINDE".into(),
                ..all.clone()
            },
        ),
        (
            "amount",
            TransactionRow {
                cents: -1_300,
                ..all.clone()
            },
        ),
        (
            "account",
            TransactionRow {
                account_id: "account-3".into(),
                ..all.clone()
            },
        ),
        (
            "counterparty",
            TransactionRow {
                counterparty_iban: None,
                ..all.clone()
            },
        ),
    ] {
        assert!(!holds(&rule, &t), "{broken}");
    }
}

#[test]
fn a_rule_without_conditions_matches_nothing() {
    let unreadable = RuleRow {
        contains: "not json".into(),
        ..rule(&[])
    };
    let lines = [
        line(),
        TransactionRow {
            counterparty_iban: Some(HOUSEHOLD_IBAN.into()),
            ..line()
        },
    ];

    for t in &lines {
        assert!(!holds(&rule(&[]), t));
        assert!(!holds(&unreadable, t));
    }
}

#[test]
fn blank_tokens_are_no_condition_and_do_not_hide_the_real_ones() {
    for tokens in [vec![" "], vec![""], vec![" ", ""]] {
        assert!(!holds(&rule(&tokens), &line()), "{tokens:?}");
    }

    let with_blank = rule(&[" ", "STREAMFLIX"]);
    assert!(holds(&with_blank, &line()));
    assert!(!holds(
        &with_blank,
        &TransactionRow {
            text: "CAFE LINDE".into(),
            ..line()
        }
    ));
}

#[test]
fn the_first_matching_rule_by_order_decides_and_the_decision_copies_the_rule() {
    let cafe = rule_row("rule-cafe", 0, &["cafe"], Action::Ignore);
    let item = RuleRow {
        subscription_id: Some("sub-1".into()),
        document_id: Some("doc-1".into()),
        ..rule_row("rule-item", 1, &["streamflix"], Action::Anv)
    };
    let ignore = rule_row("rule-ignore", 2, &["streamflix"], Action::Ignore);
    let rules = vec![cafe, item, ignore];

    let decision = decide(&rules, &line(), &own_accounts(), NOW);

    assert_eq!(
        decision,
        Some(DecisionRow {
            id: "line-1".into(),
            kind: Action::Anv,
            item_key: Some("books".into()),
            subscription_id: Some("sub-1".into()),
            document_id: Some("doc-1".into()),
            rule_id: Some("rule-item".into()),
            decided_at: NOW,
        })
    );

    let reversed: Vec<RuleRow> = rules.into_iter().rev().collect();
    let decision = decide(&reversed, &line(), &own_accounts(), NOW).unwrap();
    assert_eq!(decision.rule_id.as_deref(), Some("rule-ignore"));
    assert_eq!(decision.kind, Action::Ignore);
}

#[test]
fn no_decision_comes_without_a_matching_rule() {
    let rules = vec![rule_row("rule-cafe", 0, &["cafe"], Action::Ignore)];

    assert_eq!(decide(&rules, &line(), &own_accounts(), NOW), None);
    assert_eq!(decide(&[], &line(), &own_accounts(), NOW), None);
}

#[test]
fn the_built_in_bank_fee_rule_ignores_fee_lines_in_any_case() {
    for text in ["Kontoführung 10/2026", "Entgelt Debitkarte", "KONTOFÜHRUNG"] {
        let t = TransactionRow {
            text: text.into(),
            ..line()
        };
        let decision = decide(&builtin_rules(NOW), &t, &own_accounts(), NOW).unwrap();
        assert_eq!(
            (decision.kind, decision.rule_id.as_deref()),
            (Action::Ignore, Some("builtin-bank-fees")),
            "{text}"
        );
    }
    assert_eq!(
        decide(&builtin_rules(NOW), &line(), &own_accounts(), NOW),
        None
    );
}

#[test]
fn the_built_in_transfer_rule_ignores_lines_paying_another_own_account() {
    for (iban, expected) in [
        (Some(HOUSEHOLD_IBAN), Some("builtin-transfers")),
        (Some(PRIVATE_IBAN), None),
        (Some("AT091999900000000077"), None),
        (None, None),
    ] {
        let t = TransactionRow {
            counterparty_iban: iban.map(Into::into),
            ..line()
        };
        let decision = decide(&builtin_rules(NOW), &t, &own_accounts(), NOW);
        assert_eq!(
            decision.as_ref().and_then(|d| d.rule_id.as_deref()),
            expected,
            "{iban:?}"
        );
    }
}
