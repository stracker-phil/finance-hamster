# 0040. Tax rules live in a JSON file per year; the tax year shows one copy value per Kennzahl

## Status

Accepted (2026-11-13). Hamster's own. Supersedes 0038 and the parts of 0039 named below. Follows the
mockups approved on 13 Nov 2026 (frames 5–8, 13, 23–28) and the reference files `rules.schema.json`,
`rules-2026.json` and the rules `README.md`.

## Context

0038 planned a ruleset editor in Settings and two numbers per Kennzahl (copy value and expected value). The
owner's reviewed mockups replace both: the rules are a plain JSON file per year, edited in any text editor, and
the tax year page shows only what to type into FinanzOnline.

## Decision

- **One file per tax year**, `rules/rules-<year>.json` in the data folder, validated against
  `rules.schema.json` (shipped next to it with a README). Unknown fields or rule types are errors. Settings →
  Tax Rules lists the files (Draft, Open, Closed) with Open in Editor or View, and offers Reload (validates,
  shows what changes in the open year, then applies), Import (keeps the old file as `rules-<year>.v<N>.json`),
  Duplicate for next year and Show in Finder. No in-app editing of any rule value.
- **Reload checks beyond the schema**: duplicate keys, removed or renamed keys that entries use, unknown
  sections or questions, a `reduceBy` question that isn't an amount, a missing asset class, overlapping or
  out-of-year Kilometergeld ranges, a trips item without a rate for its vehicle.
- **A closed year keeps a frozen copy** of its file; editing the file later has no effect.
- **Items** have a stable `key`, `kz`, `name`, `section`, `kind` (`documents`, `trips`, `amount`,
  `reportedByOthers`), `visible`, `evidence` (`required`, `optional`, `none`), optional `privateShare`,
  `depreciation` (asset class), `vehicle`, `estimate`, `note`, `verified`. Hidden items are `visible: false`
  in the file and appear nowhere in the UI.
- **Questions** are yearly numbers (`integer` or `amount`, e.g. `telearbeitstage`, `telearbeitspauschale`)
  answered in a "Questions for <year>" block on the tax year page. Empty means unanswered.
- **Copy value only.** Per Kennzahl: the sum of its rows. Documents count paid × work share (100 −
  `privateShare`, changeable per entry); above `gwgLimit` with `depreciation` they are spread over
  `usefulLife[assetClass]`, and bought on or after `halfYearFrom` the first and last slice count half. Trips
  count km × the Kilometergeld rate valid on the trip date; amounts count as typed; `reportedByOthers` never
  counts.
- **Estimate rules** (`requires`, `reduceBy`, `cap` with optional `carryOver`) never change the copy value.
  Applied in that order, they produce one plain note line under the Kennzahl (e.g. "Answer the question
  above.", "won't be accepted", "The tax office subtracts …"). There is no expected value or range anywhere.
- **Sections** are shown only once one has an entry (no header while only Werbungskosten has entries).
- `verified: false` values are counted on Settings → Status.
- Adding a rule type needs code; everything else is data.

- Rejected: an in-app ruleset editor (0038): a large UI for a file edited once a year.
- Rejected: an expected value beside the copy value (0038): two numbers invite copying the wrong one.
- Rejected: 0039's "document required / bank line suffices" pair: replaced by `evidence`
  `required`/`optional`/`none`; 0039's "reported by others" becomes the kind `reportedByOthers`.

## Consequences

- The schema, README and the seed `rules-2026.json` are shipped with the app and written to the rules folder
  on first launch.
- Rules (Settings → Rules), Review, documents and vendor defaults point at item keys; a key that entries use
  can never disappear from a later file.
- The tax year page holds every row kind (documents, amounts, trips, slices from earlier years) under its
  Kennzahl; no separate entries page.
