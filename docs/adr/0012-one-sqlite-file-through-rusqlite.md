# 0012. One SQLite file through rusqlite, PDFs as plain files beside it

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `storage`); reference implementation: Taskboard ADR 0019.

## Context

Hamster keeps a few hundred rows a year (statements, lines, documents, subscriptions, rulesets) that need joins and totals, plus the original PDFs. Everything is the owner's data; nothing is a disposable cache. The spec proposed a data folder chosen at first launch (`~/Hamster`); the owner chose the family's location instead.

## Decision

- **Location:** everything lives in the config dir, `~/Library/Application Support/com.philippstracker.hamster/` (0700, files 0600): `hamster.sqlite`, `journal.jsonl`, `docs/<year>/`, `docs/inbox/`, `statements/<year>/`, `snapshots/`. Settings → Privacy & Data shows the folder with Show in Finder. The debug app has its own folder.
- **rusqlite** with bundled SQLite: WAL, foreign keys on, one writer and readers on blocking threads, numbered embedded migrations under `PRAGMA user_version`, a file from a newer Hamster refused. One file: there is no cache to split off. Money is integer cents plus a currency code; foreign-currency lines also store the original amount, rate and fee.
- **Integrity:** every PDF has a SHA-256 hash in the database; a launch check compares files and hashes and reports, never repairs.
- Settings, keybindings and UI state stay JSON (`json-stores-repair-on-load`).

- Rejected: A data folder chosen by the owner (`~/Hamster`): the owner preferred the family location.
- Rejected: sqlx: no async pool or read-only side process needed.
- Rejected: JSON files: the data needs joins and totals.

## Consequences

- The first-run step "choose data folder" is dropped.
- Time Machine covers the folder like any Application Support data.
