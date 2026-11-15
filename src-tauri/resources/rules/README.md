# Hamster rules files

One JSON file per tax year: `rules-<year>.json`, validated against `rules.schema.json`.
Location: `~/Library/Application Support/com.philippstracker.hamster/rules/`

## Lifecycle

| Year state | File | What you can do |
|---|---|---|
| Draft (next year) | editable | Edit freely. Not used yet. |
| Open | editable | Edit, then **Reload** in Settings → Tax Rules. Hamster shows what changes before applying. Bump `ruleset` (`2026 v1` → `2026 v2`). |
| Closed | frozen | Hamster keeps a copy inside the closed year. Editing the file has no effect. |

- **Import…** replaces a draft or open year's file and keeps the old one as `rules-2026.v1.json`.
- **Duplicate…** copies the open file to next year as a draft.

## How values are calculated

**Copy value** (what you type into FinanzOnline) per Kennzahl = sum of its entries:

- `documents`: paid amount × work share (100 − `privateShare`, changeable per entry).
  - Above `gwgLimit` with `depreciation`: spread over `usefulLife[assetClass]` years.
  - Bought on or after `halfYearFrom`: the first and last slice count half.
- `trips`: km × the `kilometergeld` rate valid on the trip date.
- `amount`: the number you typed.
- `reportedByOthers`: never counted, shown for information.

**Estimate rules** never change the copy value: the tax office applies them itself. They add one note under the
Kennzahl in the FinanzOnline view that works out what is left. They are applied in this order:

1. `requires`: the question is unanswered → "Enter … first." Below `min` → "won't be accepted".
2. `reduceBy`: the question's amount is subtracted. `reduceByPerUnit`: `perUnit` × the integer answer, counting
   at most `maxUnits`, is subtracted (KZ 169: €3 × home office days, at most 100).
3. `cap`: anything above `amount` is cut. With `carryOver`, the rest counts later.

Adding a new rule type needs code. Everything else is data.

## Labels and examples

`name` is the legal name FinanzOnline uses. `label` is the plain name shown while recording, and `examples` says
what belongs there; both have German versions in `de`. Without a `label` the `name` is shown.

## Retired: `ask`

Older files carry `ask` blocks from the retired year check. They still load and are otherwise ignored, except
that an `ask.hint` stands in for missing `examples`.

## Checks on Reload (beyond the schema)

Reload fails with a clear message if:

- a `key` is used twice, or a key that existing entries use was removed or renamed
- an item's `section` doesn't exist in `sections`
- a rule references an unknown `question`
- a `reduceBy` question isn't of type `amount`, or a `reduceByPerUnit` question isn't of type `integer`
- a `depreciation.assetClass` is missing from `usefulLife`
- `kilometergeld` ranges for one vehicle overlap or fall outside `year`
- a `trips` item uses a vehicle with no rate
- an `ask` sits on a hidden item or an item reported by others, or its `question` is empty

## German texts

Questions, items and sections may carry a `de` object with German versions of their texts (`label`, `help`,
`unit`; `name`, `label`, `examples`, `note`). Hamster shows them when its language is German and falls
back to the English field for any that are missing. A `de` text may not be empty: leave it out instead.

## Notes on 2025 v3 and 2026 v4

- Checked on 15 November 2026 against EStG § 16, the BMF L1 field lists and the Kilometergeldverordnung; the sources are in each file.
- The Kennzahlen and amounts are the same in both years. KZ 159 is Arbeitszimmer (hidden); it can't be filed together with KZ 158.
- KZ 158 needs 26 **Telearbeitstage** (called Homeoffice days until 2024), asked as "Home office days". Enter the full amount: the tax office counts €300 a year and the rest in later years.
- KZ 169 is reduced by €3 per Telearbeitstag (at most 100 days): the employer's Telearbeitspauschale plus the difference the tax office grants itself. Hamster works it out from the home office days.
- Kilometergeld for a car is €0.50 per km all year in both years.
- `usefulLife.furniture` is an estimate; Austria has no official table.
- From 2027 the Telearbeitspauschale ends and KZ 158 no longer needs Telearbeitstage. A 2027 file needs those changes.
