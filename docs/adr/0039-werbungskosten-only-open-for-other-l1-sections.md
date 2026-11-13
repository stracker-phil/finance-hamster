# 0039. Werbungskosten only, open for other L1 sections

## Status

Accepted (2026-11-13). Hamster's own; extends 0038. Evidence rule and "reported by others" superseded by 0040 (2026-11-13); the scope stands.

## Context

The owner wants Hamster to prepare the whole ANV, but tracks nothing outside the Werbungskosten today:
Sonderausgaben such as donations and Kirchenbeitrag are mostly reported to the tax office by the recipients,
and there are no Außergewöhnliche Belastungen or Absetzbeträge to record. Many of these would show up as bank
lines anyway, so adding them later should be configuration, not a new feature.

## Decision

- Hamster ships with the Werbungskosten part of the L1 only. Sonderausgaben, Außergewöhnliche Belastungen
  and Absetzbeträge are not seeded, shown or planned before 1.0.
- The model stays open for them: every ruleset item (0038) belongs to a **section** (Werbungskosten today;
  more sections are ruleset data), and the tax year page groups items by section.
- Each item says what **evidence** makes a payment count: *document required* (every Werbungskosten item
  today, as in the spec) or *bank line suffices*. With the second, a bank line tagged to the item (in Review,
  or by a rule) counts directly, so a future Kirchenbeitrag item merges its tagged lines into the result.
- Each item may be marked **reported by others** (for example donations): its total is shown for checking
  and has no copy value.
- Rules, Review and the ANV entries page address items by key, never a fixed list of Werbungskosten pools.

- Rejected: modelling the other sections now: nothing to track, and it would add pages before 1.0.
- Rejected: a Werbungskosten-only data model: each later section would need a migration and new code paths.

## Consequences

- Adding a section later means a ruleset edit, plus UI only where a new item kind appears.
- Tests cover an item with *bank line suffices* and one *reported by others*, even though no seed uses them.
