use chrono::NaiveDate;

use super::model::{Card, CardKind, Fx, ParsedLine, ParsedStatement};
use super::{ParseError, amount, parse, sparkasse};
use crate::platform::pdf::Pdf;
use crate::statements::check::check;
use crate::test_support::statements::{Statement, StatementLine, demo_month};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn demo() -> Statement {
    demo_month(10, 2026, 10, 141_230)
}

fn long_statement() -> Statement {
    let mut statement = demo();
    for i in 0..30 {
        statement.lines.push(StatementLine {
            text: format!("Rate {i}"),
            extra: vec!["A".into(), "B".into()],
            value_date: date(2026, 10, 25),
            cents: -1_000,
        });
    }
    statement
}

fn booking(text: &str, extra: &[&str], value_date: NaiveDate, cents: i64) -> StatementLine {
    StatementLine {
        text: text.into(),
        extra: extra.iter().map(|e| (*e).to_string()).collect(),
        value_date,
        cents,
    }
}

fn with_lines(lines: Vec<StatementLine>) -> Statement {
    Statement {
        lines,
        old_balance: 0,
        ..demo()
    }
}

fn read(statement: &Statement) -> ParsedStatement {
    parse(&statement.text_pages()).unwrap()
}

fn read_through_pdfkit(statement: &Statement) -> Vec<Vec<String>> {
    Pdf::from_bytes(&statement.pdf())
        .unwrap()
        .text_pages()
        .unwrap()
}

fn line<'a>(parsed: &'a ParsedStatement, text_prefix: &str) -> &'a ParsedLine {
    parsed
        .lines
        .iter()
        .find(|l| l.text.starts_with(text_prefix))
        .unwrap_or_else(|| panic!("no line starting with {text_prefix}"))
}

