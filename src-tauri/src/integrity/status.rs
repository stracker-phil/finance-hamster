//! Settings → Status: a health check computed on this Mac. Statement chains and balances, the last file check,
//! the journal against the database, the inbox, and the rules files.

use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

use super::FileCheck;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::db::journal;
use crate::documents::store as documents;
use crate::error::AppResult;
use crate::statements::store as statements;
use crate::statements::view::{self as statement_view, AccountCoverage};
use crate::taxrules;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatusRow {
    pub label: String,
    pub detail: String,
    /// `OK`, `Gap`, `41 of 41`.
    pub value: String,
    pub tone: Tone,
    /// What a click copies when it isn't the value itself, e.g. a file's full path.
    pub copy_text: Option<String>,
}

/// How a value reads: only ok, warning and error carry an icon; info is a plain fact.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Tone {
    Ok,
    Warning,
    Error,
    Info,
}

impl Tone {
    fn ok_or(good: bool, otherwise: Tone) -> Tone {
        if good { Tone::Ok } else { otherwise }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct StatusGroup {
    pub title: String,
    pub rows: Vec<StatusRow>,
}

fn row(
    label: impl Into<String>,
    detail: impl Into<String>,
    value: impl Into<String>,
    tone: Tone,
) -> StatusRow {
    StatusRow {
        label: label.into(),
        detail: detail.into(),
        value: value.into(),
        tone,
        copy_text: None,
    }
}

impl StatusRow {
    fn copying(self, text: impl Into<String>) -> StatusRow {
        StatusRow {
            copy_text: Some(text.into()),
            ..self
        }
    }
}

/// `13 Nov` for a time in milliseconds, in the local timezone.
fn local_day(ms: i64, order: DateOrder) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| date::day_month(t.with_timezone(&chrono::Local).date_naive(), order))
        .unwrap_or_default()
}

fn statement_row(db: &Db, account: &AccountCoverage, order: DateOrder) -> AppResult<StatusRow> {
    let until = db
        .read(|c| statements::statements_of(c, &account.id))?
        .iter()
        .filter_map(|s| date::parse(&s.to_date))
        .max()
        .map(|d| format!("until {}", date::day_month(d, order)));
    let name = account.name.clone();
    Ok(match (until, &account.note) {
        (None, _) => row(name, "No statement imported yet", "None", Tone::Info),
        // The chain's note names its first break: a missing number or balances that don't connect.
        (Some(until), Some(note)) => row(name, format!("{note} · {until}"), "Gap", Tone::Error),
        (Some(until), None) => row(
            name,
            format!("Chain complete · balances check out · {until}"),
            "OK",
            Tone::Ok,
        ),
    })
}

fn statement_rows(db: &Db, today: NaiveDate, order: DateOrder) -> AppResult<Vec<StatusRow>> {
    // The rows show the chain only, never how far behind an account is.
    statement_view::coverage(db, today, "", order, u32::MAX)?
        .accounts
        .iter()
        .map(|account| statement_row(db, account, order))
        .collect()
}

fn journal_row(db: &Db, config_dir: &Path) -> AppResult<StatusRow> {
    let in_db: i64 =
        db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))?;
    let in_file = std::fs::read_to_string(config_dir.join(journal::FILE_NAME))
        .map(|text| text.lines().filter(|l| !l.trim().is_empty()).count() as i64)
        .unwrap_or(0);
    let checked = if in_db == in_file {
        row(
            "Journal",
            format!(
                "{} · {in_db} entries · matches the database",
                journal::FILE_NAME
            ),
            "OK",
            Tone::Ok,
        )
    } else {
        row(
            "Journal",
            format!(
                "{} · {in_file} lines, the database has {in_db}; caught up at the next launch",
                journal::FILE_NAME
            ),
            "Behind",
            Tone::Warning,
        )
    };
    Ok(checked.copying(config_dir.join(journal::FILE_NAME).display().to_string()))
}

