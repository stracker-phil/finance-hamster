use chrono::NaiveDate;

use super::model::{Card, CardKind, Fx, ParsedLine, ParsedStatement};
use super::{ParseError, amount, george, parse, sparkasse};
use crate::statements::check::check;
use crate::test_support::statements::demo_month;

const IBAN_LINE: &str = "                                              IBAN:  AT241999900000000001";
const NUMBER_LINE: &str = "                                              Kontoauszug              Nr. 004/2026";
const COLUMNS_LINE: &str =
    " Buchungstext/Booking Text                        Valuta/Value   Beträge/Amounts in EUR";
const FOOTER: &str = "AT241999900000000001 12.11.202621:00        0                 004             1";
const FOOTER_TITLES: &str =
    "IBAN                Datum/DateUhrzeit/Time Belege(*)/VouchersAuszug/StatementSeite/Page";

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn page(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|l| (*l).to_string()).collect()
}

fn head() -> Vec<&'static str> {
    vec![
        "                                                 Example Sparkasse",
        NUMBER_LINE,
        "                                              Account Statement",
        IBAN_LINE,
        "                                              BIC:   EXAMPLEXXX",
        "   ",
        "   Erika Muster",
        "                                              Alter Kontostand              1.000,00",
        "                                              Old Balance",
        "                                              Gutschriften                  2.000,00",
        "                                              Credits",
        "                                              Belastungen                     562,93-",
        "                                              Debits",
        "                                              Neuer Kontostand              2.437,07",
        "                                              New Balance",
    ]
}

/// A single page with the usual header and the given lines below the column titles.
fn one_page(bookings: &[&str]) -> Vec<Vec<String>> {
    let mut lines = head();
    lines.push(COLUMNS_LINE);
    lines.extend(bookings.iter().copied());
    lines.push(FOOTER);
    lines.push(FOOTER_TITLES);
    vec![page(&lines)]
}

fn statement_pages() -> Vec<Vec<String>> {
    let mut first = head();
    first.extend([
        COLUMNS_LINE,
        " Gehalt 9/2026                                    27.09.2026                2.000,00",
        "  Example GmbH",
        "  GEHALT 9/2026",
        " E-COMM        7,10 AT  K1   28.09. 00:00         29.09.2026                    7,10-",
        "  DEMO   IMBISS  DORNBIRN 6850 040",
        "",
        " POS          20,00 AT  K1   28.09. 19:42         29.09.2026                   20,00-",
        "  BUCHLADEN MUSTER DORNBIRN 6850 040",
        " Dauerauftrag                                     28.09.2026                  400,00-",
        FOOTER,
        FOOTER_TITLES,
    ]);
    let second = [
        "                                                 Example Sparkasse",
        NUMBER_LINE,
        "                                              Account Statement",
        COLUMNS_LINE,
        "  Hausverwaltung Muster",
        "  MIETE 10/2026",
        " BAECKEREI    2373  K1   30.09. 10:54             30.09.2026                   80,00-",
        " E-COMM       24,00 USD K1   29.09. LUMEN AI SAN FRANCISCO 840   02.10.2026       22,48-",
        "  SPESEN: 0,49 KURS: 1,0912 VOM: 29.09.2026",
        " *** Abschlussbuchung per 30.09.2026 **** 30.09.2026",
        "  Reklamationen bitte binnen 2 Monaten",
        " Kontoführung                                     30.09.2026                   33,35-",
        " Neuer Kontostand/New Balance                                             2.437,07",
        " Aktuelle Informationen:",
        "  Kontostand per 30.09.2026       2.437,07",
        "  Kontostand per 31.10.2026       2.437,07",
    ];
    vec![page(&first), page(&second)]
}

fn read(pages: &[Vec<String>]) -> ParsedStatement {
    george::parse(pages).unwrap()
}

fn line<'a>(parsed: &'a ParsedStatement, text_prefix: &str) -> &'a ParsedLine {
    parsed
        .lines
        .iter()
        .find(|l| l.text.starts_with(text_prefix))
        .unwrap_or_else(|| panic!("no line starting with {text_prefix}"))
}

