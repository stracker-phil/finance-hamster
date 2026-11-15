//! Every document and statement keeps the hash of its file; `verify` compares them at launch and on demand,
//! and the last result stays in memory for the banner and Settings.

pub mod commands;
pub mod status;

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;

use crate::db::Db;
use crate::documents::files::{find_by_hash, same_file};
use crate::documents::store as documents;
use crate::documents::title_of;
use crate::error::AppResult;
use crate::lock::MutexExt;
use crate::statements::store as statements;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum FileTrouble {
    Missing,
    Changed,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct FileProblem {
    pub trouble: FileTrouble,
    /// Relative to the data folder.
    pub file: String,
    /// `Document · Lumen AI invoice`, `Statement · Private Nr. 003/2026`.
    pub what: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct FileCheck {
    pub checked: u32,
    pub problems: Vec<FileProblem>,
    /// When it ran, in milliseconds.
    pub at: i64,
}

#[derive(Default)]
pub struct LastCheck(Mutex<Option<FileCheck>>);

impl LastCheck {
    pub fn get(&self) -> Option<FileCheck> {
        self.0.lock_unpoisoned().clone()
    }

    pub fn set(&self, check: FileCheck) {
        *self.0.lock_unpoisoned() = Some(check);
    }
}

/// None when the file is where it should be with its hash.
fn file_trouble(config_dir: &Path, file: &str, hash: &str) -> Option<FileTrouble> {
    let path = config_dir.join(file);
    if same_file(&path, hash) {
        None
    } else if path.exists() {
        Some(FileTrouble::Changed)
    } else {
        Some(FileTrouble::Missing)
    }
}

pub fn verify(db: &Db, config_dir: &Path, now: i64) -> AppResult<FileCheck> {
    let mut checked = 0;
    let mut problems = Vec::new();
    let vendors = db.read(documents::vendors)?;
    for doc in db.read(documents::documents)? {
        checked += 1;
        // A draft's file moves with its facts; found by its hash under `docs/`, it is in place.
        let trouble = file_trouble(config_dir, &doc.file, &doc.file_hash)
            .filter(|_| find_by_hash(config_dir, &doc.file_hash).is_none());
        if let Some(trouble) = trouble {
            problems.push(FileProblem {
                trouble,
                file: doc.file.clone(),
                what: format!("Document · {}", title_of(&doc, &vendors)),
            });
        }
    }
    let accounts = db.read(crate::accounts::rows)?;
    for account in &accounts {
        for statement in db.read(|c| statements::statements_of(c, &account.id))? {
            checked += 1;
            if let Some(trouble) = file_trouble(config_dir, &statement.file, &statement.file_hash) {
                problems.push(FileProblem {
                    trouble,
                    file: statement.file.clone(),
                    what: format!(
                        "Statement · {} Nr. {:03}/{}",
                        account.name, statement.number, statement.year
                    ),
                });
            }
        }
    }
    Ok(FileCheck {
        checked,
        problems,
        at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Change, Op, Source};
    use crate::documents::store::{DOCUMENTS, DocState, DocumentRow};
    use crate::statements::file_hash;
    use crate::statements::import;
    use crate::statements::import_tests::open_with_accounts;
    use crate::statements::store::StatementRow;
    use crate::test_support::statements::demo_month;

    const NOW: i64 = 1_700_000_000_000;
    const INVOICE: &str = "docs/2026/2026-11-10-invoice-a.pdf";

    fn write_file(dir: &Path, file: &str, bytes: &[u8]) {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn add_document(db: &Db, dir: &Path, id: &str, file: &str, bytes: &[u8]) {
        write_file(dir, file, bytes);
        let row = DocumentRow {
            id: id.into(),
            state: DocState::Draft,
            file: file.into(),
            original_name: format!("{id}.pdf"),
            file_hash: file_hash(bytes),
            vendor_id: None,
            pay_date: Some("2026-11-10".into()),
            paid_cents: None,
            item_key: None,
            work_share: 100,
            note: String::new(),
            invoice_cents: None,
            invoice_currency: None,
            invoice_number: None,
            invoice_date: None,
            created_at: 1,
            finished_at: None,
            excluded_at: None,
            exclude_reason: None,
            useful_life: None,
        };
        db.write(
            Op::new(
                "Add document",
                vec![Change::insert(DOCUMENTS, &row).unwrap()],
            ),
            Source::Seed,
        )
        .unwrap();
    }

    fn import_statement(db: &Db, dir: &Path, number: u32, month: u32) -> StatementRow {
        let pending =
            import::prepare(db, dir, demo_month(number, 2026, month, 100_000).pdf()).unwrap();
        import::commit(db, dir, pending, NOW, Source::Import)
            .unwrap()
            .1
    }

    fn problem(trouble: FileTrouble, file: &str, what: &str) -> FileProblem {
        FileProblem {
            trouble,
            file: file.into(),
            what: what.into(),
        }
    }

    #[test]
    fn an_empty_library_checks_nothing_and_stamps_the_time() {
        let (dir, db) = open_with_accounts();

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap(),
            FileCheck {
                checked: 0,
                problems: vec![],
                at: NOW
            }
        );
    }

    #[test]
    fn documents_and_statements_in_place_with_their_hash_are_fine() {
        let (dir, db) = open_with_accounts();
        add_document(&db, dir.path(), "invoice-a", INVOICE, b"%PDF invoice a");
        import_statement(&db, dir.path(), 1, 9);

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap(),
            FileCheck {
                checked: 2,
                problems: vec![],
                at: NOW
            }
        );
    }

    #[test]
    fn a_document_moved_elsewhere_under_docs_is_found_by_its_hash() {
        let (dir, db) = open_with_accounts();
        add_document(&db, dir.path(), "invoice-a", INVOICE, b"%PDF invoice a");
        let moved = dir.path().join("docs/inbox/renamed.pdf");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::rename(dir.path().join(INVOICE), moved).unwrap();

        let check = verify(&db, dir.path(), NOW).unwrap();

        assert_eq!((check.checked, check.problems), (1, vec![]));
    }

    #[test]
    fn a_document_moved_out_of_docs_is_missing() {
        let (dir, db) = open_with_accounts();
        add_document(&db, dir.path(), "invoice-a", INVOICE, b"%PDF invoice a");
        write_file(dir.path(), "elsewhere/invoice-a.pdf", b"%PDF invoice a");
        std::fs::remove_file(dir.path().join(INVOICE)).unwrap();

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap().problems,
            vec![problem(
                FileTrouble::Missing,
                INVOICE,
                "Document · invoice-a"
            )]
        );
    }

    #[test]
    fn a_deleted_document_file_is_missing() {
        let (dir, db) = open_with_accounts();
        add_document(&db, dir.path(), "invoice-a", INVOICE, b"%PDF invoice a");
        std::fs::remove_file(dir.path().join(INVOICE)).unwrap();

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap().problems,
            vec![problem(
                FileTrouble::Missing,
                INVOICE,
                "Document · invoice-a"
            )]
        );
    }

    #[test]
    fn an_overwritten_document_file_is_changed() {
        let (dir, db) = open_with_accounts();
        add_document(&db, dir.path(), "invoice-a", INVOICE, b"%PDF invoice a");
        write_file(dir.path(), INVOICE, b"%PDF something else");

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap().problems,
            vec![problem(
                FileTrouble::Changed,
                INVOICE,
                "Document · invoice-a"
            )]
        );
    }

    #[test]
    fn a_statement_is_checked_at_its_stored_path_and_named_by_account_and_number() {
        let (dir, db) = open_with_accounts();
        let statement = import_statement(&db, dir.path(), 3, 10);
        write_file(dir.path(), &statement.file, b"%PDF forged");

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap().problems,
            vec![problem(
                FileTrouble::Changed,
                &statement.file,
                "Statement · Private Nr. 003/2026"
            )]
        );
    }

    #[test]
    fn only_the_overwritten_statement_is_reported_and_a_deleted_one_is_missing() {
        let (dir, db) = open_with_accounts();
        let september = import_statement(&db, dir.path(), 1, 9);
        let october = import_statement(&db, dir.path(), 2, 10);
        write_file(dir.path(), &september.file, b"%PDF other bytes");

        let check = verify(&db, dir.path(), NOW).unwrap();
        assert_eq!(check.checked, 2);
        assert_eq!(
            check.problems,
            vec![problem(
                FileTrouble::Changed,
                &september.file,
                "Statement · Private Nr. 001/2026"
            )]
        );

        std::fs::remove_file(dir.path().join(&october.file)).unwrap();
        let mut problems = verify(&db, dir.path(), NOW).unwrap().problems;
        problems.sort_by(|a, b| a.file.cmp(&b.file));
        assert_eq!(
            problems,
            vec![
                problem(
                    FileTrouble::Changed,
                    &september.file,
                    "Statement · Private Nr. 001/2026"
                ),
                problem(
                    FileTrouble::Missing,
                    &october.file,
                    "Statement · Private Nr. 002/2026"
                ),
            ]
        );
    }

    #[test]
    fn a_statement_file_moved_elsewhere_counts_as_missing() {
        let (dir, db) = open_with_accounts();
        let statement = import_statement(&db, dir.path(), 3, 10);
        let moved = dir.path().join("docs/moved-statement.pdf");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::rename(dir.path().join(&statement.file), moved).unwrap();

        assert_eq!(
            verify(&db, dir.path(), NOW).unwrap().problems,
            vec![problem(
                FileTrouble::Missing,
                &statement.file,
                "Statement · Private Nr. 003/2026"
            )]
        );
    }

    #[test]
    fn the_last_check_is_empty_until_one_is_set() {
        let last = LastCheck::default();
        assert_eq!(last.get(), None);

        let check = FileCheck {
            checked: 4,
            problems: vec![problem(
                FileTrouble::Missing,
                INVOICE,
                "Document · invoice-a",
            )],
            at: NOW,
        };
        last.set(check.clone());

        assert_eq!(last.get(), Some(check));
    }

    #[test]
    fn a_check_serializes_with_camel_case_fields_and_snake_case_troubles() {
        let check = FileCheck {
            checked: 1,
            problems: vec![problem(
                FileTrouble::Changed,
                INVOICE,
                "Document · invoice-a",
            )],
            at: NOW,
        };

        assert_eq!(
            serde_json::to_value(&check).unwrap(),
            serde_json::json!({
                "checked": 1,
                "problems": [{ "trouble": "changed", "file": INVOICE, "what": "Document · invoice-a" }],
                "at": NOW
            })
        );
    }
}