fn checksum_row(check: Option<&FileCheck>, order: DateOrder) -> StatusRow {
    match check {
        Some(check) => {
            let good = check.checked as usize - check.problems.len();
            row(
                "PDF checksums",
                format!("Last verified {}", local_day(check.at, order)),
                format!("{good} of {}", check.checked),
                Tone::ok_or(check.problems.is_empty(), Tone::Error),
            )
        }
        None => row(
            "PDF checksums",
            "Not verified since launch",
            "Not run",
            Tone::Info,
        ),
    }
}

fn inbox_row(db: &Db) -> AppResult<StatusRow> {
    let inbox = db
        .read(documents::documents)?
        .iter()
        .filter(|d| d.pay_date.is_none() && d.state != documents::DocState::Excluded)
        .count();
    let files = if inbox == 1 {
        "1 file".to_string()
    } else {
        format!("{inbox} files")
    };
    Ok(row(
        "Inbox",
        "docs/inbox/ · documents without a payment date",
        files,
        Tone::ok_or(inbox == 0, Tone::Info),
    ))
}

fn file_rows(
    db: &Db,
    config_dir: &Path,
    check: Option<&FileCheck>,
    order: DateOrder,
) -> AppResult<Vec<StatusRow>> {
    Ok(vec![
        checksum_row(check, order),
        journal_row(db, config_dir)?,
        inbox_row(db)?,
    ])
}

fn rules_rows(db: &Db, config_dir: &Path, order: DateOrder) -> AppResult<Vec<StatusRow>> {
    let label = |ms: i64| local_day(ms, order);
    let files = taxrules::overview(db, config_dir, &label)?;
    let mut rows: Vec<StatusRow> = files
        .iter()
        .map(|f| {
            if f.problems.is_empty() && !f.missing {
                let when = f
                    .loaded_on
                    .as_ref()
                    .map(|d| format!(" · loaded {d}"))
                    .unwrap_or_default();
                row(&f.file_name, format!("Valid{when}"), "OK", Tone::Ok)
            } else if f.missing {
                row(
                    &f.file_name,
                    "The file is gone; the year keeps what it applied",
                    "Missing",
                    Tone::Warning,
                )
            } else {
                row(&f.file_name, f.problems.join(" · "), "Invalid", Tone::Error)
            }
        })
        .collect();
    let unverified: u32 = files.iter().map(|f| f.unverified).sum();
    rows.push(row(
        "Unverified values",
        "Marked \"verified\": false in the files",
        unverified.to_string(),
        Tone::ok_or(unverified == 0, Tone::Warning),
    ));
    Ok(rows)
}

