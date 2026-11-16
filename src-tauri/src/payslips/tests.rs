//! Payslips against a temp database and a temp config folder: adding, editing, deleting, the home office days
//! rule, the month guess and the views.

use serde_json::json;
use tempfile::TempDir;

use super::edit::{self, PayslipAmount, PayslipEdit};
use super::{PayslipRow, guess, payslip, payslips, telework_days, view};
use crate::db::{Change, Db, Op, Source};
use crate::error::AppError;
use crate::i18n::with_lang;
use crate::settings::{Language, NumberFormat};
use crate::statements::file_hash;
use crate::taxyears::{self, TaxYearRow, YearState};
use crate::test_support::temp_config_dir;

const NOW: i64 = 1_700_000_000;
const DOT: NumberFormat = NumberFormat::DotDecimal;
const COMMA: NumberFormat = NumberFormat::CommaDecimal;

fn open() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn add(db: &Db, dir: &TempDir, month: u32) -> String {
    edit::add(db, dir.path(), 2026, month, None, NOW).unwrap().1
}

fn stored(db: &Db, id: &str) -> PayslipRow {
    db.read(|c| payslip(c, id)).unwrap().unwrap()
}

fn months_of(db: &Db, year: i32) -> Vec<u32> {
    db.read(|c| payslips(c, year))
        .unwrap()
        .iter()
        .map(|p| p.month)
        .collect()
}

fn amount(name: PayslipAmount, value: &str) -> PayslipEdit {
    PayslipEdit::Amount {
        name,
        value: value.into(),
    }
}

fn days(value: &str) -> PayslipEdit {
    PayslipEdit::TeleworkDays {
        value: value.into(),
    }
}

fn period(year: i32, month: u32) -> PayslipEdit {
    PayslipEdit::Period { year, month }
}

fn change(db: &Db, id: &str, change: PayslipEdit) {
    edit::edit(db, id, &change).unwrap().unwrap();
}

fn invalid_field(err: AppError) -> String {
    match err {
        AppError::InvalidField { field, .. } => field,
        other => panic!("expected an invalid field, got {other:?}"),
    }
}

fn year_2026(db: &Db, state: YearState) {
    year_state(db, 2026, state);
}

fn year_state(db: &Db, year: i32, state: YearState) {
    let row = TaxYearRow {
        state,
        closed_at: (state == YearState::Closed).then_some(2),
        ..TaxYearRow::new(year, format!("{year} v1"), "{}".into(), 1)
    };
    db.write(
        Op::new(
            "Seed year",
            vec![Change::insert(taxyears::TABLE, &row).unwrap()],
        ),
        Source::Seed,
    )
    .unwrap();
}

mod adding {
    use super::*;

    #[test]
    fn a_payslip_without_a_pdf_is_stored_empty_and_labelled_with_its_month() {
        let (dir, db) = open();

        let (written, id) = edit::add(&db, dir.path(), 2026, 7, None, NOW).unwrap();

        assert_eq!(written.label, "Add Payslip July 2026");
        assert_eq!(written.tables, vec!["payslips".to_string()]);
        assert_eq!(
            stored(&db, &id),
            PayslipRow {
                id: id.clone(),
                year: 2026,
                month: 7,
                file: None,
                file_hash: None,
                gross: None,
                social: None,
                tax: None,
                net: None,
                payout: None,
                home_office_pay: None,
                telework_days: None,
                note: String::new(),
                created_at: NOW,
            }
        );
    }

    #[test]
    fn a_payslip_with_a_pdf_copies_it_under_its_year_and_month_and_stores_the_hash() {
        let (dir, db) = open();
        let bytes = b"%PDF demo payslip";

        let (_, id) = edit::add(&db, dir.path(), 2026, 7, Some(bytes), NOW).unwrap();

        let row = stored(&db, &id);
        assert_eq!(row.file.as_deref(), Some("docs/2026/payslips/2026-07.pdf"));
        assert_eq!(row.file_hash, Some(file_hash(bytes)));
        assert_eq!(
            std::fs::read(dir.path().join("docs/2026/payslips/2026-07.pdf")).unwrap(),
            bytes
        );
    }

    #[test]
    fn single_digit_months_are_zero_padded_in_the_file_name() {
        let (dir, db) = open();

        let (_, id) = edit::add(&db, dir.path(), 2025, 3, Some(b"%PDF"), NOW).unwrap();

        assert_eq!(
            stored(&db, &id).file.as_deref(),
            Some("docs/2025/payslips/2025-03.pdf")
        );
    }

    #[test]
    fn the_label_reads_in_german() {
        let (dir, db) = open();

        let (written, _) = with_lang(Language::De, || {
            edit::add(&db, dir.path(), 2026, 7, None, NOW)
        })
        .unwrap();

        assert_eq!(written.label, "Lohnzettel Juli 2026 hinzufügen");
    }