fn replace_first_page_line(pages: &mut [Vec<String>], prefix: &str, with: &str) {
    let line = pages[0]
        .iter_mut()
        .find(|l| l.trim_start().starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting with {prefix}"));
    *line = with.to_string();
}

fn without_first_page_line(pages: &mut [Vec<String>], prefix: &str) {
    pages[0].retain(|l| !l.trim_start().starts_with(prefix));
}

#[test]
fn a_multi_page_statement_reads_its_header_sums_and_every_booking_and_passes_the_check() {
    let parsed = read(&statement_pages());
    assert_eq!(parsed.layout, "sparkasse-george-1");
    assert_eq!(parsed.iban, "AT241999900000000001");
    assert_eq!((parsed.number, parsed.year), (4, 2026));
    assert_eq!((parsed.from, parsed.to), (date(2026, 9, 27), date(2026, 10, 2)));
    assert_eq!(parsed.old_balance, 100_000);
    assert_eq!(parsed.credits, 200_000);
    assert_eq!(parsed.debits, -56_293);
    assert_eq!(parsed.new_balance, 243_707);
    assert_eq!(parsed.sums_page, 1);
    assert_eq!(
        parsed.lines.iter().map(|l| l.cents).collect::<Vec<_>>(),
        vec![200_000, -710, -2_000, -40_000, -8_000, -2_248, -3_335]
    );
    assert_eq!(check(&parsed), Ok(()));
}

#[test]
fn every_booking_keeps_its_page_row_text_value_date_and_signed_cents() {
    let parsed = read(&statement_pages());
    let found: Vec<_> = parsed
        .lines
        .iter()
        .map(|l| (l.page, l.row, l.text.as_str(), l.value_date, l.cents))
        .collect();
    assert_eq!(
        found,
        vec![
            (1, 16, "Gehalt 9/2026", date(2026, 9, 27), 200_000),
            (
                1,
                19,
                "E-COMM 7,10 AT K1 28.09. 00:00",
                date(2026, 9, 29),
                -710
            ),
            (
                1,
                21,
                "POS 20,00 AT K1 28.09. 19:42",
                date(2026, 9, 29),
                -2_000
            ),
            (1, 23, "Dauerauftrag", date(2026, 9, 28), -40_000),
            (
                2,
                7,
                "BAECKEREI 2373 K1 30.09. 10:54",
                date(2026, 9, 30),
                -8_000
            ),
            (
                2,
                8,
                "E-COMM 24,00 USD K1 29.09. LUMEN AI SAN FRANCISCO 840",
                date(2026, 10, 2),
                -2_248
            ),
            (2, 12, "Kontoführung", date(2026, 9, 30), -3_335),
        ]
    );
}

#[test]
fn detail_lines_are_the_extra_lines_of_their_booking_with_spaces_collapsed() {
    let parsed = read(&statement_pages());
    assert_eq!(
        line(&parsed, "Gehalt").extra,
        vec!["Example GmbH", "GEHALT 9/2026"]
    );
    assert_eq!(
        line(&parsed, "E-COMM 7,10").extra,
        vec!["DEMO IMBISS DORNBIRN 6850 040"]
    );
}

#[test]
fn a_page_may_start_with_the_detail_lines_of_the_previous_pages_last_booking() {
    let parsed = read(&statement_pages());
    assert_eq!(
        line(&parsed, "Dauerauftrag").extra,
        vec!["Hausverwaltung Muster", "MIETE 10/2026"]
    );
    assert!(line(&parsed, "BAECKEREI").extra.is_empty());
}

#[test]
fn the_closing_heading_its_note_and_the_trailing_information_belong_to_no_booking() {
    let parsed = read(&statement_pages());
    assert_eq!(parsed.lines.len(), 7);
    assert_eq!(
        line(&parsed, "E-COMM 24,00").extra,
        vec!["SPESEN: 0,49 KURS: 1,0912 VOM: 29.09.2026"]
    );
    let fee = line(&parsed, "Kontoführung");
    assert!(fee.extra.is_empty());
    assert_eq!(fee.counterparty, None);
    assert_eq!(fee.counterparty_iban, None);
    for l in &parsed.lines {
        for text in std::iter::once(&l.text).chain(&l.extra) {
            assert!(
                !text.contains("Abschlussbuchung")
                    && !text.contains("Reklamationen")
                    && !text.contains("Kontostand")
                    && !text.contains("Informationen"),
                "{text}"
            );
        }
    }
}

#[test]
fn an_online_payment_takes_its_merchant_from_the_first_detail_line_and_drops_midnight() {
    let parsed = read(&statement_pages());
    assert_eq!(
        line(&parsed, "E-COMM 7,10").card,
        Some(Card {
            kind: CardKind::Online,
            date: date(2026, 9, 28),
            time: None,
            merchant: "DEMO IMBISS DORNBIRN 6850".into(),
            original: None,
        })
    );
}

#[test]
fn a_terminal_payment_keeps_its_time_and_takes_its_merchant_from_the_first_detail_line() {
    let parsed = read(&statement_pages());
    assert_eq!(
        line(&parsed, "POS 20,00").card,
        Some(Card {
            kind: CardKind::Terminal,
            date: date(2026, 9, 28),
            time: Some("19:42".into()),
            merchant: "BUCHLADEN MUSTER DORNBIRN 6850".into(),
            original: None,
        })
    );
}

#[test]
fn a_terminal_name_and_number_before_the_card_is_a_terminal_payment_without_original_amount() {
    let parsed = read(&statement_pages());
    assert_eq!(
        line(&parsed, "BAECKEREI").card,
        Some(Card {
            kind: CardKind::Terminal,
            date: date(2026, 9, 30),
            time: Some("10:54".into()),
            merchant: "BAECKEREI".into(),
            original: None,
        })
    );
}

#[test]
fn a_foreign_currency_payment_has_its_original_amount_and_fx_line() {
    let parsed = read(&statement_pages());
    let lumen = line(&parsed, "E-COMM 24,00");
    assert_eq!(
        lumen.card,
        Some(Card {
            kind: CardKind::Online,
            date: date(2026, 9, 29),
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
            rate_date: date(2026, 9, 29),
        })
    );
}

#[test]
fn card_lines_have_no_counterparty_and_other_bookings_have_no_card() {
    let parsed = read(&statement_pages());
    for l in &parsed.lines {
        let is_card = ["E-COMM", "POS", "BAECKEREI"]
            .iter()
            .any(|p| l.text.starts_with(p));
        assert_eq!(l.card.is_some(), is_card, "{}", l.text);
        if is_card {
            assert_eq!((&l.counterparty, &l.counterparty_iban), (&None, &None));
        }
    }
}

#[test]
fn the_counterparty_of_a_transfer_is_its_first_detail_line_and_none_without_details() {
    let parsed = read(&statement_pages());
    for (text, expected) in [
        ("Gehalt", Some("Example GmbH")),
        ("Dauerauftrag", Some("Hausverwaltung Muster")),
        ("Kontoführung", None),
    ] {
        assert_eq!(
            line(&parsed, text).counterparty.as_deref(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn a_card_date_in_december_of_a_statement_running_into_january_is_in_the_year_before() {
    let pages = one_page(&[
        " Gehalt 1/2027                                    05.01.2027                2.000,00",
        " E-COMM       10,00 AT  K1   30.12. SHOP WIEN 040   31.12.2026                10,00-",
    ]);
    let parsed = read(&pages);
    assert_eq!((parsed.from, parsed.to), (date(2026, 12, 31), date(2027, 1, 5)));
    assert_eq!(
        line(&parsed, "E-COMM").card.as_ref().map(|c| c.date),
        Some(date(2026, 12, 30))
    );
}

#[test]
fn the_statement_number_reads_with_or_without_a_space_after_nr() {
    for label in ["Kontoauszug Nr. 004/2026", "Kontoauszug Nr.004/2026"] {
        let mut pages = statement_pages();
        replace_first_page_line(&mut pages, "Kontoauszug", label);
        let parsed = read(&pages);
        assert_eq!((parsed.number, parsed.year), (4, 2026), "{label}");
    }
}

#[test]
fn a_missing_header_part_or_sum_is_reported_by_name() {
    for (prefix, replacement, missing) in [
        ("Kontoauszug", Some("Kontoauszug"), "Kontoauszug Nr."),
        ("IBAN:", None, "IBAN"),
        ("Alter Kontostand", None, "Alter Kontostand"),
        ("Gutschriften", None, "Gutschriften"),
        ("Belastungen", None, "Belastungen"),
        ("Neuer Kontostand", None, "Neuer Kontostand"),
    ] {
        let mut pages = statement_pages();
        match replacement {
            Some(text) => replace_first_page_line(&mut pages, prefix, text),
            None => without_first_page_line(&mut pages, prefix),
        }
        assert_eq!(
            george::parse(&pages),
            Err(ParseError::Missing(missing.into())),
            "{prefix}"
        );
    }
}

#[test]
fn a_statement_without_bookings_is_missing_its_bookings() {
    assert_eq!(
        george::parse(&one_page(&[])),
        Err(ParseError::Missing("booking".into()))
    );
}

#[test]
fn an_iban_with_a_wrong_checksum_is_unreadable() {
    let mut pages = statement_pages();
    replace_first_page_line(&mut pages, "IBAN:", "IBAN:  AT251999900000000001");
    assert_eq!(
        george::parse(&pages),
        Err(ParseError::Unreadable("IBAN AT251999900000000001".into()))
    );
}

#[test]
fn only_pages_with_the_column_titles_and_an_iban_are_recognized() {
    let mut no_iban = statement_pages();
    without_first_page_line(&mut no_iban, "IBAN:");
    let mut no_columns = statement_pages();
    without_first_page_line(&mut no_columns, "Buchungstext");
    assert!(george::recognizes(&statement_pages()));
    for pages in [no_iban, no_columns, Vec::new(), vec![Vec::new()]] {
        assert!(!george::recognizes(&pages));
    }
}

#[test]
fn each_layout_is_read_by_its_own_parser() {
    let george_pages = statement_pages();
    assert_eq!(parse(&george_pages).unwrap().layout, "sparkasse-george-1");
    assert_eq!(parse(&george_pages), george::parse(&george_pages));

    let old_pages = demo_month(10, 2026, 10, 141_230).text_pages();
    assert!(!george::recognizes(&old_pages));
    assert!(sparkasse::recognizes(&old_pages));
    assert_eq!(parse(&old_pages).unwrap().layout, "sparkasse-erste-1");
}

#[test]
fn a_trailing_minus_makes_a_printed_amount_negative_and_a_second_sign_is_rejected() {
    for (text, expected) in [
        ("980,00-", Some(-98_000)),
        ("1.234,56-", Some(-123_456)),
        ("-980,00-", None),
        ("+980,00-", None),
    ] {
        assert_eq!(amount(text), expected, "{text}");
    }
}
