# 0049. Refunds net against charges; one line pays one document

## Status

Accepted (2026-11-15). Hamster's own.

## Context

Bank lines carry their sign (a charge is negative). The amount paid for a document and the value of a line
decided ANV were summed as absolute values, so a refund linked to its invoice, or decided ANV like the charge it
reverses, raised the Werbungskosten instead of lowering them. A refund with the plan's amount also matched as a
subscription's charge for the period. And one line could be linked to two documents, counting its amount twice.

## Decision

- What was paid is the net outflow (`TransactionRow::outflow_cents`, `statements::store::net_outflow`): charges
  positive, refunds negative. The work share keeps the sign (`documents::amounts::anv_cents`), so a refund in a
  later year lowers that year. A depreciable document whose refunds cover its cost has nothing to depreciate.
- Only debits are subscription charges (`subscriptions::matcher::charged_in`).
- A line pays at most one counting document (`documents::refuse_paid_elsewhere`, checked on Link, on Add with a
  line and on Restore). An invoice split across two documents is entered with typed amounts.
- Rejected: dropping credits from every sum (a refund would never lower the total); splitting a line across
  documents (no case needs it yet, and every sum would need the split).

## Consequences

- A year's total can go down when a refund arrives; Kennzahl values may be negative if refunds outweigh charges
  in a year.
