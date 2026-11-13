//! Synthetic bank statements in the Sparkasse/Erste layout, from invented data: fixtures for the parser and
//! the debug build's demo statements. Fixed-width Courier text, so the layout reads back column for column.

use chrono::{Datelike, NaiveDate};

use crate::platform::pdf::write::{self, A4_HEIGHT, Font, Page, Run};

/// Characters per line; amounts end at this column.
pub const WIDTH: usize = 86;
/// The column the value date starts in.
pub const VALUE_COLUMN: usize = 60;
/// Booking text longer than this is cut, keeping a gap before the value date.
const TEXT_LIMIT: usize = VALUE_COLUMN - 2;
const LINES_PER_PAGE: usize = 58;
const FONT_SIZE: f32 = 9.0;
const LINE_HEIGHT: f32 = 11.5;
const LEFT: f32 = 50.0;
const TOP: f32 = A4_HEIGHT - 56.0;

#[derive(Clone, Debug, PartialEq)]
pub struct StatementLine {
    /// The booking text on the line with the amount, e.g. `E-COMM 24,00 USD K1 08.11. LUMEN AI ...`.
    pub text: String,
    /// Indented lines below it: counterparty, references, the FX line.
    pub extra: Vec<String>,
    pub value_date: NaiveDate,
    /// Signed: credits positive, debits negative.
    pub cents: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub bank: String,
    pub holder: String,
    /// Grouped in fours, as printed.
    pub iban: String,
    pub bic: String,
    pub number: u32,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub old_balance: i64,
    pub lines: Vec<StatementLine>,
}

/// `1.234,56` with an optional sign, as the bank prints amounts.
pub fn amount(cents: i64, signed: bool) -> String {
    let abs = cents.unsigned_abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let sign = match (signed, cents < 0) {
        (_, true) => "-",
        (true, false) => "+",
        (false, false) => "",
    };
    format!("{sign}{grouped},{:02}", abs % 100)
}

fn day(date: NaiveDate) -> String {
    date.format("%d.%m.%Y").to_string()
}

