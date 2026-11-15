# 0048. Closed years are frozen in the write path and show what was filed

## Status

Accepted (2026-11-15). Hamster's own.

## Context

Frozen history is the third principle, but only document edits checked whether their year was closed. A rule
saved, a line decided in Review, a statement imported, a line linked or a document excluded could all change
what a closed year counts, and the year page recomputes its totals from the live data, so the copy values
would no longer be the ones filed. Some changes reach a closed year without touching a dated row at all: a
subscription's Werbungskosten item, a rules mapping.

## Decision

- `Db::apply` runs `db/frozen.rs` for every change inside the write's transaction and refuses any change to a
  row that counts in a closed year: documents (not excluded) by payment date, links by their line's and their
  document's year, ANV decisions (and subscription decisions whose subscription has an item) by their line's
  date, entries and answers by year. Undo and redo go through the same check. Closing and reopening touch
  only tables it doesn't look at.
- The bulk re-deciders (rules, subscription matching, statement import) leave lines in closed years alone
  instead of failing the whole save.
- A closed year's page shows the newest snapshot's Kennzahl values as the copy values; when the live data
  gives another value, an attention row "KZ … changed since closing" names both.
- Rejected: checks in each feature (the gap this record closes); refusing every change to subscriptions or
  rules that ever touched a closed year (would freeze them forever); rendering closed years only from the
  snapshot (loses the rows and documents behind the numbers).

## Consequences

- A late statement for a closed year is stored, but its lines stay undecided until the year is reopened, or
  are ignored.
- Every new table whose rows count in a year needs its line in `frozen::years_of`.
