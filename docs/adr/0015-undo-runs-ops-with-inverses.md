# 0015. Undo runs ops with inverses

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `undo`); reference implementation: TimeTracker ADR 0010.

## Context

Review asks for an Undo toast (⌘Z), and every change is on record in the journal.

## Decision

- Every write is an `Op` with a computed inverse, applied in one transaction through `Db::write`. A bounded, non-persisted undo/redo stack lives in `AppState`; every write path (UI, CLI) goes through ops. Undo applies the inverse as a new op, so the journal gains entries and never loses one.
- Deleting a document's file from the Archive has no inverse and is not undoable; its confirm sheet says so.

- Rejected: Plan, review, apply: no remote writes to review.
- Rejected: No undo: Review needs it.

## Consequences

- ⌘Z works the same for every change.
- Each new op needs its inverse and a test that applying both is a no-op.
