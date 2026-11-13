//! The undo and redo history of one app run (ADR 0015): ops, newest last, bounded.

use std::collections::VecDeque;

use super::Op;

const LIMIT: usize = 100;

#[derive(Debug, Default)]
pub struct UndoStack {
    done: VecDeque<Op>,
    undone: Vec<Op>,
}

impl UndoStack {
    /// A new change: undoable, and nothing is left to redo. The oldest goes past `LIMIT`.
    pub fn push(&mut self, op: Op) {
        self.undone.clear();
        self.done.push_back(op);
        if self.done.len() > LIMIT {
            self.done.pop_front();
        }
    }

    pub fn last_done(&self) -> Option<&Op> {
        self.done.back()
    }

    pub fn last_undone(&self) -> Option<&Op> {
        self.undone.last()
    }

    /// Moves the last change to the redo list, once its inverse was applied.
    pub fn mark_undone(&mut self) {
        if let Some(op) = self.done.pop_back() {
            self.undone.push(op);
        }
    }

    /// Moves the last undone change back, once it was applied again.
    pub fn mark_redone(&mut self) {
        if let Some(op) = self.undone.pop() {
            self.done.push_back(op);
        }
    }
}
