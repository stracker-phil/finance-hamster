//! The Today steps, one builder per kind of open work; `today_view` orders them.

use super::{Step, StepTarget, last_statement_end};
use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::statements::view::{AccountState, Coverage};
use crate::year::page::{AttentionKind, YearPage};
use crate::{tr, tr_plural};

/// `A`, `A and B`, `A, B and C`.
pub(super) fn joined(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => {
            let rest = rest.join(", ");
            tr!("{rest} and {last}", "{rest} und {last}")
        }
    }
}

pub(super) fn review_step(db: &Db) -> AppResult<Option<Step>> {
    let waiting = db.read(crate::review::undecided_by_account)?;
    let count: u32 = waiting.values().sum();
    if count == 0 {
        return Ok(None);
    }
    let names: Vec<String> = db
        .read(accounts::rows)?
        .into_iter()
        .filter(|a| waiting.get(&a.id).is_some_and(|n| *n > 0))
        .map(|a| a.name)
        .collect();
    let minutes = (count as usize).div_ceil(6).max(1);
    let from = joined(&names);
    Ok(Some(Step {
        key: "review".into(),
        title: tr_plural!(
            count,
            "{count} line to review",
            "{count} lines to review",
            "{count} Buchung zu prüfen",
            "{count} Buchungen zu prüfen"
        ),
        detail: tr_plural!(
            minutes,
            "about {minutes} minute",
            "about {minutes} minutes",
            "etwa {minutes} Minute",
            "etwa {minutes} Minuten"
        ),
        line: tr!(
            "From {from}. Rules decided the rest.",
            "Von {from}. Den Rest haben Regeln entschieden."
        ),
        button: tr!("Start Review", "Prüfen starten"),
        target: StepTarget::Review,
    }))
}

/// An account whose statement chain has a hole: the step names the missing statement.
fn gap_step(view: &Coverage) -> Option<Step> {
    let account = view
        .accounts
        .iter()
        .find(|a| a.state == AccountState::Gap)?;
    let note = account.note.clone()?;
    let name = &account.name;
    Some(Step {
        key: "gap".into(),
        title: tr!(
            "A {name} statement is missing",
            "Ein Kontoauszug von {name} fehlt"
        ),
        detail: tr!(
            "import it to close the gap",
            "importiere ihn, um die Lücke zu schließen"
        ),
        line: note,
        button: tr!("Import Statement…", "Kontoauszug importieren …"),
        target: StepTarget::Import,
    })
}

pub(super) fn import_step(db: &Db, view: &Coverage, order: DateOrder) -> AppResult<Option<Step>> {
    let Some(behind) = view
        .accounts
        .iter()
        .find(|a| a.state == AccountState::Behind)
    else {
        return Ok(gap_step(view));
    };
    let name = &behind.name;
    let (title, detail, line) = match last_statement_end(db, &behind.id)? {
        Some(end) => {
            let ends = date::day_month(end, order);
            let state = &behind.state_label;
            let lower = state.to_lowercase();
            (
                tr!(
                    "{name} statement is {lower}",
                    "Kontoauszug von {name}: {state}"
                ),
                tr!("last one ends {ends}", "der letzte endet am {ends}"),
                tr!(
                    "Its last statement ends on {ends}.",
                    "Sein letzter Kontoauszug endet am {ends}."
                ),
            )
        }
        None => (
            tr!(
                "{name} has no statement yet",
                "{name} hat noch keinen Kontoauszug"
            ),
            tr!("import its first one", "importiere den ersten"),
            tr!(
                "Import its first statement to see its lines.",
                "Importiere den ersten Kontoauszug, um seine Buchungen zu sehen."
            ),
        ),
    };
    Ok(Some(Step {
        key: "import".into(),
        title,
        detail,
        line,
        button: tr!("Import Statement…", "Kontoauszug importieren …"),
        target: StepTarget::Import,
    }))
}

/// The first two distinct names, in order.
fn first_names(names: impl Iterator<Item = String>) -> String {
    let mut distinct: Vec<String> = Vec::new();
    for name in names {
        if !distinct.contains(&name) {
            distinct.push(name);
        }
        if distinct.len() == 2 {
            break;
        }
    }
    joined(&distinct)
}

pub(super) fn attach_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let missing: Vec<_> = page
        .attention
        .iter()
        .filter(|a| a.kind == AttentionKind::MissingInvoice)
        .collect();
    let count = missing.len();
    (!missing.is_empty()).then(|| Step {
        key: "attach".into(),
        title: tr_plural!(
            count,
            "{count} invoice to attach",
            "{count} invoices to attach",
            "{count} Rechnung anzuhängen",
            "{count} Rechnungen anzuhängen"
        ),
        // An attention title reads `<payee> · <amount>`.
        detail: first_names(
            missing
                .iter()
                .map(|a| a.title.split(" · ").next().unwrap_or(&a.title).to_string()),
        ),
        line: tr!(
            "They count once their PDF is attached.",
            "Sie zählen, sobald ihr PDF angehängt ist."
        ),
        button: tr!("Show in Tax Year {year}", "Im Steuerjahr {year} zeigen"),
        target: StepTarget::Year {
            year,
            attention: Some(AttentionKind::MissingInvoice),
        },
    })
}

pub(super) fn questions_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let open: Vec<&str> = page
        .questions
        .iter()
        .filter(|q| !q.answered)
        .map(|q| q.label.as_str())
        .collect();
    let count = open.len();
    (!open.is_empty()).then(|| Step {
        key: "questions".into(),
        title: tr_plural!(
            count,
            "{count} question for Tax year {year}",
            "{count} questions for Tax year {year}",
            "{count} Frage zum Steuerjahr {year}",
            "{count} Fragen zum Steuerjahr {year}"
        ),
        detail: open.join(" · "),
        line: tr!(
            "Numbers from your payslip (Lohnzettel). Hamster works out what they change.",
            "Zahlen von deinem Lohnzettel. Hamster rechnet aus, was sie ändern."
        ),
        button: tr!(
            "Answer in Tax Year {year}",
            "Im Steuerjahr {year} beantworten"
        ),
        target: StepTarget::Year {
            year,
            attention: None,
        },
    })
}

pub(super) fn draft_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let drafts: Vec<_> = page
        .attention
        .iter()
        .filter(|a| a.kind == AttentionKind::Draft)
        .collect();
    let first = drafts.first()?;
    let count = drafts.len();
    let single = count == 1;
    Some(Step {
        key: "draft".into(),
        title: tr_plural!(
            count,
            "{count} draft to finish",
            "{count} drafts to finish",
            "{count} Entwurf fertigzumachen",
            "{count} Entwürfe fertigzumachen"
        ),
        detail: first_names(drafts.iter().map(|d| d.title.clone())),
        line: if single {
            tr!(
                "Finish it to lock its values for the tax year.",
                "Schließe ihn ab, um seine Werte für das Steuerjahr festzuschreiben."
            )
        } else {
            tr!(
                "Finish them to lock their values for the tax year.",
                "Schließe sie ab, um ihre Werte für das Steuerjahr festzuschreiben."
            )
        },
        button: if single {
            tr!("Open Draft", "Entwurf öffnen")
        } else {
            tr!("Show Drafts", "Entwürfe zeigen")
        },
        target: match (&first.document_id, single) {
            (Some(id), true) => StepTarget::Document {
                id: id.clone(),
                year,
            },
            _ => StepTarget::Year {
                year,
                attention: Some(AttentionKind::Draft),
            },
        },
    })
}
