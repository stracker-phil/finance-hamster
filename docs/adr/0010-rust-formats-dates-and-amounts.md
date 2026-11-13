# 0010. Rust formats dates and amounts with the macOS region

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `rust-formats-time-and-dates`); reference implementation: TimeTracker ADRs 0011 and 0025.

## Context

Hamster shows dates and money everywhere and copies Kennzahl totals into FinanzOnline as plain numbers (`1228,40`).

## Decision

- Timestamps (journal, imports) are UTC milliseconds (`i64`). Bank and payment dates are calendar dates stored as `YYYY-MM-DD` text, never timestamps, so a tax year never shifts with the time zone.
- Rust formats dates through `NSDateFormatter` with the macOS region, and amounts with the number format from Settings → Appearance (1,234.56 or 1.234,56). The copy value of a Kennzahl total is a plain number with a decimal comma and no grouping. The page may use `Intl` for relative times only.

- Rejected: A JS date library.
- Rejected: Hand-written formatting in the page.
- Rejected: Dates as timestamps.

## Consequences

- Formatting is tested in Rust.
- The page receives labels, not raw dates, for display.
