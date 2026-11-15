//! Money as integer cents plus a currency code (ADR 0012), and how amounts read (ADR 0010).

use serde::{Deserialize, Serialize};

use crate::settings::NumberFormat;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Money {
    pub cents: i64,
    /// ISO 4217, upper case (`EUR`).
    pub currency: String,
}

impl Money {
    pub fn eur(cents: i64) -> Self {
        Self {
            cents,
            currency: "EUR".into(),
        }
    }
}

/// `1234.56` → `1,234.56` or `1.234,56`; always two decimals.
fn number(cents: i64, format: NumberFormat) -> String {
    let (group, decimal) = match format {
        NumberFormat::DotDecimal => (',', '.'),
        NumberFormat::CommaDecimal => ('.', ','),
    };
    let whole = (cents.unsigned_abs() / 100).to_string();
    let mut grouped = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(group);
        }
        grouped.push(digit);
    }
    format!("{grouped}{decimal}{:02}", cents.unsigned_abs() % 100)
}

/// An amount as the window shows it: `€1,234.56`, `−€12.00` (a true minus sign), `USD 24.00` for other
/// currencies.
pub fn format_money(money: &Money, format: NumberFormat) -> String {
    let sign = if money.cents < 0 { "\u{2212}" } else { "" };
    let digits = number(money.cents, format);
    if money.currency == "EUR" {
        format!("{sign}€{digits}")
    } else {
        format!("{sign}{} {digits}", money.currency)
    }
}

/// Without a currency, as in a balance equation: `1,234.56`, `−12.00`.
pub fn format_plain(cents: i64, format: NumberFormat) -> String {
    let sign = if cents < 0 { "\u{2212}" } else { "" };
    format!("{sign}{}", number(cents, format))
}

/// With an explicit sign for bank lines: `+€1,200.00`, `−€9.99`.
pub fn format_signed(money: &Money, format: NumberFormat) -> String {
    if money.cents > 0 {
        format!("+{}", format_money(money, format))
    } else {
        format_money(money, format)
    }
}

/// The value to type into FinanzOnline: decimal comma, no grouping, no currency (`1228,40`).
pub fn copy_value(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{},{:02}", abs / 100, abs % 100)
}

/// Reads an amount the way people type it: `12`, `12.5`, `1,234.56`, `1.234,56`, `12,50`, with an optional
/// sign and `€` on either side. The last `.` or `,` followed by one or two digits is the decimal mark; any other
/// mark must group thousands, so a typo like `1,2,3` is refused rather than read as 12.30.
pub fn parse_cents(text: &str) -> Option<i64> {
    let trimmed = text.trim().trim_matches('€').trim();
    let (negative, rest) = match trimmed.strip_prefix(['-', '\u{2212}']) {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let body = rest.trim().trim_start_matches('€').trim();
    if body.is_empty()
        || !body
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
    {
        return None;
    }
    let decimal_at = body
        .rfind(['.', ','])
        .filter(|&i| (1..=2).contains(&(body.len() - i - 1)));
    let (whole, fraction, decimal) = match decimal_at {
        Some(i) => (&body[..i], &body[i + 1..], body[i..].chars().next()),
        None => (body, "", None),
    };
    let whole_digits = ungrouped(whole, decimal)?;
    let whole: i64 = if whole_digits.is_empty() {
        0
    } else {
        whole_digits.parse().ok()?
    };
    let fraction: i64 = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        _ => fraction.parse().ok()?,
    };
    let cents = whole.checked_mul(100)?.checked_add(fraction)?;
    Some(if negative { -cents } else { cents })
}

