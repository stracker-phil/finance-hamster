# 0059. Accounts show one calendar year, stepped in the toolbar

## Status

Accepted (2026-11-16). Hamster's own.

## Context

The Accounts page showed the last twelve months up to today, named by month only. It never said which year a
month belonged to, and once January came, most of the year still being wrapped up slid out of view with no
way back.

## Decision

- Accounts shows January to December of one calendar year; months after today read as still ahead.
- A `‹ 2026 ›` stepper beside the title (`YearStepper`) moves to the previous or next year that has a tax year
  or any data: statements, transactions, documents with a pay date, entries, payslips or other documents with
  a date (`statements::years::shown`). An arrow with no such year is disabled and keeps its place.
- It opens on this year; the year chosen last is kept for the session.
- Account states (up to date, behind, a statement missing) and the lines to review stay about today, whichever
  year shows, so nothing pending hides behind the stepper.

- Rejected: a segmented control with one segment per open tax year: years with statements but no tax year
  would have no segment, and the control would grow every year.

## Consequences

- An unfinished year stays reachable for as long as its data exists.
- The same stepper can serve other pages that show a year.
