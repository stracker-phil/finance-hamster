use serde_json::json;
use tempfile::TempDir;

use super::edit::{self, OtherEdit};
use super::view::{self, OtherGroup, OtherListRow, OtherView};
use super::{OtherRow, by_hash, other, others};
use crate::date::DateOrder;
use crate::db::Db;
use crate::error::AppError;
use crate::i18n::with_lang;
use crate::settings::Language;
use crate::statements::file_hash;
use crate::test_support::temp_config_dir;

const DAY_FIRST: DateOrder = DateOrder::DayFirst;

fn open() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn add_at(db: &Db, dir: &TempDir, name: &str, content: &str, now: i64) -> String {
    edit::add(db, dir.path(), name, content.as_bytes(), now)
        .unwrap()
        .1
}

fn add(db: &Db, dir: &TempDir, name: &str) -> String {
    add_at(db, dir, name, &format!("%PDF {name}"), 1)
}

fn stored(db: &Db, id: &str) -> OtherRow {
    db.read(|c| other(c, id)).unwrap().unwrap()
}

fn title(text: &str) -> OtherEdit {
    OtherEdit::Title { text: text.into() }
}

fn date(value: &str) -> OtherEdit {
    OtherEdit::Date {
        value: value.into(),
    }
}

fn note(text: &str) -> OtherEdit {
    OtherEdit::Note { text: text.into() }
}

fn change(db: &Db, id: &str, change: OtherEdit) {
    edit::edit(db, id, &change).unwrap().unwrap();
}

fn invalid_field(err: AppError) -> String {
    match err {
        AppError::InvalidField { field, .. } => field,
        unexpected => panic!("expected an invalid field, got {unexpected:?}"),
    }
}

fn dated(db: &Db, dir: &TempDir, name: &str, day: &str, now: i64) -> String {
    let id = add_at(db, dir, name, &format!("%PDF {name}"), now);
    change(db, &id, date(day));
    id
}

fn group_titles(groups: &[OtherGroup]) -> Vec<(String, Vec<String>)> {
    groups
        .iter()
        .map(|g| {
            (
                g.title.clone(),
                g.documents.iter().map(|d| d.title.clone()).collect(),
            )
        })
        .collect()
}

fn listed(db: &Db, query: &str) -> Vec<OtherGroup> {
    view::list(db, query, DAY_FIRST).unwrap()
}

mod adding {
    use super::*;

    #[test]
    fn a_pdf_is_kept_under_its_file_name_as_title_without_date_or_note() {
        let (dir, db) = open();

        let (written, id) =
            edit::add(&db, dir.path(), "Home Policy.pdf", b"%PDF policy", 42).unwrap();

        assert_eq!(written.label, "Add Home Policy");
        assert_eq!(written.tables, vec!["others".to_string()]);
        assert_eq!(
            stored(&db, &id),
            OtherRow {
                id: id.clone(),
                title: "Home Policy".into(),
                date: None,
                note: String::new(),
                file: "docs/other/home-policy.pdf".into(),
                file_hash: file_hash(b"%PDF policy"),
                created_at: 42,
            }
        );
    }

    #[test]
    fn the_pdf_is_copied_unchanged_into_the_other_folder() {
        let (dir, db) = open();

        edit::add(&db, dir.path(), "Home Policy.pdf", b"%PDF policy", 1).unwrap();

        assert_eq!(
            std::fs::read(dir.path().join("docs/other/home-policy.pdf")).unwrap(),
            b"%PDF policy"
        );
    }

    #[test]
    fn the_file_name_is_the_slug_of_the_title() {
        let (dir, db) = open();

        for (name, expected) in [
            (
                "Bescheid Übersicht.pdf",
                "docs/other/bescheid-uebersicht.pdf",
            ),
            (
                "  Tax  Notice (2025) .pdf",
                "docs/other/tax-notice-2025.pdf",
            ),
            ("archive.v2.pdf", "docs/other/archive-v2.pdf"),
        ] {
            let id = add(&db, &dir, name);

            assert_eq!(stored(&db, &id).file, expected, "{name}");
        }
    }

    #[test]
    fn a_name_without_letters_or_digits_is_filed_as_document() {
        let (dir, db) = open();

        let id = add(&db, &dir, "---.pdf");

        assert_eq!(stored(&db, &id).title, "---");
        assert_eq!(stored(&db, &id).file, "docs/other/document.pdf");
    }

