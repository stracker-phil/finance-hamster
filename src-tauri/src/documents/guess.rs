//! Best-effort facts read from an invoice's text: vendor, total, invoice number and date; any may be missing.

use chrono::NaiveDate;

use crate::money::parse_cents;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Guess {
    pub vendor: Option<String>,
    /// Positive, in `currency`.
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub invoice_number: Option<String>,
    pub invoice_date: Option<NaiveDate>,
}

const TOTAL_WORDS: [&str; 6] = [
    "gesamtbetrag",
    "gesamt",
    "summe",
    "total",
    "amount due",
    "rechnungsbetrag",
];
const NUMBER_WORDS: [&str; 6] = [
    "invoice no",
    "invoice number",
    "invoice #",
    "rechnungsnummer",
    "rechnung nr",
    "rechnungs-nr",
];

fn amount_in(line: &str) -> Option<i64> {
    line.split_whitespace()
        .rev()
        .find_map(|word| {
            let word = word.trim_matches(|c: char| !c.is_ascii_digit() && c != ',' && c != '.');
            (word.contains(',') || word.contains('.'))
                .then(|| parse_cents(word))
                .flatten()
        })
        .map(i64::abs)
}

fn currency_in(line: &str) -> Option<String> {
    let upper = line.to_uppercase();
    ["EUR", "USD", "GBP", "CHF"]
        .into_iter()
        .find(|code| upper.contains(code))
        .map(String::from)
        .or_else(|| {
            [('€', "EUR"), ('$', "USD"), ('£', "GBP")]
                .into_iter()
                .find(|(sign, _)| line.contains(*sign))
                .map(|(_, code)| code.to_string())
        })
}

fn date_in(text: &str) -> Option<NaiveDate> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .find_map(|word| {
            let word = word.trim_matches(|c: char| !c.is_ascii_digit());
            NaiveDate::parse_from_str(word, "%d.%m.%Y")
                .or_else(|_| NaiveDate::parse_from_str(word, "%Y-%m-%d"))
                .ok()
        })
}

fn number_after(line: &str, word: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let at = lower.find(word)? + word.len();
    line.get(at..)?
        .trim_start_matches(|c: char| c == ':' || c == '.' || c == '#' || c.is_whitespace())
        .split_whitespace()
        .next()
        .filter(|n| n.chars().any(|c| c.is_ascii_digit()))
        .map(String::from)
}

