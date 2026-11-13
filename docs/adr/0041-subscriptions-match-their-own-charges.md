# 0041. Subscriptions match their own charges, by plan period

## Status

Accepted (2026-11-13). Hamster's own. Follows the approved mockups (frames 9, 15, 16, 20, 24).

## Context

A subscription has a history of plans (name, amount, currency, interval, start, end). Its charges should link
automatically when a statement is imported, a price change should surface instead of silently linking or
silently dropping, and old charges must stay with the plan they were paid under after a plan change.

## Decision

- **A subscription carries its own match**: bank text (contained, case-insensitive) and an amount tolerance
  (exact, a fixed amount in the plan's currency, or any amount). It is not a row in the Rules list.
- **The amount is checked against the plan in effect on the line's date** (the plan covering it, else the last
  one started before it, else the first). A foreign-currency plan compares the bank line's original amount
  in that currency. Ended subscriptions match nothing after their end date.
- **Matching runs with the rules**: `rules::apply` asks the rules first (first match wins), then the
  subscriptions for lines no rule decides. Such a decision stores the subscription's id in `rule_id`, so it is
  not a hand decision and Re-run keeps it current. Every subscription write re-matches all lines in the same
  op, against the subscriptions as they will be (`apply::changes_with`), so undo restores both.
- **Plan changed?** A line whose text matches but whose amount doesn't stays undecided and Review offers
  Update Plan, One-off (a hand decision linking it) or New Subscription.
- **Change Plan** ends the current plan the day before the new start; **Edit** fixes the current plan in place;
  **End** sets an end date and nothing is deleted; **Delete** (Edit sheet) removes the subscription, its
  plans and the hand decisions linking it, so its charges return to Review.
- **ANV subscriptions** name an item; each charge then counts as an ANV payment on that item on the tax year
  page, needing its invoice like any other ANV line.

## Consequences

- One matching engine decides every automatic line; a subscription's own text never shows in Settings → Rules.
- A plan change never rewrites history: charges before the new start stay linked to the old plan.
