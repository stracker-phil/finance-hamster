use super::Pdf;
use super::layout::{self, Glyph};
use super::write::{self, Font, Page, Run};
use crate::error::AppError;
use crate::test_support::statements::{self, demo_month};
use crate::test_support::temp_config_dir;

fn run(text: &str) -> Run {
    Run {
        x: 50.0,
        y: 700.0,
        size: 12.0,
        font: Font::Courier,
        text: text.into(),
    }
}

fn page_of(text: &str) -> Page {
    vec![run(text)]
}

fn glyph(c: char, x: f64, y: f64) -> Glyph {
    Glyph {
        c,
        x,
        y,
        width: 5.0,
        height: 10.0,
    }
}

fn written(pages: &[Page]) -> String {
    String::from_utf8_lossy(&write::write(pages)).into_owned()
}

fn lines(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| (*s).to_string()).collect()
}

fn demo_pdf() -> Pdf {
    Pdf::from_bytes(&demo_month(10, 2026, 10, 141_230).pdf()).expect("demo statement opens")
}

mod writer {
    use super::*;

    #[test]
    fn a_document_starts_with_the_pdf_header_and_ends_with_eof() {
        let bytes = write::write(&[page_of("Hallo")]);
        assert!(bytes.starts_with(b"%PDF-1.4\n"));
        assert!(bytes.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn every_xref_offset_points_at_its_object() {
        let text = written(&[page_of("Eins"), page_of("Zwei")]);
        let table = &text[text.find("\nxref\n").expect("xref table") + 1..];
        let mut rows = table.lines().skip(1);
        let count: usize = rows
            .next()
            .and_then(|l| l.strip_prefix("0 "))
            .and_then(|n| n.parse().ok())
            .expect("xref count");
        assert_eq!(rows.next(), Some("0000000000 65535 f "));
        for number in 1..count {
            let row = rows.next().expect("xref row");
            let offset: usize = row[..10].parse().expect("offset digits");
            assert!(
                text[offset..].starts_with(&format!("{number} 0 obj\n")),
                "object {number}"
            );
        }
    }

    #[test]
    fn the_page_tree_counts_its_pages() {
        for pages in [0usize, 1, 3] {
            let input: Vec<Page> = (0..pages).map(|_| page_of("x")).collect();
            assert!(
                written(&input).contains(&format!("/Count {pages} >>")),
                "{pages} pages"
            );
        }
    }

    #[test]
    fn parentheses_and_backslashes_are_escaped_in_literals() {
        assert!(written(&[page_of("a(b)c\\d")]).contains("(a\\(b\\)c\\\\d) Tj"));
    }

    #[test]
    fn non_ascii_text_is_written_as_winansi_octal() {
        let text = written(&[page_of("é€x☃")]);
        assert!(text.contains("(\\351\\200x?) Tj"));
    }

    #[test]
    fn written_pages_open_in_pdfkit_with_their_page_count() {
        let bytes = write::write(&[page_of("Eins"), page_of("Zwei")]);
        assert_eq!(Pdf::from_bytes(&bytes).expect("opens").page_count(), 2);
    }

    #[test]
    fn written_text_reads_back_with_its_special_characters() {
        let bytes = write::write(&[page_of("Größe 12,50 €")]);
        let pdf = Pdf::from_bytes(&bytes).expect("opens");
        assert_eq!(
            pdf.text_pages().expect("text"),
            vec![lines(&["Größe 12,50 €"])]
        );
    }
}

mod layout_lines {
    use super::*;

    #[test]
    fn no_glyphs_make_no_lines() {
        assert_eq!(layout::lines(&[]), Vec::<String>::new());
    }

    #[test]
    fn whitespace_and_zero_width_glyphs_are_ignored() {
        let zero_width = Glyph {
            width: 0.0,
            ..glyph('Z', 10.0, 100.0)
        };
        let glyphs = [glyph(' ', 0.0, 100.0), zero_width, glyph('\n', 5.0, 100.0)];
        assert_eq!(layout::lines(&glyphs), Vec::<String>::new());
    }

    #[test]
    fn gaps_between_glyphs_become_spaces() {
        let glyphs = [glyph('A', 0.0, 100.0), glyph('B', 15.0, 100.0)];
        assert_eq!(layout::lines(&glyphs), lines(&["A  B"]));
    }

    #[test]
    fn a_glyph_sits_in_the_column_its_position_rounds_to() {
        let glyphs = [glyph('A', 0.0, 100.0), glyph('B', 11.0, 100.0)];
        assert_eq!(layout::lines(&glyphs), lines(&["A B"]));
    }

    #[test]
    fn glyphs_are_ordered_left_to_right_and_lines_top_to_bottom() {
        let glyphs = [
            glyph('d', 5.0, 50.0),
            glyph('b', 5.0, 100.0),
            glyph('c', 0.0, 50.0),
            glyph('a', 0.0, 100.0),
        ];
        assert_eq!(layout::lines(&glyphs), lines(&["ab", "cd"]));
    }

    #[test]
    fn baselines_within_a_quarter_of_the_glyph_height_share_a_line() {
        let near = [glyph('a', 0.0, 100.0), glyph('b', 5.0, 97.5)];
        assert_eq!(layout::lines(&near), lines(&["ab"]));
        let far = [glyph('a', 0.0, 100.0), glyph('b', 5.0, 97.0)];
        assert_eq!(layout::lines(&far), lines(&["a", " b"]));
    }

    #[test]
    fn tall_selection_boxes_do_not_interleave_lines_printed_close_together() {
        let sized = |c: char, x: f64, y: f64, width: f64| Glyph {
            c,
            x,
            y,
            width,
            height: 19.3,
        };
        let mut glyphs = Vec::new();
        for (i, c) in "Kontoauszug".chars().enumerate() {
            glyphs.push(sized(c, 319.7 + 6.0 * i as f64, 718.7, 6.0));
        }
        for (i, c) in "Account Statement".chars().enumerate() {
            glyphs.push(sized(c, 319.7 + 4.8 * i as f64, 709.7, 4.8));
        }
        let trimmed = layout::trim_margin(layout::lines(&glyphs));
        assert_eq!(trimmed, lines(&["Kontoauszug", "Account Statement"]));
    }

    #[test]
    fn wide_adjacent_glyphs_stay_one_word_on_a_page_of_narrow_text() {
        let sized = |c: char, x: f64, y: f64, width: f64| Glyph {
            c,
            x,
            y,
            width,
            height: 10.0,
        };
        let mut glyphs = Vec::new();
        for (i, c) in "768,10".chars().enumerate() {
            glyphs.push(sized(c, 60.0 + 6.0 * i as f64, 600.0, 6.0));
        }
        for (i, c) in "abcdefghij".chars().enumerate() {
            glyphs.push(sized(c, 4.0 * i as f64, 500.0, 4.0));
        }
        let expected = vec![
            format!("{}768,10", " ".repeat(15)),
            "abcdefghij".to_string(),
        ];
        assert_eq!(layout::lines(&glyphs), expected);
    }

    #[test]
    fn words_closer_than_one_column_still_get_a_space() {
        let glyphs = [glyph('a', 0.0, 100.0), glyph('b', 6.9, 100.0)];
        assert_eq!(layout::lines(&glyphs), lines(&["a b"]));
    }

    #[test]
    fn fixed_width_text_keeps_its_column_alignment() {
        let glyphs = [
            glyph('a', 0.0, 100.0),
            glyph('x', 300.0, 100.0),
            glyph('y', 305.0, 100.0),
        ];
        let expected = vec![format!("a{}xy", " ".repeat(59))];
        assert_eq!(layout::lines(&glyphs), expected);
    }

    #[test]
    fn indentation_from_the_page_edge_is_kept() {
        let glyphs = [glyph('a', 10.0, 100.0)];
        assert_eq!(layout::lines(&glyphs), lines(&["  a"]));
    }
}

mod layout_margin {
    use super::*;

    #[test]
    fn the_shared_left_margin_is_removed() {
        let trimmed = layout::trim_margin(lines(&["   ab", "     c", "   d"]));
        assert_eq!(trimmed, lines(&["ab", "  c", "d"]));
    }

    #[test]
    fn blank_lines_do_not_count_when_measuring_the_margin() {
        let trimmed = layout::trim_margin(lines(&["  ab", "", "    ", "  c"]));
        assert_eq!(trimmed, lines(&["ab", "", "", "c"]));
    }

    #[test]
    fn text_without_a_margin_is_unchanged() {
        let input = lines(&["ab", "  c"]);
        assert_eq!(layout::trim_margin(input.clone()), input);
    }

    #[test]
    fn no_lines_stay_no_lines() {
        assert_eq!(layout::trim_margin(Vec::new()), Vec::<String>::new());
    }
}

mod pdf {
    use super::*;

    #[test]
    fn a_file_that_is_not_a_pdf_is_invalid_input() {
        let dir = temp_config_dir();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "just some text").expect("write");
        assert!(matches!(
            Pdf::open(&path).err(),
            Some(AppError::InvalidInput(_))
        ));
    }

    #[test]
    fn bytes_that_are_not_a_pdf_are_invalid_input() {
        assert!(matches!(
            Pdf::from_bytes(b"nope").err(),
            Some(AppError::InvalidInput(_))
        ));
    }

    #[test]
    fn a_pdf_file_opens_by_path() {
        let dir = temp_config_dir();
        let path = dir.path().join("statement.pdf");
        std::fs::write(&path, demo_month(10, 2026, 10, 141_230).pdf()).expect("write");
        let pdf = Pdf::open(&path).expect("opens");
        assert_eq!(pdf.page_count(), 1);
    }

    #[test]
    fn a_page_renders_as_png() {
        let png = demo_pdf().page_png(0, 300.0).expect("png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn rendering_past_the_last_page_is_not_found() {
        assert!(matches!(
            demo_pdf().page_png(1, 300.0),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn a_statement_reads_back_line_for_line_with_its_columns() {
        let statement = demo_month(10, 2026, 10, 141_230);
        let expected: Vec<Vec<String>> = statement
            .text_pages()
            .into_iter()
            .map(|page| {
                page.into_iter()
                    .map(|l| l.trim_end().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .collect();
        let actual: Vec<Vec<String>> = Pdf::from_bytes(&statement.pdf())
            .expect("opens")
            .text_pages()
            .expect("text")
            .into_iter()
            .map(|page| page.into_iter().map(|l| l.trim_end().to_string()).collect())
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn statement_columns_survive_the_roundtrip() {
        let pages = demo_pdf().text_pages().expect("text");
        let entry = pages[0]
            .iter()
            .find(|l| l.starts_with("Gehalt"))
            .expect("salary line");
        assert_eq!(
            &entry[statements::VALUE_COLUMN..statements::VALUE_COLUMN + 6],
            "01.10."
        );
        assert_eq!(entry.chars().count(), statements::WIDTH);
    }
}