/// `vendors` are the known names; one found in the text wins over the first line.
pub fn guess(lines: &[String], vendors: &[String]) -> Guess {
    let text = lines.join("\n");
    let lower = text.to_lowercase();
    let vendor = vendors
        .iter()
        .find(|v| !v.trim().is_empty() && lower.contains(&v.to_lowercase()))
        .cloned()
        .or_else(|| {
            lines
                .iter()
                .map(|l| l.trim())
                .find(|l| !l.is_empty())
                .filter(|l| l.chars().count() <= 40 && l.chars().any(char::is_alphabetic))
                .map(String::from)
        });
    // The last total line: a subtotal usually comes first.
    let total_line = lines
        .iter()
        .filter(|l| {
            let l = l.to_lowercase();
            TOTAL_WORDS.iter().any(|w| l.contains(w))
        })
        .rfind(|l| amount_in(l).is_some());
    let invoice_number = lines
        .iter()
        .find_map(|l| NUMBER_WORDS.iter().find_map(|w| number_after(l, w)));
    Guess {
        vendor,
        amount_cents: total_line.and_then(|l| amount_in(l)),
        currency: total_line.and_then(|l| currency_in(l)),
        invoice_number,
        invoice_date: date_in(&text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|l| (*l).to_string()).collect()
    }

    fn names(vendors: &[&str]) -> Vec<String> {
        lines(vendors)
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn nothing_is_guessed_from_an_empty_text() {
        assert_eq!(guess(&[], &[]), Guess::default());
        assert_eq!(guess(&lines(&["", "  "]), &[]), Guess::default());
    }

    #[test]
    fn a_whole_invoice_gives_every_fact() {
        let text = lines(&[
            "Pixelforge Ltd.",
            "Invoice No: PF-2026-1109",
            "Date: 2026-11-09",
            "Subtotal USD 100.00",
            "Total USD 129.00",
        ]);

        assert_eq!(
            guess(&text, &[]),
            Guess {
                vendor: Some("Pixelforge Ltd.".into()),
                amount_cents: Some(12_900),
                currency: Some("USD".into()),
                invoice_number: Some("PF-2026-1109".into()),
                invoice_date: Some(day(2026, 11, 9)),
            }
        );
    }

    #[test]
    fn a_known_vendor_found_anywhere_in_the_text_wins_ignoring_case() {
        let text = lines(&["Order confirmation", "sold by NORDBOOK verlag"]);

        assert_eq!(
            guess(&text, &names(&["Pixelforge Ltd.", "Nordbook"])).vendor,
            Some("Nordbook".into())
        );
    }

    #[test]
    fn a_blank_vendor_name_never_matches() {
        let text = lines(&["Order confirmation"]);

        assert_eq!(
            guess(&text, &names(&[" ", ""])).vendor,
            Some("Order confirmation".into())
        );
    }

    #[test]
    fn the_first_non_empty_line_is_the_vendor_when_it_is_short_and_has_letters() {
        let long = "A".repeat(41);
        let fits = "A".repeat(40);
        for (text, expected) in [
            (vec!["", "  Nordbook  ", "Total 5,00"], Some("Nordbook")),
            (vec![long.as_str()], None),
            (vec![fits.as_str()], Some(fits.as_str())),
            (vec!["12345", "Nordbook"], None),
        ] {
            assert_eq!(
                guess(&lines(&text), &[]).vendor.as_deref(),
                expected,
                "{text:?}"
            );
        }
    }

    #[test]
    fn the_last_total_line_gives_the_amount() {
        let text = lines(&["Nordbook", "Subtotal 100,00", "VAT 20,00", "Total 120,00"]);

        let guessed = guess(&text, &[]);

        assert_eq!(guessed.amount_cents, Some(12_000));
        assert_eq!(guessed.currency, None);
    }

    #[test]
    fn every_total_word_marks_a_total_line() {
        for (line, cents) in [
            ("Gesamt 12,50", 1_250),
            ("Summe 1.234,56", 123_456),
            ("Total 7.00", 700),
            ("Amount due: 9.99", 999),
            ("Rechnungsbetrag 45,00", 4_500),
            ("Gesamtbetrag 7,00", 700),
            ("TOTAL -8,00", 800),
        ] {
            assert_eq!(
                guess(&lines(&["Nordbook", line]), &[]).amount_cents,
                Some(cents),
                "{line}"
            );
        }
    }

    #[test]
    fn a_total_line_without_an_amount_is_skipped() {
        let text = lines(&["Nordbook", "Total 5,00", "Total: see below"]);

        assert_eq!(guess(&text, &[]).amount_cents, Some(500));
    }

    #[test]
    fn without_a_total_line_there_is_no_amount_or_currency() {
        let text = lines(&["Nordbook", "Shipping EUR 4,90"]);

        let guessed = guess(&text, &[]);

        assert_eq!(guessed.amount_cents, None);
        assert_eq!(guessed.currency, None);
    }

    #[test]
    fn the_currency_comes_from_the_total_line_by_code_or_sign() {
        for (line, expected) in [
            ("Total USD 129.00", Some("USD")),
            ("Total 10.00 eur", Some("EUR")),
            ("Total CHF 10.00", Some("CHF")),
            ("Total GBP 10.00", Some("GBP")),
            ("Total $129.00", Some("USD")),
            ("Total £10.00", Some("GBP")),
            ("Total €10,00", Some("EUR")),
            ("Total 10,00", None),
        ] {
            assert_eq!(
                guess(&lines(&["Nordbook", line]), &[]).currency.as_deref(),
                expected,
                "{line}"
            );
        }
    }

    #[test]
    fn the_invoice_number_follows_its_label_and_needs_a_digit() {
        for (line, expected) in [
            ("Invoice No: PF-2026-1109", Some("PF-2026-1109")),
            ("Invoice Number 4711", Some("4711")),
            ("Invoice # 88", Some("88")),
            ("Rechnungsnummer: RE-5", Some("RE-5")),
            ("Rechnung Nr. 2026/17", Some("2026/17")),
            ("Rechnungs-Nr 12", Some("12")),
            ("invoice no: pending", None),
            ("Order 4711", None),
        ] {
            assert_eq!(
                guess(&lines(&["Nordbook", line]), &[])
                    .invoice_number
                    .as_deref(),
                expected,
                "{line}"
            );
        }
    }

    #[test]
    fn the_first_date_in_either_format_is_the_invoice_date() {
        for (line, expected) in [
            ("Date 13.11.2026", Some(day(2026, 11, 13))),
            ("Issued 2026-11-09, due later", Some(day(2026, 11, 9))),
            ("01.10.2026 and 2026-11-09", Some(day(2026, 10, 1))),
            ("Date 31.02.2026", None),
            ("No date here", None),
        ] {
            assert_eq!(
                guess(&lines(&["Nordbook", line]), &[]).invoice_date,
                expected,
                "{line}"
            );
        }
    }
}
