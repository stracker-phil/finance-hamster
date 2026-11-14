//! The Today steps, one builder per kind of open work; `today_view` orders them.

use chrono::NaiveDate;

use super::{Step, StepTarget, last_statement_end};
use crate::accounts;
use crate::date::{self, DateOrder};
use crate::db::Db;
use crate::error::AppResult;
use crate::settings::NumberFormat;
use crate::statements::view::{AccountState, Coverage};
use crate::year::page::{AttentionKind, YearPage};

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `A`, `A and B`, `A, B and C`.
pub(super) fn joined(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
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
    Ok(Some(Step {
        key: "review".into(),
        title: plural(count as usize, "line to review", "lines to review"),
        detail: format!("about {}", plural(minutes, "minute", "minutes")),
        line: format!("From {}. Rules decided the rest.", joined(&names)),
        button: "Start Review".into(),
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
    Some(Step {
        key: "gap".into(),
        title: format!("A {} statement is missing", account.name),
        detail: "import it to close the gap".into(),
        line: note,
        button: "Import Statement…".into(),
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
    let (title, detail, line) = match last_statement_end(db, &behind.id)? {
        Some(end) => {
            let ends = date::day_month(end, order);
            (
                format!(
                    "{} statement is {}",
                    behind.name,
                    behind.state_label.to_lowercase()
                ),
                format!("last one ends {ends}"),
                format!("Its last statement ends on {ends}."),
            )
        }
        None => (
            format!("{} has no statement yet", behind.name),
            "import its first one".to_string(),
            "Import its first statement to see its lines.".to_string(),
        ),
    };
    Ok(Some(Step {
        key: "import".into(),
        title,
        detail,
        line,
        button: "Import Statement…".into(),
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
    (!missing.is_empty()).then(|| Step {
        key: "attach".into(),
        title: plural(missing.len(), "invoice to attach", "invoices to attach"),
        // An attention title reads `<payee> · <amount>`.
        detail: first_names(
            missing
                .iter()
                .map(|a| a.title.split(" · ").next().unwrap_or(&a.title).to_string()),
        ),
        line: "They count once their PDF is attached.".into(),
        button: format!("Show in Tax Year {year}"),
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
    (!open.is_empty()).then(|| Step {
        key: "questions".into(),
        title: format!("{} for Tax year {year}", plural(open.len(), "question", "questions")),
        detail: open.join(" · "),
        line: "Numbers from your year, such as the Lohnzettel. They decide what the tax office accepts.".into(),
        button: format!("Answer in Tax Year {year}"),
        target: StepTarget::Year {
            year,
            attention: None,
        },
    })
}

pub(super) fn year_check_step(
    db: &Db,
    today: NaiveDate,
    number: NumberFormat,
    order: DateOrder,
) -> AppResult<Option<Step>> {
    let Some(&year) = crate::yearcheck::due_years(db, today)?.first() else {
        return Ok(None);
    };
    let view = crate::yearcheck::view(db, year, today, number, order)?;
    let count = view.questions.len();
    let run = view
        .last_run
        .map_or("Not run yet".to_string(), |d| format!("Last run {d}"));
    Ok(Some(Step {
        key: "check".into(),
        title: format!("Year check for {year}"),
        detail: format!(
            "{run} · {}",
            plural(count, "quick question", "quick questions")
        ),
        line: "Things you might forget to claim. Most are a quick No.".into(),
        button: "Start Year Check".into(),
        target: StepTarget::YearCheck { year },
    }))
}

pub(super) fn draft_step(page: &YearPage) -> Option<Step> {
    let year = page.year;
    let drafts: Vec<_> = page
        .attention
        .iter()
        .filter(|a| a.kind == AttentionKind::Draft)
        .collect();
    let first = drafts.first()?;
    let single = drafts.len() == 1;
    Some(Step {
        key: "draft".into(),
        title: plural(drafts.len(), "draft to finish", "drafts to finish"),
        detail: first_names(drafts.iter().map(|d| d.title.clone())),
        line: if single {
            "Finish it to lock its values for the tax year."
        } else {
            "Finish them to lock their values for the tax year."
        }
        .into(),
        button: if drafts.len() == 1 {
            "Open Draft"
        } else {
            "Show Drafts"
        }
        .into(),
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