    #[test]
    fn a_month_that_has_a_payslip_is_refused_and_leaves_one_row() {
        let (dir, db) = open();
        add(&db, &dir, 7);

        let err = edit::add(&db, dir.path(), 2026, 7, None, NOW).unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "There already is a payslip for July 2026"),
            "{err:?}"
        );
        assert_eq!(months_of(&db, 2026), [7]);
    }

    #[test]
    fn a_taken_month_is_refused_in_german_too() {
        let (dir, db) = open();
        add(&db, &dir, 7);

        let err = with_lang(Language::De, || {
            edit::add(&db, dir.path(), 2026, 7, None, NOW)
        })
        .unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "Für Juli 2026 gibt es schon einen Lohnzettel"),
            "{err:?}"
        );
    }

    #[test]
    fn the_same_month_of_another_year_is_free() {
        let (dir, db) = open();
        add(&db, &dir, 7);

        edit::add(&db, dir.path(), 2025, 7, None, NOW).unwrap();

        assert_eq!(
            (months_of(&db, 2025), months_of(&db, 2026)),
            (vec![7], vec![7])
        );
    }

    #[test]
    fn a_month_outside_one_to_twelve_is_refused_without_a_row_or_file() {
        let (dir, db) = open();
        for month in [0, 13, 99] {
            let err = edit::add(&db, dir.path(), 2026, month, Some(b"%PDF"), NOW).unwrap_err();

            assert!(
                matches!(&err, AppError::InvalidInput(m) if *m == format!("{month} isn't a month")),
                "{month}: {err:?}"
            );
        }
        assert_eq!(months_of(&db, 2026), Vec::<u32>::new());
        assert!(!dir.path().join("docs/2026/payslips").exists());
    }

    #[test]
    fn a_closed_year_refuses_a_new_payslip() {
        let (dir, db) = open();
        year_2026(&db, YearState::Closed);

        let err = edit::add(&db, dir.path(), 2026, 7, None, NOW).unwrap_err();

        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        assert_eq!(months_of(&db, 2026), Vec::<u32>::new());
    }

    #[test]
    fn an_open_year_accepts_a_new_payslip() {
        let (dir, db) = open();
        year_2026(&db, YearState::Open);

        add(&db, &dir, 7);

        assert_eq!(months_of(&db, 2026), [7]);
    }

    #[test]
    fn undoing_an_add_removes_the_payslip_and_redo_brings_it_back() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        db.undo().unwrap().unwrap();
        assert_eq!(months_of(&db, 2026), Vec::<u32>::new());

        db.redo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).month, 7);
    }
}

mod attaching {
    use super::*;

    #[test]
    fn a_pdf_attached_to_a_payslip_without_one_is_copied_and_hashed() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let written = edit::attach(&db, dir.path(), &id, b"%PDF first").unwrap();

        assert_eq!(written.label, "Attach Payslip PDF");
        let row = stored(&db, &id);
        assert_eq!(row.file.as_deref(), Some("docs/2026/payslips/2026-07.pdf"));
        assert_eq!(row.file_hash, Some(file_hash(b"%PDF first")));
    }

    #[test]
    fn a_different_pdf_replaces_the_reference_and_the_old_file_stays() {
        let (dir, db) = open();
        let (_, id) = edit::add(&db, dir.path(), 2026, 7, Some(b"%PDF first"), NOW).unwrap();

        edit::attach(&db, dir.path(), &id, b"%PDF second").unwrap();

        let row = stored(&db, &id);
        assert_eq!(
            row.file.as_deref(),
            Some("docs/2026/payslips/2026-07-2.pdf")
        );
        assert_eq!(row.file_hash, Some(file_hash(b"%PDF second")));
        assert_eq!(
            std::fs::read(dir.path().join("docs/2026/payslips/2026-07.pdf")).unwrap(),
            b"%PDF first"
        );
        assert_eq!(
            std::fs::read(dir.path().join("docs/2026/payslips/2026-07-2.pdf")).unwrap(),
            b"%PDF second"
        );
    }

    #[test]
    fn attaching_the_same_pdf_again_keeps_the_same_path() {
        let (dir, db) = open();
        let (_, id) = edit::add(&db, dir.path(), 2026, 7, Some(b"%PDF first"), NOW).unwrap();

        edit::attach(&db, dir.path(), &id, b"%PDF first").unwrap_or_else(|e| panic!("{e:?}"));

        assert_eq!(
            stored(&db, &id).file.as_deref(),
            Some("docs/2026/payslips/2026-07.pdf")
        );
    }

    #[test]
    fn an_unknown_payslip_is_not_found() {
        let (dir, db) = open();

        let err = edit::attach(&db, dir.path(), "missing", b"%PDF").unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn undoing_an_attach_restores_the_earlier_reference() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        edit::attach(&db, dir.path(), &id, b"%PDF first").unwrap();

        db.undo().unwrap().unwrap();

        let row = stored(&db, &id);
        assert_eq!((row.file, row.file_hash), (None, None));
    }
}

mod editing {
    use super::*;

    #[test]
    fn typed_amounts_become_cents() {
        for (text, expected) in [
            ("1.234,56", 123_456),
            ("1,234.56", 123_456),
            ("€12", 1_200),
            ("12.5", 1_250),
            ("  345 ", 34_500),
            ("0", 0),
            ("-12,50", 1_250),
        ] {
            let (dir, db) = open();
            let id = add(&db, &dir, 7);

            change(&db, &id, amount(PayslipAmount::Gross, text));

            assert_eq!(stored(&db, &id).gross, Some(expected), "{text}");
        }
    }

    #[test]
    fn each_amount_goes_to_its_own_column() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        let slots: [(PayslipAmount, fn(&PayslipRow) -> Option<i64>); 6] = [
            (PayslipAmount::Gross, |r| r.gross),
            (PayslipAmount::Social, |r| r.social),
            (PayslipAmount::Tax, |r| r.tax),
            (PayslipAmount::Net, |r| r.net),
            (PayslipAmount::Payout, |r| r.payout),
            (PayslipAmount::HomeOfficePay, |r| r.home_office_pay),
        ];

