//! Synthetic bank statements in the layout Digitales Banking prints, from invented data: fixtures for the parser
//! and the debug build's demo statements. Fixed-width Courier text, so the layout reads back column for column.

use chrono::{Datelike, NaiveDate};

use crate::platform::pdf::write::{self, A4_HEIGHT, Font, Page, Run};

/// Characters per line; amounts end at this column.
pub const WIDTH: usize = 86;
/// The column the value date starts in.
pub const VALUE_COLUMN: usize = 49;
/// The column the header's labels and the right-hand amounts start in.
const LABEL_COLUMN: usize = 46;
/// Booking text longer than this is cut, keeping a gap before the value date.
const TEXT_LIMIT: usize = VALUE_COLUMN - 3;
const LINES_PER_PAGE: usize = 62;
const FONT_SIZE: f32 = 9.0;
const LINE_HEIGHT: f32 = 11.5;
const LEFT: f32 = 40.0;
const TOP: f32 = A4_HEIGHT - 50.0;

#[derive(Clone, Debug, PartialEq)]
pub struct StatementLine {
    /// The booking text on the line with the amount, e.g. `E-COMM 24,00 USD K1 08.11. 00:00`.
    pub text: String,
    /// The lines below it: merchant or counterparty, references, the FX line.
    pub extra: Vec<String>,
    pub value_date: NaiveDate,
    /// Signed: credits positive, debits negative.
    pub cents: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub bank: String,
    pub holder: String,
    /// Grouped in fours; printed without spaces.
    pub iban: String,
    pub bic: String,
    pub number: u32,
    /// The period the statement was made for. It isn't printed: the parser takes the first and last value date.
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub old_balance: i64,
    pub lines: Vec<StatementLine>,
}

/// `1.234,56` with an optional sign, as a bank prints amounts.
pub fn amount(cents: i64, signed: bool) -> String {
    let sign = match (signed, cents < 0) {
        (_, true) => "-",
        (true, false) => "+",
        (false, false) => "",
    };
    format!("{sign}{}", unsigned(cents))
}

fn unsigned(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{grouped},{:02}", abs % 100)
}

/// `1.126,88-` for a debit, `2.850,00` for a credit, as this layout prints them.
pub fn printed(cents: i64) -> String {
    let minus = if cents < 0 { "-" } else { "" };
    format!("{}{minus}", unsigned(cents))
}

fn day(date: NaiveDate) -> String {
    date.format("%d.%m.%Y").to_string()
}

/// `left` with `right` ending at the last column.
fn row(left: &str, right: &str) -> String {
    let pad = WIDTH.saturating_sub(left.chars().count() + right.chars().count());
    format!("{left}{}{right}", " ".repeat(pad.max(1)))
}

/// `left` up to the label column, then `label`, then `right` ending at the last column.
fn labelled(left: &str, label: &str, right: &str) -> String {
    row(&format!("{left:<LABEL_COLUMN$}{label}"), right)
}

/// The balances and sums printed in the first page's header.
#[derive(Clone, Copy)]
struct Sums {
    old_balance: i64,
    credits: i64,
    debits: i64,
    new_balance: i64,
}

impl Statement {
    pub fn credits(&self) -> i64 {
        self.lines.iter().map(|l| l.cents).filter(|c| *c > 0).sum()
    }

    pub fn debits(&self) -> i64 {
        self.lines.iter().map(|l| l.cents).filter(|c| *c < 0).sum()
    }

    pub fn new_balance(&self) -> i64 {
        self.old_balance + self.credits() + self.debits()
    }

    /// `Nr. 003/2026`, numbered within the year of the statement's end.
    pub fn number_label(&self) -> String {
        format!("Nr. {:03}/{}", self.number, self.to.year())
    }

    fn compact_iban(&self) -> String {
        self.iban.split_whitespace().collect()
    }

    fn sums(&self) -> Sums {
        Sums {
            old_balance: self.old_balance,
            credits: self.credits(),
            debits: self.debits(),
            new_balance: self.new_balance(),
        }
    }