    #[test]
    fn another_pdf_with_the_same_name_gets_a_numbered_file() {
        let (dir, db) = open();
        let first = add_at(&db, &dir, "Policy.pdf", "%PDF one", 1);
        let second = add_at(&db, &dir, "Policy.pdf", "%PDF two", 2);

        assert_eq!(stored(&db, &first).file, "docs/other/policy.pdf");
        assert_eq!(stored(&db, &second).file, "docs/other/policy-2.pdf");
        assert_eq!(
            std::fs::read(dir.path().join("docs/other/policy.pdf")).unwrap(),
            b"%PDF one"
        );
    }

    #[test]
    fn the_same_pdf_twice_is_refused_naming_the_kept_one() {
        let (dir, db) = open();
        add_at(&db, &dir, "Policy.pdf", "%PDF one", 1);

        let err = edit::add(&db, dir.path(), "Copy of policy.pdf", b"%PDF one", 2).unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "This PDF is already kept as Policy"),
            "{err:?}"
        );
        assert_eq!(db.read(others).unwrap().len(), 1);
    }

    #[test]
    fn the_refusal_names_the_title_as_it_was_renamed_and_reads_in_german() {
        let (dir, db) = open();
        let id = add_at(&db, &dir, "Policy.pdf", "%PDF one", 1);
        change(&db, &id, title("Home cover"));

        let err = with_lang(Language::De, || {
            edit::add(&db, dir.path(), "again.pdf", b"%PDF one", 2)
        })
        .unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "Dieses PDF ist schon als Home cover abgelegt"),
            "{err:?}"
        );
    }

    #[test]
    fn the_label_reads_in_german() {
        let (dir, db) = open();

        let (written, _) = with_lang(Language::De, || {
            edit::add(&db, dir.path(), "Police.pdf", b"%PDF", 1)
        })
        .unwrap();

        assert_eq!(written.label, "Police hinzufügen");
    }

    #[test]
    fn undoing_an_add_removes_the_document_but_keeps_the_file_and_redo_brings_it_back() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        db.undo().unwrap().unwrap();
        assert_eq!(db.read(|c| other(c, &id)).unwrap(), None);
        assert!(dir.path().join("docs/other/policy.pdf").exists());

        db.redo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).title, "Policy");
    }

    #[test]
    fn a_kept_pdf_is_found_by_its_hash() {
        let (dir, db) = open();
        let id = add_at(&db, &dir, "Policy.pdf", "%PDF one", 1);

        let found = db
            .read(|c| by_hash(c, &file_hash(b"%PDF one")))
            .unwrap()
            .unwrap();
        let missing = db.read(|c| by_hash(c, &file_hash(b"%PDF other"))).unwrap();

        assert_eq!(found.id, id);
        assert_eq!(missing, None);
    }
}

mod editing {
    use super::*;

    #[test]
    fn a_title_is_trimmed() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        let written = edit::edit(&db, &id, &title("  Home cover  "))
            .unwrap()
            .unwrap();