        for (i, (name, _)) in slots.iter().enumerate() {
            change(&db, &id, amount(*name, &format!("{}", (i + 1) * 100)));
        }

        let row = stored(&db, &id);
        let values: Vec<Option<i64>> = slots.iter().map(|(_, get)| get(&row)).collect();
        assert_eq!(
            values,
            [10_000, 20_000, 30_000, 40_000, 50_000, 60_000].map(Some)
        );
    }

    #[test]
    fn a_blank_amount_clears_the_value() {
        for blank in ["", "   "] {
            let (dir, db) = open();
            let id = add(&db, &dir, 7);
            change(&db, &id, amount(PayslipAmount::Net, "10"));

            change(&db, &id, amount(PayslipAmount::Net, blank));

            assert_eq!(stored(&db, &id).net, None, "{blank:?}");
        }
    }

    #[test]
    fn text_that_is_no_amount_names_the_field_and_changes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        let fields = [
            (PayslipAmount::Gross, "payslip.gross"),
            (PayslipAmount::Social, "payslip.social"),
            (PayslipAmount::Tax, "payslip.tax"),
            (PayslipAmount::Net, "payslip.net"),
            (PayslipAmount::Payout, "payslip.payout"),
            (PayslipAmount::HomeOfficePay, "payslip.homeOfficePay"),
        ];
        let before = stored(&db, &id);

        for (name, field) in fields {
            for text in ["abc", "1,2,3", "12 EUR", "€"] {
                let err = edit::edit(&db, &id, &amount(name, text)).unwrap_err();

                assert_eq!(invalid_field(err), field, "{field} {text}");
            }
        }

        assert_eq!(stored(&db, &id), before);
    }

    #[test]
    fn telework_days_accept_zero_to_366_and_blank_clears() {
        for (text, expected) in [
            ("0", Some(0)),
            ("122", Some(122)),
            (" 366 ", Some(366)),
            ("", None),
            ("  ", None),
        ] {
            let (dir, db) = open();
            let id = add(&db, &dir, 7);
            change(&db, &id, days("5"));

            change(&db, &id, days(text));

            assert_eq!(stored(&db, &id).telework_days, expected, "{text:?}");
        }
    }

    #[test]
    fn telework_days_outside_the_range_or_not_whole_are_refused() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        for text in ["367", "-1", "12.5", "abc", "12 days", "1,5"] {
            let err = edit::edit(&db, &id, &days(text)).unwrap_err();

            assert_eq!(invalid_field(err), "payslip.teleworkDays", "{text}");
        }
        assert_eq!(stored(&db, &id).telework_days, None);
    }

    #[test]
    fn a_note_is_trimmed_and_can_be_cleared() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        change(
            &db,
            &id,
            PayslipEdit::Note {
                text: "  Bonus month \n".into(),
            },
        );
        assert_eq!(stored(&db, &id).note, "Bonus month");

        change(&db, &id, PayslipEdit::Note { text: "  ".into() });
        assert_eq!(stored(&db, &id).note, "");
    }

    #[test]
    fn an_edit_that_changes_nothing_returns_none_and_writes_nothing() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, amount(PayslipAmount::Gross, "1.000,00"));
        change(&db, &id, days("12"));
        change(
            &db,
            &id,
            PayslipEdit::Note {
                text: "Bonus".into(),
            },
        );
        let journal: i64 = db
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap();

        for (name, unchanged) in [
            ("same amount", amount(PayslipAmount::Gross, "1000")),
            ("amount cleared twice", amount(PayslipAmount::Net, "")),
            ("same days", days("12")),
            ("same days with spaces", days(" 12 ")),
            (
                "same note",
                PayslipEdit::Note {
                    text: " Bonus ".into(),
                },
            ),
            ("same period", period(2026, 7)),
        ] {
            assert_eq!(edit::edit(&db, &id, &unchanged).unwrap(), None, "{name}");
        }

        let after: i64 = db
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(after, journal);
    }

    #[test]
    fn labels_name_the_changed_field_and_the_payslip() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        for (edit_to_make, expected) in [
            (
                amount(PayslipAmount::Gross, "1"),
                "Change Gross of Payslip July 2026",
            ),
            (
                amount(PayslipAmount::HomeOfficePay, "1"),
                "Change Home office allowance from the employer of Payslip July 2026",
            ),
            (days("3"), "Change Home Office Days of Payslip July 2026"),
            (
                PayslipEdit::Note { text: "x".into() },
                "Change Note of Payslip July 2026",
            ),
            (period(2026, 8), "Move Payslip July 2026 to August 2026"),
            (period(2027, 8), "Move Payslip July 2026 to August 2027"),
        ] {
            let written = edit::edit(&db, &id, &edit_to_make).unwrap().unwrap();

            assert_eq!(written.label, expected);
            assert_eq!(written.tables, vec!["payslips".to_string()]);
            let row = stored(&db, &id);
            if (row.year, row.month) != (2026, 7) {
                change(&db, &id, period(2026, 7));
            }
        }
    }

    #[test]
    fn labels_read_in_german() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let labels: Vec<String> = with_lang(Language::De, || {
            [
                amount(PayslipAmount::Gross, "1"),
                days("3"),
                PayslipEdit::Note { text: "x".into() },
                period(2027, 8),
            ]
            .iter()
            .map(|c| edit::edit(&db, &id, c).unwrap().unwrap().label)
            .collect()
        });

        assert_eq!(
            labels,
            [
                "Brutto im Lohnzettel Juli 2026 ändern",
                "Homeoffice-Tage im Lohnzettel Juli 2026 ändern",
                "Notiz im Lohnzettel Juli 2026 ändern",
                "Lohnzettel Juli 2026 auf August 2027 verschieben",
            ]
        );
    }

    #[test]
    fn a_payslip_moves_to_a_free_month_and_keeps_its_values() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, amount(PayslipAmount::Gross, "100"));

        change(&db, &id, period(2026, 8));

        let row = stored(&db, &id);
        assert_eq!((row.year, row.month, row.gross), (2026, 8, Some(10_000)));
        assert_eq!(months_of(&db, 2026), [8]);
    }

    #[test]
    fn a_payslip_moves_to_another_year_in_the_same_month_or_another() {
        let (dir, db) = open();
        let july = add(&db, &dir, 7);
        let august = add(&db, &dir, 8);

        change(&db, &july, period(2027, 7));
        change(&db, &august, period(2025, 12));

        assert_eq!(months_of(&db, 2026), Vec::<u32>::new());
        assert_eq!(months_of(&db, 2027), [7]);
        assert_eq!(months_of(&db, 2025), [12]);
        let row = stored(&db, &august);
        assert_eq!((row.year, row.month), (2025, 12));
    }

    #[test]
    fn a_move_onto_a_taken_month_is_refused() {
        let (dir, db) = open();
        let july = add(&db, &dir, 7);
        add(&db, &dir, 8);

        let err = edit::edit(&db, &july, &period(2026, 8)).unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "There already is a payslip for August 2026"),
            "{err:?}"
        );
        assert_eq!(stored(&db, &july).month, 7);
    }

    #[test]
    fn a_move_onto_a_taken_month_of_another_year_is_refused() {
        let (dir, db) = open();
        let july = add(&db, &dir, 7);
        edit::add(&db, dir.path(), 2027, 8, None, NOW).unwrap();

        let err = edit::edit(&db, &july, &period(2027, 8)).unwrap_err();

        assert!(
            matches!(&err, AppError::InvalidInput(m) if m == "There already is a payslip for August 2027"),
            "{err:?}"
        );
        assert_eq!(
            (stored(&db, &july).year, stored(&db, &july).month),
            (2026, 7)
        );
    }

    #[test]
    fn a_move_to_a_month_outside_one_to_twelve_is_refused() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        for month in [0, 13] {
            let err = edit::edit(&db, &id, &period(2027, month)).unwrap_err();

            assert!(matches!(err, AppError::InvalidInput(_)), "{month}: {err:?}");
        }
        let row = stored(&db, &id);
        assert_eq!((row.year, row.month), (2026, 7));
    }

    #[test]
    fn a_move_into_a_closed_year_is_refused() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        year_state(&db, 2027, YearState::Closed);

        let err = edit::edit(&db, &id, &period(2027, 7)).unwrap_err();

        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        assert_eq!(months_of(&db, 2026), [7]);
        assert_eq!(months_of(&db, 2027), Vec::<u32>::new());
    }

    #[test]
    fn a_move_out_of_a_closed_year_is_refused() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        year_2026(&db, YearState::Closed);

        let err = edit::edit(&db, &id, &period(2027, 7)).unwrap_err();

        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        assert_eq!(months_of(&db, 2026), [7]);
        assert_eq!(months_of(&db, 2027), Vec::<u32>::new());
    }

    #[test]
    fn an_unknown_payslip_is_not_found() {
        let (_dir, db) = open();

        let err = edit::edit(&db, "missing", &days("3")).unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn a_closed_year_refuses_every_edit() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        year_2026(&db, YearState::Closed);

        for (name, refused) in [
            ("amount", amount(PayslipAmount::Gross, "1")),
            ("days", days("3")),
            ("note", PayslipEdit::Note { text: "x".into() }),
            ("period", period(2026, 8)),
        ] {
            let err = edit::edit(&db, &id, &refused).unwrap_err();

            assert!(matches!(err, AppError::InvalidInput(_)), "{name}: {err:?}");
        }
        assert_eq!(stored(&db, &id).month, 7);
    }

    #[test]
    fn undoing_an_edit_restores_the_earlier_value() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, amount(PayslipAmount::Gross, "100"));
        change(&db, &id, amount(PayslipAmount::Gross, "200"));

        db.undo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).gross, Some(10_000));

        db.redo().unwrap().unwrap();
        assert_eq!(stored(&db, &id).gross, Some(20_000));
    }

    #[test]
    fn undoing_a_move_puts_the_payslip_back_in_its_month_and_year() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, period(2027, 9));

        db.undo().unwrap().unwrap();

        let row = stored(&db, &id);
        assert_eq!((row.year, row.month), (2026, 7));
    }

    #[test]
    fn edits_arrive_from_the_page_as_tagged_camel_case_objects() {
        for (value, expected) in [
            (
                json!({ "field": "amount", "name": "homeOfficePay", "value": "5" }),
                amount(PayslipAmount::HomeOfficePay, "5"),
            ),
            (
                json!({ "field": "amount", "name": "gross", "value": "5" }),
                amount(PayslipAmount::Gross, "5"),
            ),
            (json!({ "field": "teleworkDays", "value": "3" }), days("3")),
            (
                json!({ "field": "note", "text": "x" }),
                PayslipEdit::Note { text: "x".into() },
            ),
            (
                json!({ "field": "period", "year": 2027, "month": 5 }),
                period(2027, 5),
            ),
        ] {
            assert_eq!(
                serde_json::from_value::<PayslipEdit>(value.clone()).unwrap(),
                expected,
                "{value}"
            );
        }
    }
}

