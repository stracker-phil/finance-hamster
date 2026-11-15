//! The year summary as a PDF: the copy value per Kennzahl with the rows behind it, for the snapshot taken on
//! closing and for Export Summary PDF.

use super::page::YearPage;
use crate::platform::pdf::write::{self, A4_HEIGHT, Font, Page, Run};

const LEFT: f32 = 56.0;
const RIGHT: f32 = 539.0;
const TOP: f32 = A4_HEIGHT - 64.0;
const BOTTOM: f32 = 64.0;

/// Fills pages top to bottom, starting a new page when one is full.
struct Writer {
    pages: Vec<Page>,
    y: f32,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            pages: vec![Vec::new()],
            y: TOP,
        }
    }

    fn line(&mut self, gap: f32) {
        self.y -= gap;
        if self.y < BOTTOM {
            self.pages.push(Vec::new());
            self.y = TOP - gap;
        }
    }

    fn text(&mut self, x: f32, size: f32, font: Font, text: &str) {
        if let Some(page) = self.pages.last_mut() {
            page.push(Run {
                x,
                y: self.y,
                size,
                font,
                text: text.to_string(),
            });
        }
    }

    /// Right-aligned at `RIGHT`, by the fixed advance of Courier (0.6 em).
    fn amount(&mut self, size: f32, text: &str) {
        let width = text.chars().count() as f32 * size * 0.6;
        self.text(RIGHT - width, size, Font::Courier, text);
    }
}

