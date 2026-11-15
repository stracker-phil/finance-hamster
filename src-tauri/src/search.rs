//! "Matches elsewhere" under the header's filter field: where else the typed text turns up, so a filter on one
//! page can jump to the documents or bank lines that match it.

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

/// Where a match leads.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[serde(tag = "page", rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub enum Target {
    Documents,
    #[serde(rename_all = "camelCase")]
    Document {
        document_id: String,
        year: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    Account {
        account_id: String,
    },
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

/// The documents and each account's bank lines that contain `query`.
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
            place: tr!("All documents", "Alle Belege"),
            target: Target::Documents,
        }),
    }
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
pub fn search_elsewhere(
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
    use crate::year::tests::{add_document, add_statement, debit, open_year};

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
    fn several_matching_documents_lead_to_the_documents_page() {
        let (_dir, db) = open_year();
        noted(&db, "doc-a", Some("2026-10-14"), "Seminar");
        noted(&db, "doc-b", None, "Seminar");
        noted(&db, "doc-c", Some("2026-01-01"), "Workshop");

        assert_eq!(
            found(&db, "seminar"),
            vec![Elsewhere {
                label: "2 documents".into(),
                place: "All documents".into(),
                target: Target::Documents,
            }]
        );
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
            (Target::Documents, json!({ "page": "documents" })),
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