mod deleting {
    use super::*;

    #[test]
    fn a_deleted_payslip_is_gone_but_its_pdf_stays() {
        let (dir, db) = open();
        let (_, id) = edit::add(&db, dir.path(), 2026, 7, Some(b"%PDF demo"), NOW).unwrap();
        let other = add(&db, &dir, 8);

        let written = edit::delete(&db, &id).unwrap();

        assert_eq!(written.label, "Delete Payslip July 2026");
        assert_eq!(written.tables, vec!["payslips".to_string()]);
        assert_eq!(db.read(|c| payslip(c, &id)).unwrap(), None);
        assert_eq!(months_of(&db, 2026), [8]);
        assert!(stored(&db, &other).month == 8);
        assert!(dir.path().join("docs/2026/payslips/2026-07.pdf").exists());
    }

    #[test]
    fn the_label_reads_in_german() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let written = with_lang(Language::De, || edit::delete(&db, &id)).unwrap();

        assert_eq!(written.label, "Lohnzettel Juli 2026 löschen");
    }

    #[test]
    fn an_unknown_payslip_is_not_found() {
        let (_dir, db) = open();

        let err = edit::delete(&db, "missing").unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn a_closed_year_refuses_the_deletion() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        year_2026(&db, YearState::Closed);

        let err = edit::delete(&db, &id).unwrap_err();

        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        assert_eq!(months_of(&db, 2026), [7]);
    }

    #[test]
    fn undoing_a_delete_brings_the_payslip_back_whole() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, amount(PayslipAmount::Gross, "100"));
        change(&db, &id, days("12"));
        let before = stored(&db, &id);
        edit::delete(&db, &id).unwrap();

        db.undo().unwrap().unwrap();

        assert_eq!(stored(&db, &id), before);
    }
}

