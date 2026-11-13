//! PDF previews by id (ADR 0018, ADR 0037): the page names a document or statement, Rust finds its file. The
//! native view shows it over the page; the `pdf-page` URI scheme serves its pages as PNG.

pub mod commands;
pub mod scheme;

use std::path::{Path, PathBuf};

use serde::Deserialize;
use ts_rs::TS;

use crate::db::Db;
use crate::error::{AppError, AppResult};

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum SourceKind {
    Document,
    Statement,
}

impl SourceKind {
    pub fn parse(text: &str) -> Option<SourceKind> {
        match text {
            "document" => Some(SourceKind::Document),
            "statement" => Some(SourceKind::Statement),
            _ => None,
        }
    }
}

/// A PDF the app keeps, named by what it belongs to; the page never sends paths.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct PdfSource {
    pub kind: SourceKind,
    pub id: String,
}

/// The id of the debug build's sample document: a generated demo statement.
#[cfg(debug_assertions)]
pub const SAMPLE_ID: &str = "sample";

/// The file behind a source; only ids the app knows resolve.
pub fn resolve(db: &Db, config_dir: &Path, source: &PdfSource) -> AppResult<PathBuf> {
    #[cfg(debug_assertions)]
    if source.kind == SourceKind::Document && source.id == SAMPLE_ID {
        return sample();
    }
    let file = match source.kind {
        SourceKind::Document => db
            .read(|c| crate::documents::store::document_by_id(c, &source.id))?
            .map(|d| d.file),
        SourceKind::Statement => db
            .read(|c| crate::statements::store::statement_by_id(c, &source.id))?
            .map(|s| s.file),
    };
    file.map(|f| config_dir.join(f))
        .ok_or_else(|| AppError::NotFound(format!("No PDF for {:?} {}", source.kind, source.id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::pdf::Pdf;
    use crate::test_support::temp_config_dir;

    fn source(kind: SourceKind, id: &str) -> PdfSource {
        PdfSource {
            kind,
            id: id.into(),
        }
    }

    #[test]
    fn source_kinds_parse_from_their_path_names() {
        for (text, expected) in [
            ("document", Some(SourceKind::Document)),
            ("statement", Some(SourceKind::Statement)),
            ("Document", None),
            ("", None),
        ] {
            assert_eq!(SourceKind::parse(text), expected, "{text}");
        }
    }

    #[test]
    fn unknown_ids_are_not_found() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).expect("db");
        for source in [
            source(SourceKind::Document, "missing"),
            source(SourceKind::Statement, SAMPLE_ID),
        ] {
            assert!(
                matches!(
                    resolve(&db, dir.path(), &source),
                    Err(AppError::NotFound(_))
                ),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_document_resolves_to_its_file_in_the_config_dir_by_id() {
        use crate::db::Source;
        use crate::documents::add::{self, AddInput};

        let dir = temp_config_dir();
        let db = Db::open(dir.path()).expect("db");
        let pending = add::prepare(&db, b"%PDF demo".to_vec(), "invoice.pdf".into()).unwrap();
        let input = AddInput {
            token: "token-1".into(),
            vendor: "Pixelforge Ltd.".into(),
            item_key: None,
            line_id: None,
        };
        add::commit(&db, dir.path(), pending, &input, 1, Source::Ui).unwrap();
        let id = db.read(crate::documents::store::documents).unwrap()[0]
            .id
            .clone();

        assert_eq!(
            resolve(&db, dir.path(), &source(SourceKind::Document, &id)).unwrap(),
            dir.path().join("docs/inbox/invoice.pdf")
        );
    }

    #[test]
    fn a_statement_resolves_to_its_file_in_the_config_dir_by_id() {
        use crate::db::Source;
        use crate::statements::import;
        use crate::statements::import_tests::{october, open_with_accounts};

        let (dir, db) = open_with_accounts();
        let pending = import::prepare(&db, dir.path(), october().pdf()).unwrap();
        let (_, row) = import::commit(&db, dir.path(), pending, 1, Source::Import).unwrap();

        assert_eq!(
            resolve(&db, dir.path(), &source(SourceKind::Statement, &row.id)).unwrap(),
            dir.path().join(&row.file)
        );
    }

    #[test]
    fn a_document_id_is_not_found_as_a_statement() {
        use crate::db::Source;
        use crate::documents::add::{self, AddInput};

        let dir = temp_config_dir();
        let db = Db::open(dir.path()).expect("db");
        let pending = add::prepare(&db, b"%PDF demo".to_vec(), "invoice.pdf".into()).unwrap();
        let input = AddInput {
            token: "token-1".into(),
            vendor: "Pixelforge Ltd.".into(),
            item_key: None,
            line_id: None,
        };
        add::commit(&db, dir.path(), pending, &input, 1, Source::Ui).unwrap();
        let id = db.read(crate::documents::store::documents).unwrap()[0]
            .id
            .clone();

        assert!(matches!(
            resolve(&db, dir.path(), &source(SourceKind::Statement, &id)),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn the_sample_document_resolves_to_a_readable_pdf() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).expect("db");
        let path = resolve(&db, dir.path(), &source(SourceKind::Document, SAMPLE_ID))
            .expect("sample resolves");
        assert!(path.exists());
        assert_eq!(Pdf::open(&path).expect("opens").page_count(), 1);
    }
}

#[cfg(debug_assertions)]
fn sample() -> AppResult<PathBuf> {
    let path = std::env::temp_dir().join("hamster-debug-sample.pdf");
    if !path.exists() {
        let statement = crate::test_support::statements::demo_month(10, 2026, 10, 141_230);
        std::fs::write(&path, statement.pdf())?;
    }
    Ok(path)
}
