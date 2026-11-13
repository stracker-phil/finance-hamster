//! Bank statements: reading the bank's text PDF (`parse`), checking it (`check`) and its file hash.

pub mod check;
pub mod parse;

#[cfg(test)]
mod check_tests;

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

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
    fn different_bytes_give_different_hashes() {
        assert_ne!(file_hash(b"abc"), file_hash(b"abd"));
        assert_ne!(file_hash(b"abc"), file_hash(b"abc "));
    }
}