mod reading {
    use super::*;

    #[test]
    fn a_years_payslips_come_by_month_whatever_order_they_were_added_in() {
        let (dir, db) = open();
        for month in [9, 2, 5] {
            add(&db, &dir, month);
        }
        edit::add(&db, dir.path(), 2025, 12, None, NOW).unwrap();

        assert_eq!(months_of(&db, 2026), [2, 5, 9]);
        assert_eq!(months_of(&db, 2025), [12]);
        assert_eq!(months_of(&db, 2024), Vec::<u32>::new());
    }

    #[test]
    fn an_unknown_id_is_none() {
        let (_dir, db) = open();

        assert_eq!(db.read(|c| payslip(c, "missing")).unwrap(), None);
    }

    #[test]
    fn the_telework_days_come_from_the_latest_month_that_prints_them() {
        let (dir, db) = open();
        for (month, value) in [
            (3, Some("20")),
            (5, None),
            (7, Some("30")),
            (9, None),
            (11, None),
        ] {
            let id = add(&db, &dir, month);
            if let Some(value) = value {
                change(&db, &id, days(value));
            }
        }

        assert_eq!(db.read(|c| telework_days(c, 2026)).unwrap(), Some((7, 30)));
    }

    #[test]
    fn zero_days_still_count_as_printed() {
        let (dir, db) = open();
        let march = add(&db, &dir, 3);
        change(&db, &march, days("20"));
        let april = add(&db, &dir, 4);
        change(&db, &april, days("0"));

        assert_eq!(db.read(|c| telework_days(c, 2026)).unwrap(), Some((4, 0)));
    }

    #[test]
    fn no_days_anywhere_or_in_another_year_is_none() {
        let (dir, db) = open();
        add(&db, &dir, 3);
        let other_year = edit::add(&db, dir.path(), 2025, 12, None, NOW).unwrap().1;
        change(&db, &other_year, days("50"));

        assert_eq!(db.read(|c| telework_days(c, 2026)).unwrap(), None);
        assert_eq!(db.read(|c| telework_days(c, 2025)).unwrap(), Some((12, 50)));
        assert_eq!(db.read(|c| telework_days(c, 2024)).unwrap(), None);
    }

    #[test]
    fn deleting_the_latest_payslip_hands_the_days_to_the_one_before() {
        let (dir, db) = open();
        let june = add(&db, &dir, 6);
        change(&db, &june, days("25"));
        let july = add(&db, &dir, 7);
        change(&db, &july, days("30"));

        edit::delete(&db, &july).unwrap();

        assert_eq!(db.read(|c| telework_days(c, 2026)).unwrap(), Some((6, 25)));
    }
}

mod guessing {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn the_month_and_year_are_read_from_a_heading() {
        for (line, expected) in [
            ("Lohnset Juli 2025", (2025, 7)),
            ("Payslip March 2026", (2026, 3)),
            ("Gehaltsabrechnung für Dezember 2025", (2025, 12)),
            ("Lohnzettel Jänner 2026", (2026, 1)),
            ("Lohnzettel Januar 2026", (2026, 1)),
            ("January 2026", (2026, 1)),
            ("Februar 2024", (2024, 2)),
            ("März 2026", (2026, 3)),
            ("April 2026", (2026, 4)),
            ("Mai 2026", (2026, 5)),
            ("May 2026", (2026, 5)),
            ("Juni 2026", (2026, 6)),
            ("August 2026", (2026, 8)),
            ("September 2026", (2026, 9)),
            ("October 2026", (2026, 10)),
            ("Oktober 2026", (2026, 10)),
            ("November 2026", (2026, 11)),
            ("JULI 2025", (2025, 7)),
            ("juli 2025", (2025, 7)),
            ("Abrechnung (Juli 2025):", (2025, 7)),
            ("Juli 2025, Seite 1", (2025, 7)),
        ] {
            assert_eq!(guess::month(&lines(&[line])), Some(expected), "{line}");
        }
    }

