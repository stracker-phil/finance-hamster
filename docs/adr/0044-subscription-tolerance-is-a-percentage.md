# 0044. A subscription's amount tolerance is a percentage

## Status

Accepted (2026-11-14). Hamster's own. Supersedes the tolerance clause of 0041 ("a fixed amount in the plan's
currency"); the rest of 0041 stands.

## Context

Insurers, utilities and software vendors raise prices by a share of the price, not by a fixed sum. A fixed
tolerance such as ± 1.00 fits a €9.99 plan and a €480 plan very differently, and the owner can't tell which
value catches next year's increase.

## Decision

- **The tolerance is a whole percent of the plan in effect on the charge's date**: exact (0 %), a percentage
  below 100, or any amount. A charge fits when its difference from the plan is at most that share of the plan.
- The sheet offers Exact, ± 1, 2, 5 and 10 % and Any amount, and shows a stored value outside that list as is.
- Migration 14 adds `subscriptions.tolerance_percent`, converted from the old cents against the latest plan and
  rounded up. `tolerance_cents` stays unused, so the journal's older ops still apply and undo.

## Consequences

- A charge beyond the tolerance still asks "Plan changed?" in Review (ADR 0041).
- Undoing the deletion of a subscription saved before 0.25 brings it back with "any amount", since its old
  insert carries no percentage.