/// `left` with `right` ending at the last column.
fn row(left: &str, right: &str) -> String {
    let pad = WIDTH.saturating_sub(left.chars().count() + right.chars().count());
    format!("{left}{}{right}", " ".repeat(pad.max(1)))
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

    fn header(&self, page: usize, pages: usize) -> Vec<String> {
        vec![
            row(&self.bank, &format!("Kontoauszug {}", self.number_label())),
            row("", &format!("Seite {page} von {pages}")),
            format!("Kontoinhaber: {}", self.holder),
            format!("IBAN: {}   BIC: {}", self.iban, self.bic),
            format!("Zeitraum: {} - {}", day(self.from), day(self.to)),
            String::new(),
            row(
                &format!("{:<width$}Valuta", "Buchungstext", width = VALUE_COLUMN),
                "Betrag EUR",
            ),
            "-".repeat(WIDTH),
        ]
    }

    fn entry(line: &StatementLine) -> Vec<String> {
        let text: String = line.text.chars().take(TEXT_LIMIT).collect();
        let left = format!(
            "{text:<width$}{}",
            line.value_date.format("%d.%m."),
            width = VALUE_COLUMN
        );
        let mut out = vec![row(&left, &amount(line.cents, true))];
        out.extend(line.extra.iter().map(|e| format!("    {e}")));
        out
    }

    fn opening(&self) -> Vec<String> {
        let day_before = self.from.pred_opt().unwrap_or(self.from);
        vec![row(
            &format!("Alter Saldo per {}", day(day_before)),
            &amount(self.old_balance, false),
        )]
    }

    fn closing(&self) -> Vec<String> {
        vec![
            "-".repeat(WIDTH),
            row("Summe Gutschriften", &amount(self.credits(), true)),
            row("Summe Lastschriften", &amount(self.debits(), true)),
            row(
                &format!("Neuer Saldo per {}", day(self.to)),
                &amount(self.new_balance(), false),
            ),
        ]
    }

    /// The printed text, page by page; an entry never breaks across pages.
    pub fn text_pages(&self) -> Vec<Vec<String>> {
        let room = LINES_PER_PAGE - self.header(1, 1).len();
        let mut chunks: Vec<Vec<String>> = vec![Vec::new()];
        let mut blocks = vec![self.opening()];
        blocks.extend(self.lines.iter().map(Self::entry));
        blocks.push(self.closing());
        for block in blocks {
            if chunks.last().is_some_and(|c| c.len() + block.len() > room) {
                chunks.push(Vec::new());
            }
            if let Some(chunk) = chunks.last_mut() {
                chunk.extend(block);
            }
        }
        let pages = chunks.len();
        chunks
            .into_iter()
            .enumerate()
            .map(|(i, chunk)| {
                let mut page = self.header(i + 1, pages);
                page.extend(chunk);
                page
            })
            .collect()
    }

    pub fn pdf(&self) -> Vec<u8> {
        let pages: Vec<Page> = self
            .text_pages()
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

/// A month of the made-up Private account: salary, rent, card payments, a subscription and a USD charge with
/// its FX line.
pub fn demo_month(number: u32, year: i32, month: u32, old_balance: i64) -> Statement {
    let d = |day| date(year, month, day);
    let card = |day: u32| format!("{day:02}.{month:02}.");
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
                "Gehalt",
                &[
                    "NORDWIND SOFTWARE GMBH",
                    "AT12 1999 9000 0000 0099",
                    "REF: LOHN",
                ],
                d(1),
                285_000,
            ),
            line(
                "Dauerauftrag",
                &["HAUSVERWALTUNG MUSTER", "AT85 1999 9000 0000 0042", "MIETE"],
                d(3),
                -98_000,
            ),
            line(
                &format!(
                    "POS 58,30 AT K1 {} 17:40 TANKSTELLE RHEIN LUSTENAU 040",
                    card(4)
                ),
                &[],
                d(5),
                -5_830,
            ),
            line(
                &format!("E-COMM 12,99 AT K1 {} STREAMFLIX WIEN 040", card(6)),
                &[],
                d(7),
                -1_299,
            ),
            line(
                &format!("E-COMM 24,00 USD K1 {} LUMEN AI SAN FRANCISCO 840", card(8)),
                &[&format!("SPESEN: 0,49 KURS: 1,0912 VOM: {}", day(d(6)))],
                d(10),
                -2_248,
            ),
            line(
                "Lastschrift",
                &[
                    "BUCHHANDLUNG SEITENWEISE",
                    "AT61 1999 9000 0000 0077",
                    "RE 2026-0412",
                ],
                d(14),
                -4_690,
            ),
            line(
                &format!("POS 6,21 AT K1 {} 09:12 CAFE LINDE DORNBIRN 040", card(20)),
                &[],
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
    fn a_short_statement_is_one_page_with_header_opening_entries_and_closing() {
        let pages = statement().text_pages();
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        assert_eq!(
            page[0],
            format!("{:<62}Kontoauszug Nr. 010/2026", "Example Sparkasse")
        );
        assert_eq!(page[1], format!("{:>86}", "Seite 1 von 1"));
        assert_eq!(page[2], "Kontoinhaber: Alex Beispiel");
        assert_eq!(page[3], "IBAN: AT24 1999 9000 0000 0001   BIC: EXSPATWWXXX");
        assert_eq!(page[4], "Zeitraum: 01.10.2026 - 28.10.2026");
        assert_eq!(page[5], "");
        assert_eq!(
            page[6],
            format!("{:<60}Valuta{:>20}", "Buchungstext", "Betrag EUR")
        );
        assert_eq!(page[7], "-".repeat(WIDTH));
        assert_eq!(
            page[8],
            format!("{:<78}1.412,30", "Alter Saldo per 30.09.2026")
        );
    }

    #[test]
    fn an_entry_ends_its_amount_at_the_width_and_starts_its_value_date_at_the_column() {
        let page = &statement().text_pages()[0];
        assert_eq!(
            page[9],
            format!("{:<60}01.10.{:>20}", "Gehalt", "+2.850,00")
        );
        assert_eq!(page[9].chars().count(), WIDTH);
        assert_eq!(&page[9][VALUE_COLUMN..VALUE_COLUMN + 6], "01.10.");
    }

    #[test]
    fn extra_lines_are_indented_by_four_spaces() {
        let page = &statement().text_pages()[0];
        assert_eq!(
            page[10..13],
            [
                "    NORDWIND SOFTWARE GMBH",
                "    AT12 1999 9000 0000 0099",
                "    REF: LOHN"
            ]
        );
    }

    #[test]
    fn the_closing_block_shows_sums_and_the_new_balance() {
        let page = statement().text_pages().remove(0);
        assert_eq!(
            page[page.len() - 4..],
            [
                "-".repeat(WIDTH),
                format!("{:<77}+2.850,00", "Summe Gutschriften"),
                format!("{:<77}-1.126,88", "Summe Lastschriften"),
                format!("{:<78}3.135,42", "Neuer Saldo per 28.10.2026"),
            ]
        );
    }

    #[test]
    fn a_long_statement_spans_pages_with_numbered_headers() {
        let pages = long_statement().text_pages();
        assert!(pages.len() >= 2);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(
                page[1],
                format!("{:>86}", format!("Seite {} von {}", i + 1, pages.len()))
            );
        }
    }

    #[test]
    fn no_entry_is_split_across_pages() {
        let pages = long_statement().text_pages();
        let header = long_statement().header(1, 1).len();
        let mut entries = 0;
        for page in &pages {
            let body = &page[header..];
            assert!(
                !body[0].starts_with("    "),
                "page starts with an extra line"
            );
            let rates = body.iter().filter(|l| l.starts_with("Rate ")).count();
            let a = body.iter().filter(|l| *l == "    A").count();
            let b = body.iter().filter(|l| *l == "    B").count();
            assert_eq!((a, b), (rates, rates));
            entries += rates;
        }
        assert_eq!(entries, 30);
    }
}