    fn bank_lines(&self) -> Vec<String> {
        vec![
            format!("{:>WIDTH$}", self.bank),
            format!("{:>WIDTH$}", "Musterplatz 1"),
            format!("{:>WIDTH$}", "1010 Musterstadt"),
        ]
    }

    fn columns() -> String {
        row(
            &format!(
                "{:<VALUE_COLUMN$}Valuta/Value",
                " Buchungstext/Booking Text"
            ),
            "Beträge/Amounts in EUR",
        )
    }

    fn first_header(&self, sums: Sums) -> Vec<String> {
        let mut out = self.bank_lines();
        out.extend([
            labelled("", "Kontoauszug", &self.number_label()),
            labelled("", "Account Statement", ""),
            labelled("", &format!("IBAN:  {}", self.compact_iban()), ""),
            labelled("", &format!("BIC:   {}", self.bic), ""),
            format!("   {}", self.holder),
            "   Zuletzt gültiger Kontowortlaut".to_string(),
            labelled("", "Alter Kontostand", &printed(sums.old_balance)),
            labelled("   last valid account name", "Old Balance", ""),
            "   Ausdruck Digitales Banking".to_string(),
            labelled("", "Gutschriften", &printed(sums.credits)),
            labelled("", "Credits", ""),
            labelled("", "Belastungen", &printed(sums.debits)),
            labelled("", "Debits", ""),
            labelled("", "Neuer Kontostand", &printed(sums.new_balance)),
            labelled("", "New Balance", ""),
            Self::columns(),
        ]);
        out.into_iter().map(|l| l.trim_end().to_string()).collect()
    }

    fn later_header(&self) -> Vec<String> {
        let mut out = self.bank_lines();
        out.push(Self::columns());
        out
    }

    /// The IBAN, print date and time, voucher count, statement number and page.
    fn footer(&self, page: usize) -> Vec<String> {
        vec![
            format!(
                "{}  {} 21:00         0                   {:03}               {page}",
                self.compact_iban(),
                day(self.to),
                self.number
            ),
            "IBAN                  Datum/Date Uhrzeit/Time  Belege(*)/Vouchers  Auszug/Statement  Seite/Page"
                .to_string(),
        ]
    }

    fn entry(line: &StatementLine) -> Vec<String> {
        let text: String = line.text.chars().take(TEXT_LIMIT).collect();
        let left = format!(
            " {text:<width$}{}",
            day(line.value_date),
            width = VALUE_COLUMN - 1
        );
        let mut out = vec![row(&left, &printed(line.cents))];
        out.extend(line.extra.iter().map(|e| format!("  {e}")));
        out
    }

    fn closing(sums: Sums) -> Vec<String> {
        vec![row(
            " Neuer Kontostand/New Balance",
            &printed(sums.new_balance),
        )]
    }

    /// The printed text, page by page; an entry never breaks across pages.
    pub fn text_pages(&self) -> Vec<Vec<String>> {
        self.text_pages_with(self.sums())
    }

    fn text_pages_with(&self, sums: Sums) -> Vec<Vec<String>> {
        let footer = self.footer(1).len();
        let room = |page: usize| {
            let header = if page == 0 {
                self.first_header(sums).len()
            } else {
                self.later_header().len()
            };
            LINES_PER_PAGE - header - footer
        };
        let mut chunks: Vec<Vec<String>> = vec![Vec::new()];
        let mut blocks: Vec<Vec<String>> = self.lines.iter().map(Self::entry).collect();
        blocks.push(Self::closing(sums));
        for block in blocks {
            let page = chunks.len() - 1;
            if chunks
                .last()
                .is_some_and(|c| c.len() + block.len() > room(page))
            {
                chunks.push(Vec::new());
            }
            if let Some(chunk) = chunks.last_mut() {
                chunk.extend(block);
            }
        }
        chunks
            .into_iter()
            .enumerate()
            .map(|(i, chunk)| {
                let mut page = if i == 0 {
                    self.first_header(sums)
                } else {
                    self.later_header()
                };
                page.extend(chunk);
                page.extend(self.footer(i + 1));
                page
            })
            .collect()
    }