/// `stamp` is the line under the title: when it was closed, or when it was exported.
pub fn summary_pdf(page: &YearPage, stamp: &str) -> Vec<u8> {
    let mut w = Writer::new();
    w.text(
        LEFT,
        18.0,
        Font::HelveticaBold,
        &crate::tr!(
            "Tax year {} · Werbungskosten",
            "Steuerjahr {} · Werbungskosten",
            page.year
        ),
    );
    w.line(18.0);
    w.text(
        LEFT,
        10.0,
        Font::Helvetica,
        &crate::tr!(
            "Ruleset {} · {stamp}",
            "Regelwerk {} · {stamp}",
            page.ruleset
        ),
    );
    w.line(28.0);
    w.text(
        LEFT,
        12.0,
        Font::HelveticaBold,
        &crate::tr!("Total to enter", "Einzutragende Summe"),
    );
    w.amount(12.0, &page.total);
    for kz in page.kennzahlen.iter().filter(|k| !k.rows.is_empty()) {
        w.line(26.0);
        w.text(
            LEFT,
            11.0,
            Font::HelveticaBold,
            &format!("KZ {}  {}", kz.kz, kz.name),
        );
        w.amount(11.0, &kz.copy);
        for row in &kz.rows {
            w.line(15.0);
            w.text(
                LEFT + 16.0,
                9.0,
                Font::Helvetica,
                &format!("{} · {}", row.title, row.detail),
            );
            w.amount(9.0, &row.amount);
        }
        if let Some(note) = &kz.note {
            w.line(14.0);
            w.text(LEFT + 16.0, 8.0, Font::Helvetica, note);
        }
    }
    let answered: Vec<_> = page.questions.iter().filter(|q| q.answered).collect();
    if !answered.is_empty() {
        w.line(28.0);
        w.text(
            LEFT,
            11.0,
            Font::HelveticaBold,
            &crate::tr!("Answers", "Antworten"),
        );
        for q in answered {
            w.line(15.0);
            w.text(LEFT + 16.0, 9.0, Font::Helvetica, &q.label);
            w.amount(
                9.0,
                format!("{} {}", q.value, q.unit.clone().unwrap_or_default()).trim(),
            );
        }
    }
    write::write(&w.pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::pdf::Pdf;
    use crate::taxyears::YearState;
    use crate::year::page::{KennzahlView, QuestionView, RowKind, YearRow};

    fn row(id: &str, detail: &str, amount: &str) -> YearRow {
        YearRow {
            id: id.into(),
            kind: RowKind::Document,
            title: id.into(),
            detail: detail.into(),
            amount: amount.into(),
            draft: false,
            carried_from: None,
            kz: "720".into(),
            attach: false,
        }
    }

    fn kennzahl(kz: &str, name: &str, copy: &str, rows: Vec<YearRow>) -> KennzahlView {
        KennzahlView {
            key: name.to_lowercase(),
            kz: kz.into(),
            name: name.into(),
            label: name.into(),
            examples: None,
            copy: copy.into(),
            total: format!("€{}", copy.replace(',', ".")),
            rows,
            note: None,
            hint: None,
            add: None,
            rate: None,
            reported: false,
            shown: true,
        }
    }

    fn question(label: &str, value: &str, unit: Option<&str>, answered: bool) -> QuestionView {
        QuestionView {
            id: label.to_lowercase(),
            label: label.into(),
            help: None,
            unit: unit.map(String::from),
            value: value.into(),
            answered,
        }
    }

    fn page(kennzahlen: Vec<KennzahlView>, questions: Vec<QuestionView>) -> YearPage {
        YearPage {
            year: 2026,
            ruleset: "2026 v1".into(),
            state: YearState::Open,
            total: "€118.90".into(),
            progress: String::new(),
            progress_percent: 0,
            pauschale_note: None,
            questions,
            attention: Vec::new(),
            kennzahlen,
            months: Vec::new(),
            archived: 0,
        }
    }

    fn demo_page() -> YearPage {
        page(
            vec![
                kennzahl(
                    "720",
                    "Books",
                    "46,90",
                    vec![row("doc-a", "14 Oct", "€46.90")],
                ),
                kennzahl(
                    "721",
                    "Phone",
                    "72,00",
                    vec![row("doc-b", "2 Nov · 60 % of €120.00", "€72.00")],
                ),
                kennzahl("722", "Tips", "0,00", vec![]),
            ],
            vec![
                question("Telearbeitstage", "28", Some("days"), true),
                question("Pauschale", "", None, false),
            ],
        )
    }

    /// The text of every page as lines with all whitespace removed (PDFKit splits proportional text
    /// unpredictably), blank lines dropped.
    fn read_back(page: &YearPage, stamp: &str) -> Vec<Vec<String>> {
        Pdf::from_bytes(&summary_pdf(page, stamp))
            .expect("the summary opens")
            .text_pages()
            .expect("the summary has text")
            .into_iter()
            .map(|lines| {
                lines
                    .iter()
                    .map(|l| l.split_whitespace().collect::<String>())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn the_summary_reads_back_as_title_stamp_total_kennzahlen_rows_and_answers() {
        assert_eq!(
            read_back(&demo_page(), "Closed 13 Nov 2026"),
            vec![vec![
                "Taxyear2026·Werbungskosten".to_string(),
                "Ruleset2026v1·Closed13Nov2026".into(),
                "Totaltoenter€118.90".into(),
                "KZ720Books46,90".into(),
                "doc-a·14Oct€46.90".into(),
                "KZ721Phone72,00".into(),
                "doc-b·2Nov·60%of€120.00€72.00".into(),
                "Answers".into(),
                "Telearbeitstage28days".into(),
            ]]
        );
    }

    #[test]
    fn a_kennzahl_without_rows_is_left_out() {
        let lines = read_back(&demo_page(), "x").concat();

        assert!(!lines.iter().any(|l| l.contains("KZ722")), "{lines:?}");
    }

    #[test]
    fn without_answers_there_is_no_answers_section() {
        let unanswered = page(vec![], vec![question("Pauschale", "", None, false)]);

        let lines = read_back(&unanswered, "x").concat();

        assert_eq!(
            lines,
            [
                "Taxyear2026·Werbungskosten",
                "Ruleset2026v1·x",
                "Totaltoenter€118.90"
            ]
        );
    }

    #[test]
    fn a_kennzahl_note_is_printed_under_its_rows() {
        let mut books = kennzahl(
            "720",
            "Books",
            "46,90",
            vec![row("doc-a", "14 Oct", "€46.90")],
        );
        books.note = Some("Enter the full amount.".into());

        let lines = read_back(&page(vec![books], vec![]), "x").concat();

        assert_eq!(
            lines[3..],
            [
                "KZ720Books46,90",
                "doc-a·14Oct€46.90",
                "Enterthefullamount."
            ][..]
        );
    }

    #[test]
    fn a_long_year_continues_on_further_pages_without_losing_rows() {
        let rows: Vec<YearRow> = (1..=120)
            .map(|i| row(&format!("doc-{i:03}"), "14 Oct", "€1.00"))
            .collect();

        let long_year = page(vec![kennzahl("720", "Books", "120,00", rows)], vec![]);

        let pages = read_back(&long_year, "x");

        assert!(pages.len() >= 3, "{} pages", pages.len());
        let all = pages.concat();
        assert_eq!(all.iter().filter(|l| l.starts_with("doc-")).count(), 120);
        assert_eq!(all.last().map(String::as_str), Some("doc-120·14Oct€1.00"));
    }
}
