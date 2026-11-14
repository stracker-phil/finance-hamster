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

## Year check

The year check asks about every item and question that has an `ask` block, lowest `order` first (equal orders
keep the file's order). Without `ask`, it is never asked.

```json
"ask": { "order": 20, "question": "Did you travel for work?", "hint": "Conferences, client visits" }
```

- On an item, `question` is a yes/no question; Yes opens the item's input (Add document, a trip, an amount).
- On a question, `question` says what to type; the answer is the same one the tax year page shows.
- Not allowed on hidden items or items reported by others.
- Hamster stores only the date of the last completed run, nothing per question. Close Year asks for a run
  after 31 Dec.

## Checks on Reload (beyond the schema)

Reload fails with a clear message if:

- a `key` is used twice, or a key that existing entries use was removed or renamed
- an item's `section` doesn't exist in `sections`
- a rule references an unknown `question`
- a `reduceBy` question isn't of type `amount`
- a `depreciation.assetClass` is missing from `usefulLife`
- `kilometergeld` ranges for one vehicle overlap or fall outside `year`
- a `trips` item uses a vehicle with no rate
- an `ask` sits on a hidden item or an item reported by others, or its `question` is empty

## Notes on 2026 v1

- KZ 159 is Arbeitszimmer (hidden). The furniture rule needs 26 **Telearbeitstage**; "Homeoffice days" was renamed in 2025.
- The Telearbeitspauschale reduces **KZ 169** (digital equipment), not 158.
- Kilometergeld and the KZ 158 cap/carry-over are `verified: false`. Check them before closing the year.