    pub fn pdf(&self) -> Vec<u8> {
        render(self.text_pages())
    }

    /// The statement with one line's amount changed but the original sums printed, as a forged copy would be.
    pub fn tampered_pdf(&self, line: usize, cents: i64) -> Vec<u8> {
        let mut changed = self.clone();
        if let Some(l) = changed.lines.get_mut(line) {
            l.cents = cents;
        }
        render(changed.text_pages_with(self.sums()))
    }
}

fn render(text_pages: Vec<Vec<String>>) -> Vec<u8> {
    let pages: Vec<Page> = text_pages
        .into_iter()
        .map(|lines| {
            lines
                .into_iter()
                .enumerate()
                .filter(|(_, text)| !text.is_empty())
                .map(|(i, text)| Run {
                    x: LEFT,
                    y: TOP - i as f32 * LINE_HEIGHT,
                    size: FONT_SIZE,
                    font: Font::Courier,
                    text,
                })
                .collect()
        })
        .collect();
    write::write(&pages)
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or_default()
}

fn line(text: &str, extra: &[&str], value_date: NaiveDate, cents: i64) -> StatementLine {
    StatementLine {
        text: text.into(),
        extra: extra.iter().map(|e| (*e).to_string()).collect(),
        value_date,
        cents,
    }
}

