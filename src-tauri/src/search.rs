//! "Matches elsewhere" under the header's filter field: where else the typed text turns up, so a filter on one
//! page can jump to the documents of every kind or the bank lines that match it.

use serde::Serialize;
use ts_rs::TS;

use crate::accounts;
use crate::date::DateOrder;
use crate::db::Db;
use crate::documents::list;
use crate::error::AppResult;
use crate::review::title_of;
use crate::settings::NumberFormat;
use crate::statements::store as statements;
use crate::{tr, tr_plural};

/// The tabs of Dokumente.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum DocumentsTab {
    Invoices,
    Statements,
    Payslips,
    Other,
}

/// Where a match leads.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "page", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Target {
    /// Dokumente at one of its tabs.
    Documents { tab: DocumentsTab },
    #[serde(rename_all = "camelCase")]
    Document {
        document_id: String,
        year: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    Account { account_id: String },
    #[serde(rename_all = "camelCase")]
    Statement { statement_id: String },
    #[serde(rename_all = "camelCase")]
    Payslip { payslip_id: String, year: i32 },
    #[serde(rename_all = "camelCase")]
    Other { other_id: String },
}

/// One match leads to its page; several to their tab of Dokumente, counted.
fn summarized(
    matches: Vec<(String, String, Target)>,
    tab: DocumentsTab,
    count: impl Fn(usize) -> String,
) -> Option<Elsewhere> {
    match matches.len() {
        0 => None,
        1 => matches
            .into_iter()
            .next()
            .map(|(label, place, target)| Elsewhere {
                label,
                place,
                target,
            }),
        n => Some(Elsewhere {
            label: count(n),
            place: tr!("Documents", "Dokumente"),
            target: Target::Documents { tab },
        }),
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Elsewhere {
    /// `Pixelforge Ltd. invoice`, `3 documents`, `4 lines`.
    pub label: String,
    /// `Tax year 2026`, `All documents`, `Private`.
    pub place: String,
    pub target: Target,
}

/// The documents of every kind and each account's bank lines that contain `query`.
pub fn elsewhere(
    db: &Db,
    query: &str,
    format: NumberFormat,
    order: DateOrder,
) -> AppResult<Vec<Elsewhere>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let groups = list::documents(db, query, format, order)?;
    let found: Vec<_> = groups
        .iter()
        .flat_map(|g| g.documents.iter().map(move |d| (g, d)))
        .collect();
    match found.as_slice() {
        [] => {}
        [(group, doc)] => out.push(Elsewhere {
            label: doc.title.clone(),
            place: group.title.clone(),
            target: Target::Document {
                document_id: doc.id.clone(),
                year: group.year,
            },
        }),
        many => out.push(Elsewhere {
            label: {
                let count = many.len();
                tr_plural!(
                    count,
                    "{count} document",
                    "{count} documents",
                    "{count} Beleg",
                    "{count} Belege"
                )
            },
            place: tr!("All invoices", "Alle Belege"),
            target: Target::Documents {
                tab: DocumentsTab::Invoices,
            },
        }),
    }
    let statements_found = crate::statements::documents::list(db, query, format, order)?
        .into_iter()
        .flat_map(|a| {
            a.statements.into_iter().map(move |s| {
                (
                    s.title,
                    a.name.clone(),
                    Target::Statement { statement_id: s.id },
                )
            })
        })
        .collect();
    out.extend(summarized(
        statements_found,
        DocumentsTab::Statements,
        |n| {
            tr_plural!(
                n,
                "{n} statement",
                "{n} statements",
                "{n} Kontoauszug",
                "{n} Kontoauszüge"
            )
        },
    ));
    let mut payslips_found = Vec::new();
    for year in crate::payslips::view::list(db, format)? {
        for month in year.months {
            let Some(id) = month.id else { continue };
            let label = format!("{} {}", month.name, year.year);
            if label.to_lowercase().contains(&needle)
                || tr!("payslip", "lohnzettel").contains(&needle)
            {
                payslips_found.push((
                    label,
                    tr!("Payslips", "Lohnzettel"),
                    Target::Payslip {
                        payslip_id: id,
                        year: year.year,
                    },
                ));
            }
        }
    }
    out.extend(summarized(payslips_found, DocumentsTab::Payslips, |n| {
        tr_plural!(
            n,
            "{n} payslip",
            "{n} payslips",
            "{n} Lohnzettel",
            "{n} Lohnzettel"
        )
    }));
    let others_found = crate::others::view::list(db, query, order)?
        .into_iter()
        .flat_map(|g| {
            g.documents
                .into_iter()
                .map(move |d| (d.title, g.title.clone(), Target::Other { other_id: d.id }))
        })
        .collect();
    out.extend(summarized(others_found, DocumentsTab::Other, |n| {
        tr_plural!(
            n,
            "{n} other document",
            "{n} other documents",
            "{n} anderes Dokument",
            "{n} andere Dokumente"
        )
    }));
    let lines = db.read(statements::all_transactions)?;
    for account in db.read(accounts::rows)? {
        let count = lines
            .iter()
            .filter(|t| t.account_id == account.id)
            .filter(|t| {
                format!("{} {}", title_of(t), t.text)
                    .to_lowercase()
                    .contains(&needle)
            })
            .count();
        if count > 0 {
            out.push(Elsewhere {
                label: tr_plural!(
                    count,
                    "{count} line",
                    "{count} lines",
                    "{count} Buchung",
                    "{count} Buchungen"
                ),
                place: account.name.clone(),
                target: Target::Account {
                    account_id: account.id.clone(),
                },
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn search_elsewhere(
    state: tauri::State<'_, crate::state::AppState>,
    query: String,
) -> AppResult<Vec<Elsewhere>> {
    elsewhere(
        &state.db,
        &query,
        state.settings.get().number_format,
        DateOrder::system(),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::i18n::with_lang;
    use crate::settings::Language;
    use crate::year::tests::{add_document, add_statement, debit, open_year};
    use crate::{others, payslips};

    fn found(db: &Db, query: &str) -> Vec<Elsewhere> {
        elsewhere(db, query, NumberFormat::DotDecimal, DateOrder::DayFirst).unwrap()
    }

    fn noted(db: &Db, id: &str, pay_date: Option<&str>, note: &str) {
        add_document(db, id, |d| {
            d.pay_date = pay_date.map(String::from);
            d.note = note.into();
        });
    }

    #[test]
    fn an_empty_query_finds_nothing_elsewhere() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");

        for query in ["", "   "] {
            assert_eq!(found(&db, query), vec![], "{query:?}");
        }
    }

    #[test]
    fn one_matching_document_is_named_with_its_tax_year_and_opens_directly() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        noted(&db, "doc-b", Some("2026-05-01"), "Workshop");

        assert_eq!(
            found(&db, " SEMINAR "),
            vec![Elsewhere {
                label: "doc-a".into(),
                place: "Tax year 2026".into(),
                target: Target::Document {
                    document_id: "doc-a".into(),
                    year: Some(2026),
                },
            }]
        );
    }

    #[test]
    fn one_matching_undated_document_is_found_in_the_inbox() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", None, "Seminar");

        assert_eq!(
            found(&db, "seminar"),
            vec![Elsewhere {
                label: "doc-a".into(),
                place: "Inbox".into(),
                target: Target::Document {
                    document_id: "doc-a".into(),
                    year: None,
                },
            }]
        );
    }

    #[test]
    fn several_matching_documents_lead_to_the_invoices_tab() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        noted(&db, "doc-b", None, "Seminar");
        noted(&db, "doc-c", Some("2026-01-01"), "Workshop");

        assert_eq!(
            found(&db, "seminar"),
            vec![Elsewhere {
                label: "2 documents".into(),
                place: "All invoices".into(),
                target: Target::Documents {
                    tab: DocumentsTab::Invoices
                },
            }]
        );
    }

    #[test]
    fn several_matching_documents_read_in_german() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        noted(&db, "doc-b", None, "Seminar");

        let german = with_lang(Language::De, || found(&db, "seminar"));

        assert_eq!(
            german,
            vec![Elsewhere {
                label: "2 Belege".into(),
                place: "Alle Belege".into(),
                target: Target::Documents {
                    tab: DocumentsTab::Invoices
                },
            }]
        );
    }

    #[test]
    fn one_matching_statement_names_its_number_and_account_and_opens_directly() {
        let (_dir, db) = open_year();
        add_statement(&db, "statement-1", "account-1", 1, &[]);
        add_statement(&db, "statement-3", "account-1", 3, &[]);

        assert_eq!(
            found(&db, "NR. 003"),
            vec![Elsewhere {
                label: "Nr. 003/2026".into(),
                place: "Private".into(),
                target: Target::Statement {
                    statement_id: "statement-3".into()
                },
            }]
        );
    }

    #[test]
    fn a_statement_is_found_by_its_account_name() {
        let (_dir, db) = open_year();
        add_statement(&db, "statement-1", "account-1", 1, &[]);
        add_statement(&db, "statement-2", "account-2", 2, &[]);

        assert_eq!(
            found(&db, "household"),
            vec![Elsewhere {
                label: "Nr. 002/2026".into(),
                place: "Household".into(),
                target: Target::Statement {
                    statement_id: "statement-2".into()
                },
            }]
        );
    }

    #[test]
    fn several_matching_statements_lead_to_the_statements_tab_with_a_count() {
        let (_dir, db) = open_year();
        add_statement(&db, "statement-1", "account-1", 1, &[]);
        add_statement(&db, "statement-2", "account-2", 2, &[]);
        add_statement(&db, "statement-3", "account-1", 3, &[]);

        let english = found(&db, "nr.");
        let german = with_lang(Language::De, || found(&db, "nr."));

        assert_eq!(
            english,
            vec![Elsewhere {
                label: "3 statements".into(),
                place: "Documents".into(),
                target: Target::Documents {
                    tab: DocumentsTab::Statements
                },
            }]
        );
        assert_eq!(
            german,
            vec![Elsewhere {
                label: "3 Kontoauszüge".into(),
                place: "Dokumente".into(),
                target: Target::Documents {
                    tab: DocumentsTab::Statements
                },
            }]
        );
    }

    #[test]
    fn one_matching_payslip_names_its_month_and_opens_directly() {
        let (dir, db) = open_year();
        let (_, id) = payslips::edit::add(&db, dir.path(), 2026, 7, None, 1).unwrap();
        payslips::edit::add(&db, dir.path(), 2026, 8, None, 1).unwrap();

        assert_eq!(
            found(&db, "july"),
            vec![Elsewhere {
                label: "July 2026".into(),
                place: "Payslips".into(),
                target: Target::Payslip {
                    payslip_id: id,
                    year: 2026
                },
            }]
        );
    }

    #[test]
    fn several_matching_payslips_lead_to_the_payslips_tab_with_a_count() {
        let (dir, db) = open_year();
        payslips::edit::add(&db, dir.path(), 2026, 7, None, 1).unwrap();
        payslips::edit::add(&db, dir.path(), 2026, 8, None, 1).unwrap();
        payslips::edit::add(&db, dir.path(), 2025, 12, None, 1).unwrap();

        let by_year = found(&db, "2026");
        let by_word = found(&db, "payslip");
        let german = with_lang(Language::De, || found(&db, "lohnzettel"));

        let tab = Target::Documents {
            tab: DocumentsTab::Payslips,
        };
        assert_eq!(
            by_year,
            vec![Elsewhere {
                label: "2 payslips".into(),
                place: "Documents".into(),
                target: tab.clone(),
            }]
        );
        assert_eq!(by_word[0].label, "3 payslips");
        assert_eq!(by_word[0].target, tab);
        assert_eq!(
            german,
            vec![Elsewhere {
                label: "3 Lohnzettel".into(),
                place: "Dokumente".into(),
                target: tab,
            }]
        );
    }

    #[test]
    fn one_matching_other_document_names_its_year_group_and_opens_directly() {
        let (dir, db) = open_year();
        let (_, undated) =
            others::edit::add(&db, dir.path(), "Home Policy.pdf", b"%PDF a", 1).unwrap();
        let (_, dated) = others::edit::add(&db, dir.path(), "Contract.pdf", b"%PDF b", 2).unwrap();
        others::edit::edit(
            &db,
            &dated,
            &others::edit::OtherEdit::Date {
                value: "2026-03-01".into(),
            },
        )
        .unwrap();

        assert_eq!(
            found(&db, "policy"),
            vec![Elsewhere {
                label: "Home Policy".into(),
                place: "Without date".into(),
                target: Target::Other { other_id: undated },
            }]
        );
        assert_eq!(
            found(&db, "contract"),
            vec![Elsewhere {
                label: "Contract".into(),
                place: "2026".into(),
                target: Target::Other { other_id: dated },
            }]
        );
    }

    #[test]
    fn an_other_document_is_found_by_its_note() {
        let (dir, db) = open_year();
        let (_, id) = others::edit::add(&db, dir.path(), "Scan.pdf", b"%PDF a", 1).unwrap();
        others::edit::edit(
            &db,
            &id,
            &others::edit::OtherEdit::Note {
                text: "Renews in May".into(),
            },
        )
        .unwrap();

        assert_eq!(
            found(&db, "renews"),
            vec![Elsewhere {
                label: "Scan".into(),
                place: "Without date".into(),
                target: Target::Other { other_id: id },
            }]
        );
    }

    #[test]
    fn several_matching_other_documents_lead_to_the_other_tab_with_a_count() {
        let (dir, db) = open_year();
        others::edit::add(&db, dir.path(), "Policy A.pdf", b"%PDF a", 1).unwrap();
        others::edit::add(&db, dir.path(), "Policy B.pdf", b"%PDF b", 2).unwrap();

        let english = found(&db, "policy");
        let german = with_lang(Language::De, || found(&db, "policy"));

        assert_eq!(
            english,
            vec![Elsewhere {
                label: "2 other documents".into(),
                place: "Documents".into(),
                target: Target::Documents {
                    tab: DocumentsTab::Other
                },
            }]
        );
        assert_eq!(german[0].label, "2 andere Dokumente");
        assert_eq!(german[0].place, "Dokumente");
    }

    #[test]
    fn every_kind_of_document_that_matches_comes_before_the_account_lines() {
        let (dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        others::edit::add(&db, dir.path(), "Seminar notes.pdf", b"%PDF a", 1).unwrap();
        add_statement(
            &db,
            "statement-1",
            "account-1",
            1,
            &[debit("l-1", "2026-10-14", -4_690, "Lastschrift Seminar")],
        );

        let places: Vec<String> = found(&db, "seminar").into_iter().map(|e| e.place).collect();

        assert_eq!(places, ["Tax year 2026", "Without date", "Private"]);
    }

    #[test]
    fn matching_lines_are_counted_per_account_by_title_or_booking_text() {
        let (_dir, db) = open_year();
        add_statement(
            &db,
            "statement-1",
            "account-1",
            1,
            &[
                debit("l-1", "2026-10-14", -4_690, "Lastschrift SEITENWEISE"),
                debit("l-2", "2026-10-20", -1_000, "Lastschrift Seitenweise Wien"),
                debit("l-3", "2026-10-21", -500, "Lastschrift CAFE LINDE"),
            ],
        );
        let household = |id: &str, text: &str| crate::statements::store::TransactionRow {
            statement_id: "statement-2".into(),
            account_id: "account-2".into(),
            ..debit(id, "2026-10-14", -900, text)
        };
        add_statement(
            &db,
            "statement-2",
            "account-2",
            1,
            &[household("l-4", "Lastschrift SEITENWEISE")],
        );

        assert_eq!(
            found(&db, "seitenweise"),
            vec![
                Elsewhere {
                    label: "2 lines".into(),
                    place: "Private".into(),
                    target: Target::Account {
                        account_id: "account-1".into(),
                    },
                },
                Elsewhere {
                    label: "1 line".into(),
                    place: "Household".into(),
                    target: Target::Account {
                        account_id: "account-2".into(),
                    },
                },
            ]
        );
    }

    #[test]
    fn documents_come_before_account_lines_and_a_query_without_matches_finds_nothing() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        add_statement(
            &db,
            "statement-1",
            "account-1",
            1,
            &[debit(
                "l-1",
                "2026-10-14",
                -4_690,
                "Lastschrift Seminar Wien",
            )],
        );

        let places: Vec<String> = found(&db, "seminar").into_iter().map(|e| e.place).collect();

        assert_eq!(places, ["Tax year 2026", "Private"]);
        assert_eq!(found(&db, "no such text"), vec![]);
    }

    #[test]
    fn targets_travel_tagged_by_page_in_camel_case() {
        for (target, expected) in [
            (
                Target::Documents {
                    tab: DocumentsTab::Invoices,
                },
                json!({ "page": "documents", "tab": "invoices" }),
            ),
            (
                Target::Documents {
                    tab: DocumentsTab::Statements,
                },
                json!({ "page": "documents", "tab": "statements" }),
            ),
            (
                Target::Documents {
                    tab: DocumentsTab::Payslips,
                },
                json!({ "page": "documents", "tab": "payslips" }),
            ),
            (
                Target::Documents {
                    tab: DocumentsTab::Other,
                },
                json!({ "page": "documents", "tab": "other" }),
            ),
            (
                Target::Statement {
                    statement_id: "statement-1".into(),
                },
                json!({ "page": "statement", "statementId": "statement-1" }),
            ),
            (
                Target::Payslip {
                    payslip_id: "payslip-1".into(),
                    year: 2026,
                },
                json!({ "page": "payslip", "payslipId": "payslip-1", "year": 2026 }),
            ),
            (
                Target::Other {
                    other_id: "other-1".into(),
                },
                json!({ "page": "other", "otherId": "other-1" }),
            ),
            (
                Target::Document {
                    document_id: "doc-a".into(),
                    year: Some(2026),
                },
                json!({ "page": "document", "documentId": "doc-a", "year": 2026 }),
            ),
            (
                Target::Document {
                    document_id: "doc-a".into(),
                    year: None,
                },
                json!({ "page": "document", "documentId": "doc-a", "year": null }),
            ),
            (
                Target::Account {
                    account_id: "account-1".into(),
                },
                json!({ "page": "account", "accountId": "account-1" }),
            ),
        ] {
            assert_eq!(
                serde_json::to_value(&target).unwrap(),
                expected,
                "{target:?}"
            );
        }
    }
}
