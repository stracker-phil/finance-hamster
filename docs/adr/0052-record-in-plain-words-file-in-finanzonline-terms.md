# 0052. Record in plain words, file in FinanzOnline terms; no year check

## Status

Accepted (2026-11-15). Hamster's own. Supersedes 0043 (the year check). Changes how 0040's one copy value per
Kennzahl is shown, not how it is worked out.

## Context

The tax year page led with the Kennzahl numbers and legal names ("KZ 724 Sonstige Werbungskosten"), asked
rules questions in legal terms, and one question made the owner do the arithmetic ("€3 × Telearbeitstage, at
most 100 days"). The year check asked about every item again, in other words, but led to the same "+ Add" as
the page itself. Recording an expense needed knowledge of the L1 form, although the form only matters once a
year, when filing.

## Decision

- **Items have a plain label and examples** in the rules (`label`, `examples`, both with German versions).
  `name` stays the legal name.
- **The tax year page has three views.** Categories is the default for an open year: each item by its label
  and examples, with its entries, total and "+ Add", and no Kennzahl or rules text. Months shows the same
  entries by month. FinanzOnline is the default for a closed year: every Kennzahl with something to enter, in
  number order, with its legal name, the copy value, the item's note, and one note line worked out from the
  estimate rules.
- **Hamster does the arithmetic.** The owner enters only what they know, such as the home office days.
  Estimate rules turn that into an amount in the FinanzOnline view ("About €928.40 counts: the tax office
  subtracts €300.00 (100 days × €3.00)"). A new rule type, `reduceByPerUnit`, replaces the question that asked
  for the product. The copy value stays the full amount, because the tax office applies these rules itself.
- **No year check.** The Categories view does its job: every category is always listed with its examples, and
  an empty one is visibly empty. The check page, its Today step, its reminder and the Close Year prompt are
  gone. A December reminder ("Tax year 2026 ends soon") points to the year page instead and uses the same
  setting. `ask` blocks in older rules still load and are ignored, except that an `ask.hint` stands in for
  missing `examples`. The `year_checks` table stays, so the journal's older ops still replay.

- Rejected: keeping the year check with different wording. It would still repeat the page, and its stored
  date only served its own prompts.

## Consequences

- Recording needs no tax knowledge; filing reads the FinanzOnline view top to bottom.
- Every new item in a rules file needs a `label` (and German label) and should have `examples`.