fn edit_line(pages: &mut [Vec<String>], prefix: &str, edit: impl Fn(&str) -> String) {
    let line = pages
        .iter_mut()
        .flatten()
        .find(|l| l.trim_start().starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting with {prefix}"));
    *line = edit(line);
}

fn without(pages: &mut [Vec<String>], prefix: &str) {
    for page in pages {
        page.retain(|l| !l.trim_start().starts_with(prefix));
    }
}

#[test]
fn printed_amounts_read_as_signed_cents() {
    for (text, expected) in [
        ("1.234,56", Some(123_456)),
        ("+2.850,00", Some(285_000)),
        ("-980,00", Some(-98_000)),
        ("0,05", Some(5)),
        ("1.000.000,00", Some(100_000_000)),
        ("12,5", None),
        ("12.50", None),
        ("", None),
        ("abc", None),
        ("1,234", None),
        (",50", None),
        ("-", None),
        ("12,5x", None),
        ("99999999999999999999,00", None),
    ] {
        assert_eq!(amount(text), expected, "{text:?}");
    }
}

#[test]
fn text_in_an_unknown_layout_is_rejected() {
    for pages in [
        Vec::new(),
        vec![Vec::new()],
        vec![vec![
            "Some other bank".to_string(),
            "IBAN: nope".to_string(),
        ]],
    ] {
        assert_eq!(parse(&pages), Err(ParseError::UnknownLayout), "{pages:?}");
    }
}

#[test]
fn a_sparkasse_statement_is_recognized_and_read_by_its_parser() {
    let pages = demo().text_pages();
    assert!(sparkasse::recognizes(&pages));
    let parsed = parse(&pages).unwrap();
    assert_eq!(parsed.layout, sparkasse::LAYOUT);
    assert_eq!(parsed.layout, "sparkasse-erste-1");
    assert_eq!(Ok(parsed), sparkasse::parse(&pages));
}

#[test]
fn a_statement_without_its_header_lines_is_not_recognized() {
    let mut no_number = demo().text_pages();
    edit_line(&mut no_number, "Example", |_| "Example Sparkasse".into());
    let mut no_iban = demo().text_pages();
    edit_line(&mut no_iban, "IBAN:", |_| "Konto".into());
    for pages in [no_number, no_iban, Vec::new()] {
        assert!(!sparkasse::recognizes(&pages));
        assert_eq!(parse(&pages), Err(ParseError::UnknownLayout));
    }
}

#[test]
fn the_header_is_read_with_a_normalized_iban_number_year_and_period() {
    let parsed = read(&demo());
    assert_eq!(parsed.iban, "AT241999900000000001");
    assert_eq!(parsed.number, 10);
    assert_eq!(parsed.year, 2026);
    assert_eq!(parsed.from, date(2026, 10, 1));
    assert_eq!(parsed.to, date(2026, 10, 28));
}

#[test]
fn the_balances_and_printed_sums_are_read_with_debits_negative() {
    let parsed = read(&demo());
    assert_eq!(parsed.old_balance, 141_230);
    assert_eq!(parsed.credits, 285_000);
    assert_eq!(parsed.debits, -112_688);
    assert_eq!(parsed.new_balance, 313_542);
    assert_eq!(parsed.sums_page, 1);
}

#[test]
fn every_booking_line_keeps_its_page_row_text_value_date_and_signed_cents() {
    let parsed = read(&demo());
    let first_row = parsed.lines[0].row;
    let lines: Vec<_> = parsed
        .lines
        .iter()
        .map(|l| {
            (
                l.page,
                l.row - first_row,
                l.text.as_str(),
                l.value_date,
                l.cents,
            )
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            (1, 0, "Gehalt", date(2026, 10, 1), 285_000),
            (1, 4, "Dauerauftrag", date(2026, 10, 3), -98_000),
            (
                1,
                8,
                "POS 58,30 AT K1 04.10. 17:40 TANKSTELLE RHEIN LUSTENAU 040",
                date(2026, 10, 5),
                -5_830
            ),
            (
                1,
                9,
                "E-COMM 12,99 AT K1 06.10. STREAMFLIX WIEN 040",
                date(2026, 10, 7),
                -1_299
            ),
            (
                1,
                10,
                "E-COMM 24,00 USD K1 08.10. LUMEN AI SAN FRANCISCO 840",
                date(2026, 10, 10),
                -2_248
            ),
            (1, 12, "Lastschrift", date(2026, 10, 14), -4_690),
            (
                1,
                16,
                "POS 6,21 AT K1 20.10. 09:12 CAFE LINDE DORNBIRN 040",
                date(2026, 10, 21),
                -621
            ),
        ]
    );
}

#[test]
fn the_indented_lines_below_a_booking_are_kept_trimmed_as_its_extra_lines() {
    let parsed = read(&demo());
    assert_eq!(
        line(&parsed, "Gehalt").extra,
        vec![
            "NORDWIND SOFTWARE GMBH",
            "AT94 1999 9000 0000 0099",
            "REF: LOHN"
        ]
    );
    assert_eq!(
        line(&parsed, "E-COMM 24,00").extra,
        vec!["SPESEN: 0,49 KURS: 1,0912 VOM: 06.10.2026"]
    );
    assert!(line(&parsed, "POS 58,30").extra.is_empty());
}

#[test]
fn the_counterparty_is_the_first_extra_line_of_a_transfer() {
    let parsed = read(&demo());
    for (text, expected, iban) in [
        ("Gehalt", "NORDWIND SOFTWARE GMBH", "AT941999900000000099"),
        (
            "Dauerauftrag",
            "HAUSVERWALTUNG MUSTER",
            "AT811999900000000042",
        ),
        (
            "Lastschrift",
            "BUCHHANDLUNG SEITENWEISE",
            "AT091999900000000077",
        ),
    ] {
        let l = line(&parsed, text);
        assert_eq!(l.counterparty.as_deref(), Some(expected), "{text}");
        assert_eq!(l.counterparty_iban.as_deref(), Some(iban), "{text}");
    }
}

#[test]
fn a_valid_iban_among_the_extra_lines_is_the_counterparty_iban() {
    let statement = with_lines(vec![
        booking(
            "Überweisung",
            &["  MAX MUSTER  ", "DE89 3704 0044 0532 0130 00", "MIETE"],
            date(2026, 10, 2),
            -50_000,
        ),
        booking(
            "Gutschrift",
            &["DE89 3704 0044 0532 0130 00", "RUECKZAHLUNG"],
            date(2026, 10, 3),
            10_000,
        ),
        booking("Entgelt", &["KONTOFUEHRUNG"], date(2026, 10, 4), -500),
        booking(
            "Entgelt",
            &["AT25 1999 9000 0000 0001"],
            date(2026, 10, 5),
            -100,
        ),
    ]);
    let parsed = read(&statement);
    let found: Vec<_> = parsed
        .lines
        .iter()
        .map(|l| {
            (
                l.counterparty.as_deref(),
                l.counterparty_iban.as_deref(),
                l.extra.first().map(String::as_str),
            )
        })
        .collect();
    assert_eq!(
        found,
        vec![
            (
                Some("MAX MUSTER"),
                Some("DE89370400440532013000"),
                Some("MAX MUSTER")
            ),
            (
                None,
                Some("DE89370400440532013000"),
                Some("DE89 3704 0044 0532 0130 00")
            ),
            (Some("KONTOFUEHRUNG"), None, Some("KONTOFUEHRUNG")),
            (
                Some("AT25 1999 9000 0000 0001"),
                None,
                Some("AT25 1999 9000 0000 0001")
            ),
        ]
    );
}

#[test]
fn an_online_card_payment_in_a_foreign_currency_has_its_original_amount_and_fx_line() {
    let parsed = read(&demo());
    let lumen = line(&parsed, "E-COMM 24,00");
    assert_eq!(
        lumen.card,
        Some(Card {
            kind: CardKind::Online,
            date: date(2026, 10, 8),
            time: None,
            merchant: "LUMEN AI SAN FRANCISCO".into(),
            original: Some((2_400, "USD".into())),
        })
    );
    assert_eq!(
        lumen.fx,
        Some(Fx {
            fee: 49,
            rate: "1,0912".into(),
            rate_date: date(2026, 10, 6),
        })
    );
}

#[test]
fn an_online_card_payment_in_the_home_country_has_no_original_amount_or_fx() {
    let parsed = read(&demo());
    let streamflix = line(&parsed, "E-COMM 12,99");
    assert_eq!(
        streamflix.card,
        Some(Card {
            kind: CardKind::Online,
            date: date(2026, 10, 6),
            time: None,
            merchant: "STREAMFLIX WIEN".into(),
            original: None,
        })
    );
    assert_eq!(streamflix.fx, None);
}

#[test]
fn a_terminal_payment_has_a_time_and_a_merchant_without_the_country_code() {
    let parsed = read(&demo());
    assert_eq!(
        line(&parsed, "POS 58,30").card,
        Some(Card {
            kind: CardKind::Terminal,
            date: date(2026, 10, 4),
            time: Some("17:40".into()),
            merchant: "TANKSTELLE RHEIN LUSTENAU".into(),
            original: None,
        })
    );
    assert_eq!(
        line(&parsed, "POS 6,21").card,
        Some(Card {
            kind: CardKind::Terminal,
            date: date(2026, 10, 20),
            time: Some("09:12".into()),
            merchant: "CAFE LINDE DORNBIRN".into(),
            original: None,
        })
    );
}

#[test]
fn card_lines_have_no_counterparty_and_transfers_have_no_card() {
    let parsed = read(&demo());
    for l in &parsed.lines {
        let is_card = l.text.starts_with("POS ") || l.text.starts_with("E-COMM ");
        assert_eq!(l.card.is_some(), is_card, "{}", l.text);
        if is_card {
            assert_eq!((&l.counterparty, &l.counterparty_iban), (&None, &None));
        } else {
            assert_eq!(l.fx, None, "{}", l.text);
        }
    }
}

#[test]
fn a_statement_across_new_year_puts_december_lines_in_the_year_before() {
    let statement = Statement {
        number: 1,
        from: date(2026, 12, 28),
        to: date(2027, 1, 27),
        lines: vec![
            booking("Dauerauftrag", &["ABO"], date(2026, 12, 30), -1_000),
            booking("Gehalt", &[], date(2027, 1, 5), 200_000),
            booking(
                "E-COMM 10,00 AT K1 30.12. SHOP WIEN 040",
                &[],
                date(2026, 12, 31),
                -1_000,
            ),
        ],
        old_balance: 0,
        ..demo()
    };
    let parsed = read(&statement);
    assert_eq!((parsed.number, parsed.year), (1, 2027));
    assert_eq!(
        parsed
            .lines
            .iter()
            .map(|l| l.value_date)
            .collect::<Vec<_>>(),
        vec![date(2026, 12, 30), date(2027, 1, 5), date(2026, 12, 31)]
    );
    assert_eq!(
        parsed.lines[2].card.as_ref().map(|c| c.date),
        Some(date(2026, 12, 30))
    );
}

#[test]
fn a_statement_without_lines_reads_with_no_booking() {
    let parsed = read(&with_lines(Vec::new()));
    assert!(parsed.lines.is_empty());
    assert_eq!((parsed.credits, parsed.debits), (0, 0));
}

#[test]
fn a_multi_page_statement_reads_the_lines_of_every_page_in_order() {
    let statement = long_statement();
    let pages = statement.text_pages();
    let parsed = parse(&pages).unwrap();
    assert!(pages.len() >= 2);
    assert_eq!(parsed.lines.len(), statement.lines.len());
    assert_eq!(
        parsed.lines.iter().map(|l| l.cents).collect::<Vec<_>>(),
        statement.lines.iter().map(|l| l.cents).collect::<Vec<_>>()
    );
    assert_eq!(parsed.lines.last().map(|l| l.page), Some(pages.len()));
    assert_eq!(parsed.sums_page, pages.len());
    for l in &parsed.lines {
        let printed = pages[l.page - 1]
            .iter()
            .filter(|p| !p.trim().is_empty())
            .nth(l.row - 1);
        assert!(
            printed.is_some_and(|p| p.starts_with(&l.text)),
            "page {} row {}",
            l.page,
            l.row
        );
    }
}

#[test]
fn rows_count_from_the_header_on_each_page_and_continuation_headers_are_skipped() {
    let pages = long_statement().text_pages();
    let parsed = parse(&pages).unwrap();
    let first_on = |page: usize| parsed.lines.iter().find(|l| l.page == page).unwrap().row;
    for page in 2..=pages.len() {
        assert_eq!(first_on(page) + 1, first_on(1), "page {page}");
    }
    assert!(
        parsed
            .lines
            .iter()
            .all(|l| !l.text.contains("Kontoauszug") && !l.text.contains("Seite"))
    );
    assert_eq!(
        parsed.lines.iter().filter(|l| l.extra.len() == 2).count(),
        30
    );
}

#[test]
fn a_missing_header_part_or_sum_is_reported_by_name() {
    for (prefix, replacement, missing) in [
        ("Example", Some("Example Sparkasse"), "Kontoauszug Nr."),
        ("IBAN:", Some("Konto"), "IBAN"),
        ("Zeitraum:", Some(""), "Zeitraum"),
        ("Alter Saldo", None, "Alter Saldo"),
        ("Summe Gutschriften", None, "Summe Gutschriften"),
        ("Summe Lastschriften", None, "Summe Lastschriften"),
        ("Neuer Saldo", None, "Neuer Saldo"),
    ] {
        let mut pages = demo().text_pages();
        match replacement {
            Some(text) => edit_line(&mut pages, prefix, |_| text.to_string()),
            None => without(&mut pages, prefix),
        }
        assert_eq!(
            sparkasse::parse(&pages),
            Err(ParseError::Missing(missing.into())),
            "{prefix}"
        );
    }
}

#[test]
fn an_iban_with_a_wrong_checksum_is_unreadable() {
    let mut pages = demo().text_pages();
    edit_line(&mut pages, "IBAN:", |l| l.replace("AT24", "AT25"));
    assert_eq!(
        parse(&pages),
        Err(ParseError::Unreadable(
            "IBAN AT25 1999 9000 0000 0001".into()
        ))
    );
}

#[test]
fn a_booking_line_without_a_signed_amount_is_unreadable_and_names_its_page() {
    let mut pages = demo().text_pages();
    edit_line(&mut pages, "Gehalt", |l| {
        l.replace("+2.850,00", " 2.850,00")
    });
    let edited = pages[0].iter().find(|l| l.starts_with("Gehalt")).unwrap();
    assert_eq!(
        parse(&pages),
        Err(ParseError::Unreadable(format!("page 1: {}", edited.trim())))
    );
}

#[test]
fn an_unreadable_booking_on_a_later_page_names_that_page() {
    let mut pages = long_statement().text_pages();
    let last = pages.len();
    assert!(last >= 2);
    edit_line(&mut pages, "Rate 29", |l| l.replace("-10,00", " 10,00"));
    let edited = pages[last - 1]
        .iter()
        .find(|l| l.starts_with("Rate 29"))
        .unwrap();
    assert_eq!(
        parse(&pages),
        Err(ParseError::Unreadable(format!(
            "page {last}: {}",
            edited.trim()
        )))
    );
}

#[test]
fn text_read_through_pdfkit_parses_like_the_generated_text_and_passes_the_checks() {
    for statement in [demo(), long_statement()] {
        let from_pdf = parse(&read_through_pdfkit(&statement)).unwrap();
        assert_eq!(from_pdf, read(&statement));
        assert_eq!(check(&from_pdf), Ok(()));
    }
}
