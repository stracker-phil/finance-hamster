# 0007. One write path, mirrored to an append-only journal

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `one-write-path`); reference implementation: Taskboard ADR 0019 (`Db::write`), TimeTracker ADR 0010 (ops).

## Context

Hamster's second principle is that nothing disappears by accident and every change is on record. The spec asks for an append-only `journal.jsonl` that feeds the Log page and document History and stays readable without the app.

## Decision

- Every change to Hamster's data is an op applied by `Db::write` in one SQLite transaction. The same transaction inserts one row per changed field into the `journal` table (timestamp, entity, id, field, old, new, source). After the commit the new rows are appended to `journal.jsonl` and synced; on launch, rows missing from the file are appended again, so the file never lags and is never rewritten.
- File moves of draft documents (rename to `<yyyy-mm-dd>-<vendor>.pdf`, move into the year folder) are part of the op: the file is renamed atomically before the commit and renamed back if the transaction fails.
- After the commit `Db::write` emits the events the op names (`statements_changed`, `documents_changed`, …). No other code writes the database, the journal or the data folder.

- Rejected: Writing the journal file first and the database second: a crash leaves a journal line for a change that never happened.
- Rejected: The journal as the only store (event sourcing): every read would replay it.

## Consequences

- Log and History read the `journal` table; the file is the readable copy.
- A failed append shows a banner and retries; it never blocks the write.