        assert_eq!(stored(&db, &id).title, "Home cover");
        assert_eq!(written.label, "Rename Policy to Home cover");
        assert_eq!(written.tables, vec!["others".to_string()]);
    }

    #[test]
    fn a_blank_title_is_refused_and_changes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        for blank in ["", "   "] {
            let err = edit::edit(&db, &id, &title(blank)).unwrap_err();

            assert_eq!(invalid_field(err), "other.title", "{blank:?}");
        }
        assert_eq!(stored(&db, &id).title, "Policy");
    }

    #[test]
    fn a_date_is_stored_as_iso_text() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        let written = edit::edit(&db, &id, &date(" 2026-11-10 "))
            .unwrap()
            .unwrap();

        assert_eq!(stored(&db, &id).date.as_deref(), Some("2026-11-10"));
        assert_eq!(written.label, "Change Date of Policy");
    }

    #[test]
    fn a_blank_date_clears_it() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        change(&db, &id, date("2026-11-10"));

        change(&db, &id, date("  "));

        assert_eq!(stored(&db, &id).date, None);
    }

    #[test]
    fn text_that_is_no_date_names_the_field_and_changes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        change(&db, &id, date("2026-11-10"));

        for garbage in ["10.11.2026", "tomorrow", "2026-13-40", "2026-11"] {
            let err = edit::edit(&db, &id, &date(garbage)).unwrap_err();

            assert_eq!(invalid_field(err), "other.date", "{garbage}");
        }
        assert_eq!(stored(&db, &id).date.as_deref(), Some("2026-11-10"));
    }

    #[test]
    fn a_note_is_trimmed_and_can_be_cleared() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        let written = edit::edit(&db, &id, &note("  Renews in May \n"))
            .unwrap()
            .unwrap();
        assert_eq!(stored(&db, &id).note, "Renews in May");
        assert_eq!(written.label, "Change Note of Policy");

        change(&db, &id, note(""));
        assert_eq!(stored(&db, &id).note, "");
    }

    #[test]
    fn labels_read_in_german() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Police.pdf");

        let labels: Vec<String> = with_lang(Language::De, || {
            [title("Hausrat"), date("2026-11-10"), note("x")]
                .iter()
                .map(|c| edit::edit(&db, &id, c).unwrap().unwrap().label)
                .collect()
        });

        assert_eq!(
            labels,
            [
                "Police in Hausrat umbenennen",
                "Datum von Hausrat ändern",
                "Notiz von Hausrat ändern",
            ]
        );
    }

    #[test]
    fn an_edit_that_changes_nothing_returns_none_and_writes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        change(&db, &id, date("2026-11-10"));
        change(&db, &id, note("Renews in May"));
        let journal = |db: &Db| -> i64 {
            db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
                .unwrap()
        };
        let before = journal(&db);

        for (name, unchanged) in [
            ("same title", title("Policy")),
            ("same title with spaces", title(" Policy ")),
            ("same date", date("2026-11-10")),
            ("same note", note(" Renews in May ")),
        ] {
            assert_eq!(edit::edit(&db, &id, &unchanged).unwrap(), None, "{name}");
        }

        assert_eq!(journal(&db), before);
    }

    #[test]
    fn clearing_a_date_that_is_not_set_changes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        assert_eq!(edit::edit(&db, &id, &date("")).unwrap(), None);
    }

    #[test]
    fn an_unknown_document_is_not_found() {
        let (_dir, db) = open();

        let err = edit::edit(&db, "missing", &note("x")).unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn undoing_each_kind_of_edit_restores_the_earlier_value() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        change(&db, &id, title("Home cover"));
        change(&db, &id, date("2026-11-10"));
        change(&db, &id, note("Renews in May"));

        db.undo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).note, "");
        db.undo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).date, None);
        db.undo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).title, "Policy");

        db.redo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).title, "Home cover");
    }

    #[test]
    fn edits_arrive_from_the_page_as_tagged_camel_case_objects() {
        for (value, expected) in [
            (json!({ "field": "title", "text": "x" }), title("x")),
            (
                json!({ "field": "date", "value": "2026-11-10" }),
                date("2026-11-10"),
            ),
            (json!({ "field": "note", "text": "y" }), note("y")),
        ] {
            assert_eq!(
                serde_json::from_value::<OtherEdit>(value.clone()).unwrap(),
                expected,
                "{value}"
            );
        }
    }
}

mod deleting {
    use super::*;

    #[test]
    fn a_deleted_document_is_gone_but_its_pdf_stays() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        let kept = add(&db, &dir, "Manual.pdf");

        let written = edit::delete(&db, &id).unwrap();

        assert_eq!(written.label, "Delete Policy");
        assert_eq!(written.tables, vec!["others".to_string()]);
        assert_eq!(db.read(|c| other(c, &id)).unwrap(), None);
        assert_eq!(stored(&db, &kept).title, "Manual");
        assert!(dir.path().join("docs/other/policy.pdf").exists());
    }

    #[test]
    fn the_label_reads_in_german() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Police.pdf");

        let written = with_lang(Language::De, || edit::delete(&db, &id)).unwrap();

        assert_eq!(written.label, "Police löschen");
    }

    #[test]
    fn an_unknown_document_is_not_found() {
        let (_dir, db) = open();

        let err = edit::delete(&db, "missing").unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn undoing_a_delete_brings_the_document_back_whole() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");
        change(&db, &id, date("2026-11-10"));
        change(&db, &id, note("Renews in May"));
        let before = stored(&db, &id);
        edit::delete(&db, &id).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(stored(&db, &id), before);
    }

    #[test]
    fn a_deleted_pdf_can_be_added_again() {
        let (dir, db) = open();
        let id = add_at(&db, &dir, "Policy.pdf", "%PDF one", 1);
        edit::delete(&db, &id).unwrap();

        let (_, again) = edit::add(&db, dir.path(), "Policy.pdf", b"%PDF one", 2).unwrap();

        assert_eq!(stored(&db, &again).file, "docs/other/policy.pdf");
    }
}

