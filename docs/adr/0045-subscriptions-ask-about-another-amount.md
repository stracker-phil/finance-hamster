# 0045. Subscriptions ask about another amount and take one charge per period

## Status

Accepted (2026-11-14). Hamster's own. Supersedes 0044 and the tolerance clause of 0041; the rest of 0041 stands.

## Context

A tolerance has to be chosen when the subscription is set up, before anyone knows how its price will move:
streaming services raise by a euro or two, insurers by a share, a savings transfer never changes. Any
tolerance also books a different amount silently, so the plan and the cost per month drift from reality. And a
second charge from the same service a day after the first is rarely the subscription again.

## Decision

- **Only the plan's exact amount is booked on its own.** A charge whose text matches at another amount waits in
  Review as "Is this <subscription>?": Yes, new price (Change Plan from that charge, so later charges book
  again), Yes, this once (a one-off linked by hand), or No (the line is asked like any other). Nothing is
  booked at another amount without the owner saying so.
- **Exact amount only** is the one switch: such a subscription never asks; another amount is simply not it.
- **One charge per period.** A subscription takes no line closer than 20 days (monthly), 60 (quarterly) or 240
  (yearly) to a charge it already has, booked or linked by hand. Lines are settled oldest first, exact amounts
  before questions, so an odd amount can't take the real charge's slot.
- Migration 15 adds `subscriptions.exact_only` (0 for every existing subscription). The tolerance columns
  stay unused for the journal's older ops.

- Rejected: tolerances in euros or percent (0041, 0044): a number to guess upfront that books silently.
- Rejected: a learned tolerance from past charges: the same silent booking, just harder to explain.

## Consequences

- A subscription whose amount varies every month asks every month; Yes, this once is one key.
- Subscriptions saved with a tolerance before 0.26 now ask about any amount but their plan's.
- "No" isn't stored: a skipped line asks again next time.
