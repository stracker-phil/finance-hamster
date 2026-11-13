//! `hamster.sqlite` and the one write path (ADR 0007, ADR 0012, ADR 0015): every change is an `Op` of row
//! changes, applied by `Db::write` in one transaction with its journal rows, then mirrored to `journal.jsonl`.

pub mod change;
pub mod commands;
pub mod journal;
pub mod migrations;
mod undo;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub use change::{Change, Row};

use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;
use undo::UndoStack;

pub const FILE_NAME: &str = "hamster.sqlite";

/// One change as the owner sees it: a label for the toast, Undo and the Log, and its row changes, applied in
/// order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Op {
    pub label: String,
    pub changes: Vec<Change>,
}

impl Op {
    pub fn new(label: impl Into<String>, changes: Vec<Change>) -> Self {
        Self {
            label: label.into(),
            changes,
        }
    }

    /// Undoes the op: every change inverted, in reverse order.
    pub fn inverse(&self) -> Op {
        Op {
            label: self.label.clone(),
            changes: self.changes.iter().rev().map(Change::inverse).collect(),
        }
    }

    /// The tables the op touches, each once, for the change event.
    pub fn tables(&self) -> Vec<String> {
        let mut tables: Vec<String> = self.changes.iter().map(|c| c.table().to_string()).collect();
        tables.sort();
        tables.dedup();
        tables
    }
}

/// Where a write came from; the journal records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Ui,
    Cli,
    Import,
    /// The debug build's demo data.
    Seed,
    /// Hamster's own bookkeeping, such as a tax year started from its rules file.
    App,
    Undo,
    Redo,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Ui => "ui",
            Source::Cli => "cli",
            Source::Import => "import",
            Source::Seed => "seed",
            Source::App => "app",
            Source::Undo => "undo",
            Source::Redo => "redo",
        }
    }

    /// Changes by the owner or an import join the undo history; the rest don't.
    pub fn is_undoable(self) -> bool {
        matches!(self, Source::Ui | Source::Cli | Source::Import)
    }
}

/// What a write did: the tables to announce, and the label to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub label: String,
    pub tables: Vec<String>,
}

fn restrict_to_owner(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

pub struct Db {
    conn: Mutex<Connection>,
    journal_path: PathBuf,
    history: Mutex<UndoStack>,
}

impl Db {
    /// Opens (or creates) the file in `dir`, migrates it and appends journal rows the mirror lacks.
    pub fn open(dir: &Path) -> AppResult<Db> {
        let path = dir.join(FILE_NAME);
        let mut conn = Connection::open(&path)?;
        restrict_to_owner(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrations::migrate(&mut conn)?;
        let journal_path = dir.join(journal::FILE_NAME);
        if let Err(e) = journal::catch_up(&conn, &journal_path) {
            log::warn!("Couldn't bring journal.jsonl up to date: {e}");
        }
        Ok(Db {
            conn: Mutex::new(conn),
            journal_path,
            history: Mutex::new(UndoStack::default()),
        })
    }

    /// Hands the connection to a read; every write goes through `write`.
    pub fn read<T>(&self, read: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        read(&self.conn.lock_unpoisoned())
    }

    /// Applies `op` and its journal rows in one transaction; nothing changes when any part fails. A write from
    /// the owner or an import becomes the newest undoable change.
    pub fn write(&self, op: Op, source: Source) -> AppResult<Written> {
        let written = self.apply(&op, source)?;
        if source.is_undoable() {
            self.history.lock_unpoisoned().push(op);
        }
        Ok(written)
    }

    /// Undoes the newest change as a new, journaled op; `None` when there is nothing to undo.
    pub fn undo(&self) -> AppResult<Option<Written>> {
        let Some(op) = self.history.lock_unpoisoned().last_done().cloned() else {
            return Ok(None);
        };
        let written = self.apply(&op.inverse(), Source::Undo)?;
        self.history.lock_unpoisoned().mark_undone();
        Ok(Some(written))
    }

    /// Applies the newest undone change again; `None` when there is nothing to redo.
    pub fn redo(&self) -> AppResult<Option<Written>> {
        let Some(op) = self.history.lock_unpoisoned().last_undone().cloned() else {
            return Ok(None);
        };
        let written = self.apply(&op, Source::Redo)?;
        self.history.lock_unpoisoned().mark_redone();
        Ok(Some(written))
    }

    fn apply(&self, op: &Op, source: Source) -> AppResult<Written> {
        if op.changes.is_empty() {
            return Err(AppError::InvalidInput("Nothing to change".into()));
        }
        let mut conn = self.conn.lock_unpoisoned();
        let tx = conn.transaction()?;
        let mut fields = Vec::new();
        for change in &op.changes {
            change.apply(&tx)?;
            fields.extend(change.fields());
        }
        let header = journal::OpHeader {
            label: &op.label,
            source: source.as_str(),
            at: chrono::Utc::now().timestamp_millis(),
        };
        journal::record(&tx, &header, &fields)?;
        tx.commit()?;
        if let Err(e) = journal::catch_up(&conn, &self.journal_path) {
            log::warn!("Couldn't append to journal.jsonl; the next write tries again: {e}");
        }
        Ok(Written {
            label: op.label.clone(),
            tables: op.tables(),
        })
    }
}