    #[test]
    fn the_first_matching_line_wins() {
        let text = lines(&[
            "Firma Beispiel GmbH",
            "Lohnset Juli 2025",
            "Lohnset August 2025",
        ]);

        assert_eq!(guess::month(&text), Some((2025, 7)));
    }

    #[test]
    fn the_heading_must_sit_in_the_first_twelve_lines() {
        let mut text = vec!["Zeile".to_string(); 11];
        text.push("Lohnset Juli 2025".into());
        assert_eq!(guess::month(&text), Some((2025, 7)));

        let mut text = vec!["Zeile".to_string(); 12];
        text.push("Lohnset Juli 2025".into());
        assert_eq!(guess::month(&text), None);
    }

    #[test]
    fn text_without_a_month_and_year_pair_gives_nothing() {
        for (name, text) in [
            ("nothing", lines(&[])),
            ("empty line", lines(&[""])),
            ("month only", lines(&["Lohnset Juli"])),
            ("year only", lines(&["Lohnset 2025"])),
            ("year before month", lines(&["2025 Juli"])),
            ("month apart from year", lines(&["Juli bis 2025"])),
            ("abbreviation", lines(&["Jul 2025"])),
            ("year too early", lines(&["Juli 1800"])),
            ("year too late", lines(&["Juli 2300"])),
            ("no number", lines(&["Juli abc"])),
            ("unrelated", lines(&["Brutto 3.000,00", "Netto 2.100,00"])),
        ] {
            assert_eq!(guess::month(&text), None, "{name}");
        }
    }
}

mod views {
    use super::*;

    fn fill(db: &Db, id: &str, values: [&str; 6]) {
        for (name, value) in [
            PayslipAmount::Gross,
            PayslipAmount::Social,
            PayslipAmount::Tax,
            PayslipAmount::Net,
            PayslipAmount::Payout,
            PayslipAmount::HomeOfficePay,
        ]
        .into_iter()
        .zip(values)
        {
            if !value.is_empty() {
                change(db, id, amount(name, value));
            }
        }
    }

    #[test]
    fn the_year_always_lists_twelve_months_in_order() {
        let (_dir, db) = open();

        let year = view::year_view(&db, 2026, DOT).unwrap();

        let months: Vec<(u32, &str)> = year
            .months
            .iter()
            .map(|m| (m.month, m.name.as_str()))
            .collect();
        assert_eq!(
            months,
            [
                (1, "January"),
                (2, "February"),
                (3, "March"),
                (4, "April"),
                (5, "May"),
                (6, "June"),
                (7, "July"),
                (8, "August"),
                (9, "September"),
                (10, "October"),
                (11, "November"),
                (12, "December"),
            ]
        );
        assert!(
            year.months
                .iter()
                .all(|m| m.id.is_none() && m.payout.is_none() && !m.has_pdf)
        );
    }

    #[test]
    fn a_month_with_a_payslip_shows_its_id_payout_and_pdf() {
        let (dir, db) = open();
        let (_, march) = edit::add(&db, dir.path(), 2026, 3, Some(b"%PDF demo"), NOW).unwrap();
        let july = add(&db, &dir, 7);
        fill(&db, &march, ["3000", "500", "400", "2100", "2345.60", "0"]);
        let august = add(&db, &dir, 8);

        let year = view::year_view(&db, 2026, DOT).unwrap();
        let comma = view::year_view(&db, 2026, COMMA).unwrap();

        let shown: Vec<(u32, Option<&str>, Option<&str>, bool)> = year
            .months
            .iter()
            .filter(|m| m.id.is_some())
            .map(|m| (m.month, m.id.as_deref(), m.payout.as_deref(), m.has_pdf))
            .collect();
        assert_eq!(
            shown,
            [
                (3, Some(march.as_str()), Some("€2,345.60"), true),
                (7, Some(july.as_str()), None, false),
                (8, Some(august.as_str()), None, false),
            ]
        );
        assert_eq!(comma.months[2].payout.as_deref(), Some("€2.345,60"));
    }

    #[test]
    fn totals_sum_the_typed_values_and_leave_out_the_payout() {
        let (dir, db) = open();
        let march = add(&db, &dir, 3);
        let july = add(&db, &dir, 7);
        add(&db, &dir, 9);
        fill(&db, &march, ["3000", "500", "400", "2100", "2100", "20"]);
        fill(&db, &july, ["3200.50", "", "450", "2300", "2300", "20"]);

        let year = view::year_view(&db, 2026, DOT).unwrap();

        let totals: Vec<(&str, &str)> = year
            .totals
            .iter()
            .map(|t| (t.label.as_str(), t.value.as_str()))
            .collect();
        assert_eq!(
            totals,
            [
                ("Gross", "€6,200.50"),
                ("Social insurance", "€500.00"),
                ("Income tax", "€850.00"),
                ("Net", "€4,400.00"),
                ("Home office allowance from the employer", "€40.00"),
            ]
        );
    }

