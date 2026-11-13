//! Bank statements: reading the bank's text PDF (`parse`), checking it (`check`) and its file hash.

pub mod check;
pub mod commands;
pub mod import;
pub mod lines;
pub mod parse;
pub mod store;
pub mod view;

#[cfg(test)]
mod check_tests;
#[cfg(test)]
pub(crate) mod import_tests;
#[cfg(test)]
mod lines_tests;
#[cfg(test)]
mod view_tests;

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

/// Whether the PDF reads as a statement of a layout Hamster knows.
pub fn is_statement(bytes: &[u8]) -> bool {
    crate::platform::pdf::Pdf::from_bytes(bytes)
        .and_then(|pdf| pdf.text_pages())
        .is_ok_and(|pages| parse::parse(&pages).is_ok())
}

/// The file's SHA-256 in hex; the same hash on import is a duplicate.
pub fn file_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_hash_is_the_lowercase_hex_sha256() {
        for (bytes, expected) in [
            (
                &b"abc"[..],
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                &b""[..],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
        ] {
            assert_eq!(file_hash(bytes), expected, "{bytes:?}");
        }
    }

    #[test]
    fn a_demo_statement_reads_as_a_statement() {
        let bytes = crate::test_support::statements::demo_month(10, 2026, 10, 141_230).pdf();
        assert!(is_statement(&bytes));
    }

    #[test]
    fn an_invoice_pdf_and_bytes_that_are_no_pdf_do_not_read_as_a_statement() {
        use crate::platform::pdf::write::{self, Font, Run};

        let invoice = write::write(&[vec![Run {
            x: 50.0,
            y: 780.0,
            size: 10.0,
            font: Font::Courier,
            text: "Pixelforge Ltd. invoice, total USD 129.00".into(),
        }]]);

        assert!(!is_statement(&invoice));
        assert!(!is_statement(b"just some text"));
        assert!(!is_statement(b""));
    }

    #[test]
    fn different_bytes_give_different_hashes() {
        assert_ne!(file_hash(b"abc"), file_hash(b"abd"));
        assert_ne!(file_hash(b"abc"), file_hash(b"abc "));
    }
}