pub fn status(
    db: &Db,
    config_dir: &Path,
    check: Option<&FileCheck>,
    today: NaiveDate,
    order: DateOrder,
) -> AppResult<Vec<StatusGroup>> {
    Ok(vec![
        StatusGroup {
            title: "Statements".into(),
            rows: statement_rows(db, today, order)?,
        },
        StatusGroup {
            title: "Files".into(),
            rows: file_rows(db, config_dir, check, order)?,
        },
        StatusGroup {
            title: "Rules".into(),
            rows: rules_rows(db, config_dir, order)?,
        },
    ])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::db::{Change, Op, Source};
    use crate::documents::store::{DOCUMENTS, DocState, DocumentRow};
    use crate::integrity::{FileProblem, FileTrouble};
    use crate::statements::import;
    use crate::statements::import_tests::open_with_accounts_and_year;
    use crate::taxyears::{self, TaxYearRow};
    use crate::test_support::rules::{minimal_rules, minimal_text, rules_text, set};
    use crate::test_support::statements::{Statement, demo_month};
    use crate::test_support::temp_config_dir;

    const ORDER: DateOrder = DateOrder::DayFirst;
    const NOW: i64 = 1_700_000_000_000;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn label(ms: i64) -> String {
        let day = chrono::DateTime::from_timestamp_millis(ms)
            .unwrap()
            .with_timezone(&chrono::Local)
            .date_naive();
        date::day_month(day, ORDER)
    }

    fn import_all(db: &Db, dir: &Path, statements: &[Statement]) {
        for statement in statements {
            let pending = import::prepare(db, dir, statement.pdf()).unwrap();
            import::commit(db, dir, pending, NOW, Source::Import).unwrap();
        }
    }

    fn groups(db: &Db, dir: &Path, check: Option<&FileCheck>) -> Vec<StatusGroup> {
        status(db, dir, check, today(), ORDER).unwrap()
    }

    fn rows_of(groups: &[StatusGroup], title: &str) -> Vec<StatusRow> {
        groups
            .iter()
            .find(|g| g.title == title)
            .unwrap_or_else(|| panic!("no group {title}"))
            .rows
            .clone()
    }

    fn row_of(groups: &[StatusGroup], title: &str, label: &str) -> StatusRow {
        rows_of(groups, title)
            .into_iter()
            .find(|r| r.label == label)
            .unwrap_or_else(|| panic!("no row {label}"))
    }

    fn clean_check() -> FileCheck {
        FileCheck {
            checked: 2,
            problems: vec![],
            at: NOW,
        }
    }

    fn add_document(db: &Db, id: &str, pay_date: Option<&str>, state: DocState) {
        let row = DocumentRow {
            id: id.into(),
            state,
            file: format!("docs/inbox/{id}.pdf"),
            original_name: format!("{id}.pdf"),
            file_hash: crate::statements::file_hash(id.as_bytes()),
            vendor_id: None,
            pay_date: pay_date.map(String::from),
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

    fn journal_entries(db: &Db) -> i64 {
        db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM journal", [], |r| r.get(0))?))
            .unwrap()
    }

    fn rules_db() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    fn open_year(db: &Db, year: i32, text: &str, loaded_at: i64) {
        let row = TaxYearRow::new(year, format!("{year} v1"), text.into(), loaded_at);
        db.write(
            Op::new(
                "Add year",
                vec![Change::insert(taxyears::TABLE, &row).unwrap()],
            ),
            Source::App,
        )
        .unwrap();
    }

    fn unverified_row(count: u32) -> StatusRow {
        row(
            "Unverified values",
            "Marked \"verified\": false in the files",
            count.to_string(),
            if count == 0 { Tone::Ok } else { Tone::Warning },
        )
    }

    #[test]
    fn the_groups_come_in_the_order_statements_files_rules() {
        let (dir, db) = open_with_accounts_and_year();

        let titles: Vec<String> = groups(&db, dir.path(), None)
            .into_iter()
            .map(|g| g.title)
            .collect();

        assert_eq!(titles, ["Statements", "Files", "Rules"]);
    }

    #[test]
    fn a_complete_chain_with_matching_balances_is_ok() {
        let (dir, db) = open_with_accounts_and_year();
        let first = demo_month(1, 2026, 9, 100_000);
        let second = demo_month(2, 2026, 10, first.new_balance());
        import_all(&db, dir.path(), &[first, second]);

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Statements", "Private"),
            row(
                "Private",
                "Chain complete · balances check out · until 21 Oct",
                "OK",
                Tone::Ok
            )
        );
    }

    #[test]
    fn a_numbering_gap_shows_the_coverage_note_and_one_gap() {
        let (dir, db) = open_with_accounts_and_year();
        let first = demo_month(1, 2026, 8, 100_000);
        let third = demo_month(3, 2026, 10, first.new_balance());
        import_all(&db, dir.path(), &[first, third]);

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Statements", "Private"),
            row(
                "Private",
                "Statement Nr. 002/2026 was never imported: no lines between 21 Aug and 1 Oct. · until 21 Oct",
                "Gap",
                Tone::Error
            )
        );
    }

    #[test]
    fn balances_that_do_not_connect_are_reported_as_a_gap_with_the_two_statements() {
        let (dir, db) = open_with_accounts_and_year();
        let first = demo_month(1, 2026, 9, 100_000);
        let second = demo_month(2, 2026, 10, first.new_balance() + 100);
        import_all(&db, dir.path(), &[first, second]);

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Statements", "Private"),
            row(
                "Private",
                "The balances of Nr. 001/2026 and Nr. 002/2026 don't connect. · until 21 Oct",
                "Gap",
                Tone::Error
            )
        );
    }

    #[test]
    fn an_account_without_statements_says_none_imported() {
        let (dir, db) = open_with_accounts_and_year();

        assert_eq!(
            rows_of(&groups(&db, dir.path(), None), "Statements"),
            ["Private", "Household", "Business"].map(|name| row(
                name,
                "No statement imported yet",
                "None",
                Tone::Info
            ))
        );
    }

    #[test]
    fn without_a_check_the_checksums_are_not_run() {
        let (dir, db) = open_with_accounts_and_year();

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "PDF checksums"),
            row(
                "PDF checksums",
                "Not verified since launch",
                "Not run",
                Tone::Info
            )
        );
    }

    #[test]
    fn a_clean_check_reports_every_file_good_and_the_day_it_ran() {
        let (dir, db) = open_with_accounts_and_year();

        assert_eq!(
            row_of(
                &groups(&db, dir.path(), Some(&clean_check())),
                "Files",
                "PDF checksums"
            ),
            row(
                "PDF checksums",
                format!("Last verified {}", label(NOW)),
                "2 of 2",
                Tone::Ok
            )
        );
    }

    #[test]
    fn a_check_with_problems_counts_only_the_good_files_and_is_not_ok() {
        let (dir, db) = open_with_accounts_and_year();
        let check = FileCheck {
            checked: 4,
            problems: vec![FileProblem {
                trouble: FileTrouble::Missing,
                file: "docs/2026/2026-11-10-invoice-a.pdf".into(),
                what: "Document · invoice-a".into(),
            }],
            at: NOW,
        };

        assert_eq!(
            row_of(
                &groups(&db, dir.path(), Some(&check)),
                "Files",
                "PDF checksums"
            ),
            row(
                "PDF checksums",
                format!("Last verified {}", label(NOW)),
                "3 of 4",
                Tone::Error
            )
        );
    }

    #[test]
    fn a_journal_file_with_as_many_lines_as_the_database_rows_is_ok() {
        let (dir, db) = open_with_accounts_and_year();
        let journal_path = dir.path().join(journal::FILE_NAME);
        let mut text = std::fs::read_to_string(&journal_path).unwrap();
        text.push_str("\n   \n\n");
        std::fs::write(&journal_path, text).unwrap();

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "Journal"),
            row(
                "Journal",
                format!(
                    "journal.jsonl · {} entries · matches the database",
                    journal_entries(&db)
                ),
                "OK",
                Tone::Ok
            )
            .copying(dir.path().join("journal.jsonl").display().to_string())
        );
    }

    #[test]
    fn a_journal_file_that_lost_lines_is_behind() {
        let (dir, db) = open_with_accounts_and_year();
        std::fs::write(dir.path().join(journal::FILE_NAME), "").unwrap();

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "Journal"),
            row(
                "Journal",
                format!(
                    "journal.jsonl · 0 lines, the database has {}; caught up at the next launch",
                    journal_entries(&db)
                ),
                "Behind",
                Tone::Warning
            )
            .copying(dir.path().join("journal.jsonl").display().to_string())
        );
    }

    #[test]
    fn the_inbox_counts_documents_without_a_payment_date_that_are_not_excluded() {
        let (dir, db) = open_with_accounts_and_year();
        let inbox = |label: &str| {
            row(
                "Inbox",
                "docs/inbox/ · documents without a payment date",
                label,
                if label == "0 files" {
                    Tone::Ok
                } else {
                    Tone::Info
                },
            )
        };
        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "Inbox"),
            inbox("0 files")
        );

        add_document(&db, "doc-a", None, DocState::Draft);
        add_document(&db, "doc-dated", Some("2026-11-10"), DocState::Draft);
        add_document(&db, "doc-out", None, DocState::Excluded);
        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "Inbox"),
            inbox("1 file")
        );

        add_document(&db, "doc-b", None, DocState::Draft);
        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Files", "Inbox"),
            inbox("2 files")
        );
    }

    #[test]
    fn without_rules_files_only_the_unverified_count_remains() {
        let (dir, db) = rules_db();

        assert_eq!(
            rows_of(&groups(&db, dir.path(), None), "Rules"),
            [unverified_row(0)]
        );
    }

    #[test]
    fn a_valid_open_year_file_is_listed_with_the_day_it_was_loaded() {
        let (dir, db) = rules_db();
        let text = minimal_text(2026);
        crate::taxrules::files::create(dir.path(), 2026, &text).unwrap();
        open_year(&db, 2026, &text, NOW);

        assert_eq!(
            rows_of(&groups(&db, dir.path(), None), "Rules"),
            [
                row(
                    "rules-2026.json",
                    format!("Valid · loaded {}", label(NOW)),
                    "OK",
                    Tone::Ok
                ),
                unverified_row(0)
            ]
        );
    }

    #[test]
    fn an_invalid_file_shows_its_problems_and_is_invalid() {
        let (dir, db) = rules_db();
        let mut rules = minimal_rules(2026);
        set(&mut rules, "/schemaVersion", json!(2));
        crate::taxrules::files::create(dir.path(), 2026, &rules_text(&rules)).unwrap();
        open_year(&db, 2026, &minimal_text(2026), NOW);

        assert_eq!(
            rows_of(&groups(&db, dir.path(), None), "Rules")[0],
            row(
                "rules-2026.json",
                "schemaVersion: this Hamster reads version 1, not 2",
                "Invalid",
                Tone::Error
            )
        );
    }

    #[test]
    fn a_year_whose_file_is_gone_is_missing() {
        let (dir, db) = rules_db();
        open_year(&db, 2026, &minimal_text(2026), NOW);

        assert_eq!(
            rows_of(&groups(&db, dir.path(), None), "Rules")[0],
            row(
                "rules-2026.json",
                "The file is gone; the year keeps what it applied",
                "Missing",
                Tone::Warning
            )
        );
    }

    #[test]
    fn a_row_serializes_with_a_lower_case_tone_and_a_camel_case_copy_text() {
        let plain = row("Inbox", "detail", "0 files", Tone::Ok);
        let copying = row("Journal", "detail", "OK", Tone::Info).copying("/data/journal.jsonl");

        assert_eq!(
            serde_json::to_value(&plain).unwrap(),
            json!({
                "label": "Inbox",
                "detail": "detail",
                "value": "0 files",
                "tone": "ok",
                "copyText": null
            })
        );
        assert_eq!(
            serde_json::to_value(&copying).unwrap(),
            json!({
                "label": "Journal",
                "detail": "detail",
                "value": "OK",
                "tone": "info",
                "copyText": "/data/journal.jsonl"
            })
        );
    }

    #[test]
    fn tones_serialize_as_lower_case_words() {
        for (tone, expected) in [
            (Tone::Ok, "ok"),
            (Tone::Warning, "warning"),
            (Tone::Error, "error"),
            (Tone::Info, "info"),
        ] {
            assert_eq!(serde_json::to_value(tone).unwrap(), json!(expected));
        }
    }

    #[test]
    fn only_the_journal_row_offers_text_to_copy() {
        let (dir, db) = open_with_accounts_and_year();
        let all = groups(&db, dir.path(), None);

        let copying: Vec<String> = all
            .iter()
            .flat_map(|g| g.rows.iter())
            .filter(|r| r.copy_text.is_some())
            .map(|r| r.label.clone())
            .collect();

        assert_eq!(copying, ["Journal"]);
    }

    #[test]
    fn unverified_values_are_summed_over_all_files() {
        let (dir, db) = rules_db();
        for year in [2026, 2027] {
            let mut rules = minimal_rules(year);
            set(&mut rules, "/items/0/verified", json!(false));
            crate::taxrules::files::create(dir.path(), year, &rules_text(&rules)).unwrap();
        }

        assert_eq!(
            row_of(&groups(&db, dir.path(), None), "Rules", "Unverified values"),
            unverified_row(2)
        );
    }
}