mod views {
    use super::*;

    fn seeded() -> (TempDir, Db) {
        let (dir, db) = open();
        dated(&db, &dir, "Contract", "2026-03-01", 1);
        let policy = dated(&db, &dir, "Policy", "2026-11-10", 2);
        change(&db, &policy, note("Home cover"));
        dated(&db, &dir, "Assessment", "2025-06-30", 3);
        add_at(&db, &dir, "Manual.pdf", "%PDF manual", 4);
        dated(&db, &dir, "Warranty", "2026-11-10", 5);
        (dir, db)
    }

    fn expected(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        rows.iter()
            .map(|(year, titles)| {
                (
                    year.to_string(),
                    titles.iter().map(|t| t.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn no_documents_list_no_groups() {
        let (_dir, db) = open();

        assert_eq!(listed(&db, ""), vec![]);
    }

    #[test]
    fn groups_run_newest_year_first_with_the_undated_last_and_newest_date_first_within() {
        let (_dir, db) = seeded();

        assert_eq!(
            group_titles(&listed(&db, "")),
            expected(&[
                ("2026", &["Warranty", "Policy", "Contract"]),
                ("2025", &["Assessment"]),
                ("Without date", &["Manual"]),
            ])
        );
    }

    #[test]
    fn the_undated_group_reads_in_german() {
        let (_dir, db) = seeded();

        let groups = with_lang(Language::De, || listed(&db, ""));

        assert_eq!(groups[2].title, "Ohne Datum");
    }

    #[test]
    fn a_group_carries_its_year_and_each_row_its_date_in_the_date_order() {
        let (_dir, db) = seeded();

        let day_first = listed(&db, "");
        let month_first = view::list(&db, "", DateOrder::MonthFirst).unwrap();

        assert_eq!(
            day_first.iter().map(|g| g.year).collect::<Vec<_>>(),
            [Some(2026), Some(2025), None]
        );
        assert_eq!(day_first[0].documents[0].date, "10 Nov 2026");
        assert_eq!(month_first[0].documents[0].date, "Nov 10, 2026");
        assert_eq!(day_first[2].documents[0].date, "");
    }

    #[test]
    fn a_query_keeps_the_documents_whose_title_or_note_contains_it_ignoring_case() {
        let (_dir, db) = seeded();

        for (query, expected_groups) in [
            ("POLICY", expected(&[("2026", &["Policy"])])),
            ("home cover", expected(&[("2026", &["Policy"])])),
            ("  man ", expected(&[("Without date", &["Manual"])])),
            (
                "a",
                expected(&[
                    ("2026", &["Warranty", "Contract"]),
                    ("2025", &["Assessment"]),
                    ("Without date", &["Manual"]),
                ]),
            ),
            ("nothing like it", vec![]),
        ] {
            assert_eq!(
                group_titles(&listed(&db, query)),
                expected_groups,
                "{query}"
            );
        }
    }

    #[test]
    fn the_list_serializes_with_camel_case_names_and_a_null_year_for_the_undated() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Manual.pdf");

        let value = serde_json::to_value(listed(&db, "")).unwrap();

        assert_eq!(
            value,
            json!([{
                "title": "Without date",
                "year": null,
                "documents": [{ "id": id, "title": "Manual", "date": "" }],
            }])
        );
        assert_eq!(
            listed(&db, "")[0].documents[0],
            OtherListRow {
                id,
                title: "Manual".into(),
                date: String::new(),
            }
        );
    }

    #[test]
    fn a_view_carries_title_date_and_note() {
        let (dir, db) = open();
        let id = dated(&db, &dir, "Policy", "2026-11-10", 1);
        change(&db, &id, note("Renews in May"));

        assert_eq!(
            view::view(&db, &id).unwrap(),
            OtherView {
                id: id.clone(),
                title: "Policy".into(),
                date: "2026-11-10".into(),
                note: "Renews in May".into(),
            }
        );
    }

    #[test]
    fn an_undated_document_shows_an_empty_date_field() {
        let (dir, db) = open();
        let id = add(&db, &dir, "Policy.pdf");

        let value = serde_json::to_value(view::view(&db, &id).unwrap()).unwrap();

        assert_eq!(
            value,
            json!({ "id": id, "title": "Policy", "date": "", "note": "" })
        );
    }

    #[test]
    fn an_unknown_document_has_no_view() {
        let (_dir, db) = open();

        let err = view::view(&db, "missing").unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }
}
