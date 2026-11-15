# 0050. German as a second UI language

## Status

Accepted (2026-11-15). Hamster's own; the family's other apps are English only.

## Context

Hamster's content is German: bank statement terms (Buchungstag, Valuta) and the tax return's vocabulary
(Werbungskosten, Kennzahl, Telearbeitstage) appear on every page. Wrapped in an English UI, each screen
mixes two languages. The owner wants a fully German UI as an option, with English kept as it is.

## Decision

- Settings → Appearance → Language: English (default) or Deutsch (`settings::Language`). German is de-DE and
  addresses the owner as du; bank and FinanzOnline terms stay as those sources write them.
- Each text is written in both languages where it is built, not in a key table:
  - Rust: `tr!("Undo", "Widerrufen")` (formats like `format!`), `tr_plural!`, and `i18n::Text` for
    compile-time tables such as the command catalog. A missing German text fails to compile.
  - React: `src/i18n/en.ts` and `de.ts` (typed as the English catalog, so a missing key fails tsc), read
    with `useT()` in components and `t()` elsewhere.
- Rust reads the language from `i18n::lang()`: a process-wide value set at launch and on change, with a
  thread-local override (`i18n::with_lang`) so tests in parallel never see each other's choice.
- A switch takes effect at once: `update_settings` rebuilds the menu bar, renames the Settings window and
  announces new command titles; `settings_changed` makes every query stale, so Rust-formatted views refetch.
- Dates use German month and weekday names, always day first. The number format stays its own setting.
- Op labels are written in the language active when the op runs; older journal entries keep their text.
- The CLI and the logs stay English: they are a technical interface.
- Rejected: a key table with lookups (keys and texts drift apart, and a missing text shows up only at run
  time); following the macOS language (the owner wants to choose); translating only the frontend (half the
  visible text is built in Rust).

## Consequences

- Every new visible text needs both languages; the type systems enforce it on each side.
- The log and the journal may hold labels in both languages after a switch.
- Tests assert English by default; German is tested where a text is built, with `with_lang` in Rust and the
  language store in React.
