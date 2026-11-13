# 0038. Tax rules are versioned data per year, split into copy values and preview

## Status

Accepted (2026-11-13). Superseded by 0040 (2026-11-13). Hamster's own. Research behind it: `docs/plan/research-anv.md` (not in the repo).

## Context

Between 2016 and 2026 the Werbungskosten rules changed almost every year: new Kennzahlen (169 Digitale
Arbeitsmittel and 158 Ergonomisches Mobiliar in 2021), items moving between them, caps (€300 for 158),
conditions that come and go (26 Homeoffice days, gone from 2027), allowances that offset costs
(Homeoffice-/Telearbeitspauschale 2021–2026), the GWG limit (€400 → €800 in 2020 → €1,000 in 2023) and
Kilometergeld (changed on 1 July 2025). Hamster must prepare the whole Werbungskosten part of the L1 and give
the numbers to type into FinanzOnline, while some rules are applied by the tax office, not by the filer.

## Decision

- **Rulesets are data**, one per tax year, versioned ("2026 v1"), started by duplicating the previous year,
  editable while the year is open, frozen when it closes (Settings → Tax Rules). No rule value is a constant in
  code; code knows only the kinds below. Seed values are flagged unverified until the owner confirms them.
- **Items** are the L1 lines a ruleset offers, each with a stable key that survives years (rules, vendor
  defaults and documents point at the key) and per-year name, Kennzahl, note and visibility (Show/Hide, e.g.
  Pendlerpauschale for someone who walks). Item kinds:
  - **documents**: the total of the year's documents in this item (719, 720, 722, 724, 169, 158, 723, 300 …);
  - **trips**: a trip log priced with rates that are valid for date ranges (721 Kilometergeld);
  - **entered value**: a number the owner types for the year, optionally with a PDF as evidence (718/916 from
    the BMF Pendlerrechner, 717);
  - **count**: a yearly number other rules use (Homeoffice days from the Lohnzettel).
- **Item rules** each say where they apply:
  - *in the copy value* (the filer applies it): default private share (169: 40%, pre-fills Share %);
    depreciation (below);
  - *preview only* (the tax office applies it): annual cap (158: €300), yearly condition (158: ≥26 Homeoffice
    days until 2026), yearly offset (tax-free Telearbeitspauschale received, until 2026), the €132
    Werbungskostenpauschale.
  The tax year page shows each Kennzahl's **copy value** (what to type into FinanzOnline, copied as a plain
  number) and, separately and labelled as an estimate, the **expected value** after the preview-only rules.
- **Depreciation (AfA)**: the ruleset holds the GWG limit, the half-year cut-off (purchase after 30 June
  → half a year) and a default useful life per item. A document above the purchase year's GWG limit gets a
  schedule computed with the purchase year's ruleset; each tax year counts its slice; slices are stored, so a
  closed year's slice never changes and later rulesets never recompute it.
- Hamster stops at Werbungskosten amounts: no income tax calculation (spec, out of scope).

- Rejected: Kennzahlen and limits as code constants: every year would need a release.
- Rejected: one formula for everything: the split between what the filer enters and what the office applies
  would be lost, and the copy values would be wrong.
- Rejected: counting a depreciated item fully in its payment year: wrong totals whenever hardware is bought.

## Consequences

- A year's ruleset fully explains its numbers; the summary PDF and the snapshot name the ruleset version.
- The tax year needs a page for the non-document items before closing (not in the mockup).
- Every new kind of rule is code; every new value, Kennzahl or limit is data.
