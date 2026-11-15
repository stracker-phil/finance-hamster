//! The answers to a tax year's questions: one row per year and question, a count or cents for an amount.

use std::collections::HashMap;

use serde::Serialize;

use crate::db::{Change, Db, Op, Source, Written};
use crate::error::{AppError, AppResult};
use crate::money::parse_cents;
use crate::taxrules::model::{Question, QuestionType, Ruleset};

pub const ANSWERS: &str = "answers";

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AnswerRow {
    /// `2026:telearbeitstage`.
    pub id: String,
    pub year: i32,
    pub question: String,
    /// A count, or cents for an amount question.
    pub value: i64,
    pub answered_at: i64,
}

/// The year's answers by question id.
pub fn answers(db: &Db, year: i32) -> AppResult<HashMap<String, i64>> {
    db.read(|conn| {
        let mut query = conn.prepare(&format!(
            "SELECT question, value FROM {ANSWERS} WHERE year = ?"
        ))?;
        let rows = query.query_map([year], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    })
}

fn row(db: &Db, year: i32, question: &str) -> AppResult<Option<AnswerRow>> {
    db.read(|conn| {
        let mut query = conn.prepare(&format!(
            "SELECT id, year, question, value, answered_at FROM {ANSWERS} WHERE year = ? AND question = ?"
        ))?;
        let mut rows = query.query_map(rusqlite::params![year, question], |row| {
            Ok(AnswerRow {
                id: row.get(0)?,
                year: row.get(1)?,
                question: row.get(2)?,
                value: row.get(3)?,
                answered_at: row.get(4)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    })
}

/// The stored value of an answer: a count, or cents without sign.
fn parse(question: &Question, text: &str) -> AppResult<i64> {
    let (value, message) = match question.kind {
        QuestionType::Integer => (
            text.trim().parse::<i64>().ok().filter(|n| *n >= 0),
            crate::tr!("A whole number", "Eine ganze Zahl"),
        ),
        QuestionType::Amount => (
            parse_cents(text).map(i64::abs),
            crate::tr!("This isn't an amount", "Das ist kein Betrag"),
        ),
    };
    value.ok_or_else(|| AppError::InvalidField {
        field: format!("answer.{}", question.id),
        message,
    })
}

/// The change that makes the stored answer say `text`; None when it already does.
fn change_for(
    db: &Db,
    question: &Question,
    year: i32,
    text: &str,
    now: i64,
) -> AppResult<Option<Change>> {
    let before = row(db, year, &question.id)?;
    if text.trim().is_empty() {
        return before.map(|old| Change::delete(ANSWERS, &old)).transpose();
    }
    let value = parse(question, text)?;
    let after = AnswerRow {
        id: format!("{year}:{}", question.id),
        year,
        question: question.id.clone(),
        value,
        answered_at: now,
    };
    Ok(Some(match &before {
        Some(old) if old.value == value => return Ok(None),
        Some(old) => Change::update(ANSWERS, old, &after)?,
        None => Change::insert(ANSWERS, &after)?,
    }))
}

/// Answers a question (blank text takes the answer back); None when nothing changes.
pub fn answer(
    db: &Db,
    ruleset: &Ruleset,
    year: i32,
    question_id: &str,
    text: &str,
    now: i64,
    source: Source,
) -> AppResult<Option<Written>> {
    let question = ruleset
        .questions
        .iter()
        .find(|q| q.id == question_id)
        .ok_or_else(|| {
            AppError::NotFound(crate::tr!(
                "No question {question_id} in {year}",
                "Keine Frage {question_id} in {year}"
            ))
        })?;
    let Some(change) = change_for(db, question, year, text, now)? else {
        return Ok(None);
    };
    db.write(
        Op::new(
            crate::tr!("Answer {}", "Antwort: {}", question.display_label()),
            vec![change],
        ),
        source,
    )
    .map(Some)
}