    #[test]
    fn totals_of_an_empty_year_are_zero_and_use_the_number_format() {
        let (_dir, db) = open();

        let year = view::year_view(&db, 2026, COMMA).unwrap();

        assert_eq!(year.totals.len(), 5);
        assert!(year.totals.iter().all(|t| t.value == "€0,00"));
        assert_eq!(year.telework, None);
    }

    #[test]
    fn totals_of_a_year_ignore_the_payslips_of_other_years() {
        let (dir, db) = open();
        let id = add(&db, &dir, 3);
        fill(&db, &id, ["1000", "", "", "", "", ""]);
        let other = edit::add(&db, dir.path(), 2025, 3, None, NOW).unwrap().1;
        fill(&db, &other, ["9999", "", "", "", "", ""]);

        let year = view::year_view(&db, 2026, DOT).unwrap();

        assert_eq!(year.totals[0].value, "€1,000.00");
    }

    #[test]
    fn the_telework_text_names_the_days_and_the_month_they_stand_at() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, days("122"));
        add(&db, &dir, 8);

        let english = view::year_view(&db, 2026, DOT).unwrap();
        let german = with_lang(Language::De, || view::year_view(&db, 2026, DOT)).unwrap();

        assert_eq!(english.telework.as_deref(), Some("122 days · as of July"));
        assert_eq!(german.telework.as_deref(), Some("122 Tage · Stand Juli"));
    }

    #[test]
    fn the_count_says_how_many_of_twelve_months_have_a_payslip() {
        let (dir, db) = open();
        let empty = view::year_view(&db, 2026, DOT).unwrap().count;
        for month in 1..=10 {
            add(&db, &dir, month);
        }

        let english = view::year_view(&db, 2026, DOT).unwrap().count;
        let german = with_lang(Language::De, || view::year_view(&db, 2026, DOT))
            .unwrap()
            .count;

        assert_eq!(empty, "0 of 12 payslips");
        assert_eq!(english, "10 of 12 payslips");
        assert_eq!(german, "10 von 12 Lohnzetteln");
    }

    #[test]
    fn the_list_holds_every_year_with_a_payslip_or_an_open_tax_year_newest_first() {
        let (dir, db) = open();
        add(&db, &dir, 3);
        edit::add(&db, dir.path(), 2024, 5, None, NOW).unwrap();
        edit::add(&db, dir.path(), 2026, 4, None, NOW).unwrap();
        year_state(&db, 2027, YearState::Open);
        year_state(&db, 2025, YearState::Closed);
        year_state(&db, 2026, YearState::Open);

        let years = view::list(&db, DOT).unwrap();

        assert_eq!(
            years
                .iter()
                .map(|y| (y.year, y.open, y.count.as_str()))
                .collect::<Vec<_>>(),
            [
                (2027, true, "0 of 12 payslips"),
                (2026, true, "2 of 12 payslips"),
                (2024, false, "1 of 12 payslips"),
            ]
        );
    }

    #[test]
    fn each_year_in_the_list_is_the_full_year_view() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        fill(&db, &id, ["1000", "", "", "", "800", ""]);
        year_2026(&db, YearState::Open);

        let years = view::list(&db, COMMA).unwrap();

        assert_eq!(years, vec![view::year_view(&db, 2026, COMMA).unwrap()]);
    }

    #[test]
    fn without_payslips_or_open_years_the_list_is_empty() {
        let (_dir, db) = open();
        year_state(&db, 2025, YearState::Closed);

        assert_eq!(view::list(&db, DOT).unwrap(), vec![]);
    }

    #[test]
    fn the_form_lists_the_twelve_month_names_for_the_picker() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let english = view::payslip_view(&db, &id, DOT).unwrap().month_names;
        let german = with_lang(Language::De, || view::payslip_view(&db, &id, DOT))
            .unwrap()
            .month_names;

        assert_eq!(
            english,
            [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December"
            ]
        );
        assert_eq!(german.len(), 12);
        assert_eq!(german[0], "Januar");
        assert_eq!(german[2], "März");
    }

    #[test]
    fn the_year_is_open_only_with_an_open_tax_year_row() {
        let (_dir, db) = open();
        assert!(!view::year_view(&db, 2026, DOT).unwrap().open);

        year_2026(&db, YearState::Open);
        assert!(view::year_view(&db, 2026, DOT).unwrap().open);
    }

    #[test]
    fn a_closed_year_is_not_open() {
        let (_dir, db) = open();
        year_2026(&db, YearState::Closed);

        assert!(!view::year_view(&db, 2026, DOT).unwrap().open);
    }

    #[test]
    fn the_year_travels_with_camel_case_field_names() {
        let (dir, db) = open();
        let id = add(&db, &dir, 1);
        change(&db, &id, days("3"));

        let value = serde_json::to_value(view::year_view(&db, 2026, DOT).unwrap()).unwrap();

        let keys = |v: &serde_json::Value| -> Vec<String> {
            let mut keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
            keys.sort_unstable();
            keys
        };
        assert_eq!(
            keys(&value),
            ["count", "months", "open", "telework", "totals", "year"]
        );
        assert_eq!(value["count"], json!("1 of 12 payslips"));
        assert_eq!(
            keys(&value["months"][0]),
            ["hasPdf", "id", "month", "name", "payout"]
        );
        assert_eq!(keys(&value["totals"][0]), ["label", "value"]);
        assert_eq!(value["months"][1]["id"], json!(null));
    }

    #[test]
    fn the_form_shows_every_amount_in_the_number_format() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        fill(&db, &id, ["1234.56", "200", "300", "700,5", "1000", "12"]);

        let dot = view::payslip_view(&db, &id, DOT).unwrap();
        let comma = view::payslip_view(&db, &id, COMMA).unwrap();

        let values = |v: &view::PayslipView| -> Vec<(PayslipAmount, String)> {
            v.amounts
                .iter()
                .map(|a| (a.name, a.value.clone()))
                .collect()
        };
        assert_eq!(
            values(&dot),
            [
                (PayslipAmount::Gross, "1,234.56".to_string()),
                (PayslipAmount::Social, "200.00".to_string()),
                (PayslipAmount::Tax, "300.00".to_string()),
                (PayslipAmount::Net, "700.50".to_string()),
                (PayslipAmount::Payout, "1,000.00".to_string()),
                (PayslipAmount::HomeOfficePay, "12.00".to_string()),
            ]
        );
        assert_eq!(comma.amounts[0].value, "1.234,56");
        assert_eq!(comma.amounts[3].value, "700,50");
    }

    #[test]
    fn untyped_values_show_as_empty_strings() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let form = view::payslip_view(&db, &id, DOT).unwrap();

        assert!(form.amounts.iter().all(|a| a.value.is_empty()));
        assert_eq!((form.telework_days.as_str(), form.note.as_str()), ("", ""));
        assert!(!form.has_pdf);
    }

    #[test]
    fn a_cleared_amount_shows_empty_again_while_zero_shows_as_zero() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);
        change(&db, &id, amount(PayslipAmount::Gross, "0"));
        change(&db, &id, amount(PayslipAmount::Net, "10"));
        change(&db, &id, amount(PayslipAmount::Net, ""));

        let form = view::payslip_view(&db, &id, DOT).unwrap();

        assert_eq!(form.amounts[0].value, "0.00");
        assert_eq!(form.amounts[3].value, "");
    }

    #[test]
    fn the_form_carries_title_days_note_pdf_and_state() {
        let (dir, db) = open();
        let (_, id) = edit::add(&db, dir.path(), 2026, 7, Some(b"%PDF demo"), NOW).unwrap();
        change(&db, &id, days("122"));
        change(
            &db,
            &id,
            PayslipEdit::Note {
                text: "Bonus".into(),
            },
        );
        year_2026(&db, YearState::Open);

        let form = view::payslip_view(&db, &id, DOT).unwrap();

        assert_eq!(
            (
                form.id.as_str(),
                form.year,
                form.month,
                form.title.as_str(),
                form.open,
                form.has_pdf,
                form.telework_days.as_str(),
                form.note.as_str(),
            ),
            (
                id.as_str(),
                2026,
                7,
                "Payslip July 2026",
                true,
                true,
                "122",
                "Bonus"
            )
        );
    }

    #[test]
    fn the_title_labels_and_hints_read_in_german() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let form = with_lang(Language::De, || view::payslip_view(&db, &id, DOT)).unwrap();

        assert_eq!(form.title, "Lohnzettel Juli 2026");
        let labels: Vec<&str> = form.amounts.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Brutto",
                "Sozialversicherung",
                "Lohnsteuer",
                "Netto",
                "Auszahlung",
                "Homeoffice-Pauschale vom Arbeitgeber"
            ]
        );
        let hints: Vec<&str> = form.amounts.iter().map(|a| a.hint.as_str()).collect();
        assert_eq!(
            hints,
            [
                "",
                "„SV Beitrag lfd“",
                "„LSt Beitrag lfd“",
                "",
                "Der Betrag auf dem Kontoauszug",
                "Steuerfrei, nur wenn der Arbeitgeber sie zahlt"
            ]
        );
    }

    #[test]
    fn the_english_labels_and_hints_say_where_the_value_is_printed() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let form = view::payslip_view(&db, &id, DOT).unwrap();

        let fields: Vec<(&str, &str)> = form
            .amounts
            .iter()
            .map(|a| (a.label.as_str(), a.hint.as_str()))
            .collect();
        assert_eq!(
            fields,
            [
                ("Gross", "“Brutto”"),
                ("Social insurance", "“SV Beitrag lfd”"),
                ("Income tax", "“LSt Beitrag lfd”"),
                ("Net", "“Netto”"),
                ("Payout", "“Auszahlung”, the amount on your bank statement"),
                (
                    "Home office allowance from the employer",
                    "Tax-free, only when the employer pays it"
                ),
            ]
        );
    }

    #[test]
    fn an_unknown_payslip_has_no_form() {
        let (_dir, db) = open();

        let err = view::payslip_view(&db, "missing", DOT).unwrap_err();

        assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn the_form_travels_with_camel_case_field_names_and_amount_names() {
        let (dir, db) = open();
        let id = add(&db, &dir, 7);

        let value = serde_json::to_value(view::payslip_view(&db, &id, DOT).unwrap()).unwrap();

        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "amounts",
                "hasPdf",
                "id",
                "month",
                "monthNames",
                "note",
                "open",
                "teleworkDays",
                "title",
                "year"
            ]
        );
        let names: Vec<serde_json::Value> = value["amounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["name"].clone())
            .collect();
        assert_eq!(
            names,
            [
                json!("gross"),
                json!("social"),
                json!("tax"),
                json!("net"),
                json!("payout"),
                json!("homeOfficePay")
            ]
        );
    }
}