/// A month of the made-up Private account: salary, rent, card payments with the merchant on the line below, a
/// subscription and a USD charge with its FX line.
pub fn demo_month(number: u32, year: i32, month: u32, old_balance: i64) -> Statement {
    let d = |day| date(year, month, day);
    let card = |kind: &str, paid: &str, country: &str, day: u32, time: &str| {
        format!("{kind} {paid:>8} {country:<3} K1   {day:02}.{month:02}. {time}")
    };
    let salary = format!("GEHALT {month}/{year}");
    let rent = format!("Miete {month:02}/{year}");
    Statement {
        bank: "Example Sparkasse".into(),
        holder: "Alex Beispiel".into(),
        iban: "AT24 1999 9000 0000 0001".into(),
        bic: "EXSPATWWXXX".into(),
        number,
        from: d(1),
        to: date(year, month, 28),
        old_balance,
        lines: vec![
            line(
                &salary,
                &["NORDWIND SOFTWARE GMBH", "260101000000001", &salary],
                d(1),
                285_000,
            ),
            line(&rent, &["HAUSVERWALTUNG MUSTER", &rent], d(3), -98_000),
            line(
                &card("POS   ", "58,30", "AT", 4, "17:40"),
                &["TANKSTELLE RHEIN LUSTENAU 6890 040"],
                d(5),
                -5_830,
            ),
            line(
                &card("E-COMM", "12,99", "AT", 6, "00:00"),
                &["STREAMFLIX WIEN 1010 040"],
                d(7),
                -1_299,
            ),
            line(
                &card("E-COMM", "24,00", "USD", 8, "00:00"),
                &[
                    "LUMEN AI SAN FRANCISCO 840",
                    &format!("SPESEN: 0,49 KURS: 1,0912 VOM: {}", day(d(6))),
                ],
                d(10),
                -2_248,
            ),
            line(
                "RE 2026-0412",
                &["BUCHHANDLUNG SEITENWEISE", "RE 2026-0412"],
                d(14),
                -4_690,
            ),
            line(
                &card("POS   ", "6,21", "AT", 20, "09:12"),
                &["CAFE LINDE DORNBIRN 6850 040"],
                d(21),
                -621,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statement() -> Statement {
        demo_month(10, 2026, 10, 141_230)
    }

    fn long_statement() -> Statement {
        let mut statement = statement();
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

    #[test]
    fn amounts_are_grouped_with_comma_decimals_and_optional_sign() {
        for (cents, signed, expected) in [
            (123_456, false, "1.234,56"),
            (-1_290, true, "-12,90"),
            (-1_290, false, "-12,90"),
            (240_000, true, "+2.400,00"),
            (5, false, "0,05"),
            (100_000_000, false, "1.000.000,00"),
        ] {
            assert_eq!(amount(cents, signed), expected, "{cents} {signed}");
        }
    }

    #[test]
    fn totals_split_credits_from_debits_and_give_the_new_balance() {
        let statement = statement();
        assert_eq!(statement.credits(), 285_000);
        assert_eq!(statement.debits(), -112_688);
        assert_eq!(statement.new_balance(), 313_542);
    }

    #[test]
    fn the_number_label_is_padded_and_uses_the_end_year() {
        assert_eq!(statement().number_label(), "Nr. 010/2026");
    }

    #[test]
    fn debits_print_a_trailing_minus_and_credits_print_unsigned() {
        for (cents, expected) in [
            (285_000, "2.850,00"),
            (-112_688, "1.126,88-"),
            (141_230, "1.412,30"),
            (0, "0,00"),
            (5, "0,05"),
            (-5, "0,05-"),
            (100_000_000, "1.000.000,00"),
        ] {
            assert_eq!(printed(cents), expected, "{cents}");
        }
    }

    fn bank_lines() -> Vec<String> {
        ["Example Sparkasse", "Musterplatz 1", "1010 Musterstadt"]
            .iter()
            .map(|l| format!("{l:>86}"))
            .collect()
    }

    fn at_label(text: &str) -> String {
        format!("{:<46}{text}", "")
    }

    fn with_amount(left: &str, amount: &str) -> String {
        format!(
            "{left}{amount:>width$}",
            width = WIDTH - left.chars().count()
        )
    }

    fn columns() -> String {
        with_amount(
            &format!("{:<49}Valuta/Value", " Buchungstext/Booking Text"),
            "Beträge/Amounts in EUR",
        )
    }

    fn footer(page: usize) -> Vec<String> {
        vec![
            format!(
                "AT241999900000000001  28.10.2026 21:00         0                   010               {page}"
            ),
            "IBAN                  Datum/Date Uhrzeit/Time  Belege(*)/Vouchers  Auszug/Statement  Seite/Page"
                .to_string(),
        ]
    }

    #[test]
    fn a_short_statement_is_one_page_with_the_full_header_entries_closing_and_footer() {
        let pages = statement().text_pages();
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        let mut header = bank_lines();
        header.extend([
            with_amount(&at_label("Kontoauszug"), "Nr. 010/2026"),
            at_label("Account Statement"),
            at_label("IBAN:  AT241999900000000001"),
            at_label("BIC:   EXSPATWWXXX"),
            "   Alex Beispiel".to_string(),
            "   Zuletzt gültiger Kontowortlaut".to_string(),
            with_amount(&at_label("Alter Kontostand"), "1.412,30"),
            format!("{:<46}Old Balance", "   last valid account name"),
            "   Ausdruck Digitales Banking".to_string(),
            with_amount(&at_label("Gutschriften"), "2.850,00"),
            at_label("Credits"),
            with_amount(&at_label("Belastungen"), "1.126,88-"),
            at_label("Debits"),
            with_amount(&at_label("Neuer Kontostand"), "3.135,42"),
            at_label("New Balance"),
            columns(),
        ]);
        assert_eq!(page[..header.len()], header);
        let closing = with_amount(" Neuer Kontostand/New Balance", "3.135,42");
        assert_eq!(page[page.len() - 3], closing);
        assert_eq!(page[page.len() - 2..], footer(1));
    }

    #[test]
    fn an_entry_ends_its_amount_at_the_width_and_starts_its_value_date_at_the_column() {
        let page = &statement().text_pages()[0];
        let salary = &page[19];
        assert_eq!(
            *salary,
            with_amount(&format!(" {:<48}01.10.2026", "GEHALT 10/2026"), "2.850,00")
        );
        assert_eq!(salary.chars().count(), WIDTH);
        assert_eq!(&salary[VALUE_COLUMN..VALUE_COLUMN + 10], "01.10.2026");
        let rent = &page[23];
        assert_eq!(
            *rent,
            with_amount(&format!(" {:<48}03.10.2026", "Miete 10/2026"), "980,00-")
        );
    }

    #[test]
    fn extra_lines_are_indented_by_two_spaces() {
        let page = &statement().text_pages()[0];
        assert_eq!(
            page[20..23],
            [
                "  NORDWIND SOFTWARE GMBH",
                "  260101000000001",
                "  GEHALT 10/2026"
            ]
        );
    }

    #[test]
    fn booking_text_longer_than_the_limit_is_cut_before_the_value_date_column() {
        let mut statement = statement();
        statement.lines = vec![StatementLine {
            text: "X".repeat(60),
            extra: Vec::new(),
            value_date: date(2026, 10, 1),
            cents: -100,
        }];
        let page = &statement.text_pages()[0];
        assert_eq!(
            page[19],
            with_amount(&format!(" {}  01.10.2026", "X".repeat(46)), "1,00-")
        );
    }

    #[test]
    fn a_long_statement_spans_pages_with_the_footer_on_every_page() {
        let pages = long_statement().text_pages();
        assert!(pages.len() >= 3);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page[page.len() - 2..], footer(i + 1), "page {}", i + 1);
        }
    }

    #[test]
    fn later_pages_repeat_only_the_bank_lines_and_the_column_titles() {
        let pages = long_statement().text_pages();
        for page in &pages[1..] {
            let mut header = bank_lines();
            header.push(columns());
            assert_eq!(page[..header.len()], header);
            assert!(!page.iter().any(|l| l.contains("Alter Kontostand")));
        }
    }

    #[test]
    fn the_closing_line_is_only_on_the_last_page() {
        let pages = long_statement().text_pages();
        let closing = with_amount(" Neuer Kontostand/New Balance", "2.835,42");
        let last = pages.len() - 1;
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page.contains(&closing), i == last, "page {}", i + 1);
        }
    }

    #[test]
    fn no_entry_is_split_across_pages() {
        let pages = long_statement().text_pages();
        let mut entries = 0;
        for page in &pages {
            let start = page
                .iter()
                .position(|l| l.starts_with(" Buchungstext/Booking Text"))
                .unwrap()
                + 1;
            let body = &page[start..];
            assert!(!body[0].starts_with("  "), "page starts with an extra line");
            let rates = body.iter().filter(|l| l.starts_with(" Rate ")).count();
            let a = body.iter().filter(|l| l.trim() == "A").count();
            let b = body.iter().filter(|l| l.trim() == "B").count();
            assert_eq!((a, b), (rates, rates));
            entries += rates;
        }
        assert_eq!(entries, 30);
    }

    fn read_pdf(bytes: &[u8]) -> crate::statements::parse::model::ParsedStatement {
        let pages = crate::platform::pdf::Pdf::from_bytes(bytes)
            .unwrap()
            .text_pages()
            .unwrap();
        crate::statements::parse::parse(&pages).unwrap()
    }

    #[test]
    fn a_tampered_pdf_changes_the_line_but_keeps_the_original_sums() {
        let original = statement();
        let forged = read_pdf(&original.tampered_pdf(1, -90_000));
        let untouched = read_pdf(&original.pdf());
        assert_eq!(forged.lines[1].cents, -90_000);
        assert_eq!(untouched.lines[1].cents, -98_000);
        assert_eq!(
            (forged.credits, forged.debits, forged.new_balance),
            (
                original.credits(),
                original.debits(),
                original.new_balance()
            )
        );
        assert_eq!(forged.lines[0], untouched.lines[0]);
    }

    #[test]
    fn tampering_a_line_that_does_not_exist_changes_nothing() {
        let original = statement();
        assert_eq!(
            read_pdf(&original.tampered_pdf(99, -1)),
            read_pdf(&original.pdf())
        );
    }
}