/// The digits of a whole part: one grouping mark, not the decimal one, with groups of three after the first.
fn ungrouped(whole: &str, decimal: Option<char>) -> Option<String> {
    let Some(mark) = whole.chars().find(|c| !c.is_ascii_digit()) else {
        return Some(whole.to_string());
    };
    if Some(mark) == decimal || whole.chars().any(|c| !c.is_ascii_digit() && c != mark) {
        return None;
    }
    let groups: Vec<&str> = whole.split(mark).collect();
    let first_fits = (1..=3).contains(&groups[0].len());
    (first_fits && groups[1..].iter().all(|g| g.len() == 3)).then(|| groups.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOT: NumberFormat = NumberFormat::DotDecimal;
    const COMMA: NumberFormat = NumberFormat::CommaDecimal;

    fn usd(cents: i64) -> Money {
        Money {
            cents,
            currency: "USD".into(),
        }
    }

    #[test]
    fn eur_builds_euro_money() {
        assert_eq!(
            Money::eur(500),
            Money {
                cents: 500,
                currency: "EUR".into()
            }
        );
    }

    #[test]
    fn money_formats_with_the_chosen_separators() {
        for (money, format, expected) in [
            (Money::eur(123_456), DOT, "€1,234.56"),
            (Money::eur(123_456), COMMA, "€1.234,56"),
            (Money::eur(5), DOT, "€0.05"),
            (Money::eur(0), COMMA, "€0,00"),
            (Money::eur(100_000_000), DOT, "€1,000,000.00"),
            (Money::eur(123_456_789_00), COMMA, "€123.456.789,00"),
            (Money::eur(99_999), DOT, "€999.99"),
            (Money::eur(100_000), DOT, "€1,000.00"),
        ] {
            assert_eq!(
                format_money(&money, format),
                expected,
                "{money:?} {format:?}"
            );
        }
    }

    #[test]
    fn negative_amounts_use_a_true_minus_sign() {
        assert_eq!(format_money(&Money::eur(-1200), DOT), "\u{2212}€12.00");
        assert_eq!(format_money(&usd(-2400), COMMA), "\u{2212}USD 24,00");
    }

    #[test]
    fn other_currencies_show_their_code_before_the_amount() {
        assert_eq!(format_money(&usd(2400), DOT), "USD 24.00");
    }

    #[test]
    fn signed_amounts_carry_a_plus_only_when_positive() {
        assert_eq!(format_signed(&Money::eur(120_000), DOT), "+€1,200.00");
        assert_eq!(format_signed(&Money::eur(-999), DOT), "\u{2212}€9.99");
        assert_eq!(format_signed(&Money::eur(0), DOT), "€0.00");
        assert_eq!(format_signed(&usd(100), COMMA), "+USD 1,00");
    }

    #[test]
    fn plain_amounts_have_no_currency_and_a_true_minus_sign() {
        for (cents, format, expected) in [
            (123_456, DOT, "1,234.56"),
            (123_456, COMMA, "1.234,56"),
            (-1_200, DOT, "\u{2212}12.00"),
            (-123_456, COMMA, "\u{2212}1.234,56"),
            (0, DOT, "0.00"),
            (5, COMMA, "0,05"),
            (-5, DOT, "\u{2212}0.05"),
            (100_000_000, DOT, "1,000,000.00"),
        ] {
            assert_eq!(format_plain(cents, format), expected, "{cents} {format:?}");
        }
    }

    #[test]
    fn copy_values_use_a_decimal_comma_and_no_grouping() {
        for (cents, expected) in [
            (122_840, "1228,40"),
            (-500, "-5,00"),
            (7, "0,07"),
            (0, "0,00"),
            (123_456_789, "1234567,89"),
            (-7, "-0,07"),
        ] {
            assert_eq!(copy_value(cents), expected, "{cents}");
        }
    }

    #[test]
    fn typed_amounts_are_read_in_every_common_notation() {
        for (text, expected) in [
            ("12", 1200),
            ("12.5", 1250),
            ("12,50", 1250),
            ("12.50", 1250),
            ("1,234.56", 123_456),
            ("1.234,56", 123_456),
            ("€ 9,99", 999),
            ("€9.99", 999),
            ("  7  ", 700),
            ("-3", -300),
            ("\u{2212}3", -300),
            ("- 3,50", -350),
            ("+4", 400),
            ("1.234", 123_400),
            ("1,234", 123_400),
            ("1.234.567", 123_456_700),
            (".5", 50),
            ("0,07", 7),
            (".50", 50),
            ("12.345.678,90", 1_234_567_890),
            ("12,50 €", 1250),
            ("€12,50", 1250),
            ("-€5", -500),
            ("€-5", -500),
            ("- 5", -500),
        ] {
            assert_eq!(parse_cents(text), Some(expected), "{text:?}");
        }
    }

    #[test]
    fn badly_grouped_amounts_are_refused() {
        for text in ["1,2,3", "1.23.45", "12.34,5", "1.2345,00", "12,345,6"] {
            assert_eq!(parse_cents(text), None, "{text:?}");
        }
    }

    #[test]
    fn text_that_is_not_an_amount_reads_as_none() {
        for text in [
            "",
            "   ",
            "abc",
            "1.2.3x",
            "€",
            "-",
            "12 EUR",
            "1e5",
            "99999999999999999999",
        ] {
            assert_eq!(parse_cents(text), None, "{text:?}");
        }
    }
}
