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

**Estimate rules** never change the copy value. They only add a note under the Kennzahl. They are applied in this order:

1. `requires`: the question is unanswered → "Answer the question above." Below `min` → "won't be accepted".
2. `reduceBy`: the question's answer is subtracted.
3. `cap`: anything above `amount` is cut. With `carryOver`, the rest counts later.

Adding a new rule type needs code. Everything else is data.

## Checks on Reload (beyond the schema)

Reload fails with a clear message if:

- a `key` is used twice, or a key that existing entries use was removed or renamed
- an item's `section` doesn't exist in `sections`
- a rule references an unknown `question`
- a `reduceBy` question isn't of type `amount`
- a `depreciation.assetClass` is missing from `usefulLife`
- `kilometergeld` ranges for one vehicle overlap or fall outside `year`
- a `trips` item uses a vehicle with no rate

## Notes on 2026 v1

- KZ 159 is Arbeitszimmer (hidden). The furniture rule needs 26 **Telearbeitstage**; "Homeoffice days" was renamed in 2025.
- The Telearbeitspauschale reduces **KZ 169** (digital equipment), not 158.
- Kilometergeld and the KZ 158 cap/carry-over are `verified: false`. Check them before closing the year.
