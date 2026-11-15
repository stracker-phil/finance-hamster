# 0053. Hide unused categories per year; FinanzOnline lists every Kennzahl

## Status

Accepted (2026-11-15). Hamster's own. Extends 0052.

## Context

The Categories view lists every visible item of the year's rules. Some never apply to the owner, union dues
for example, so they take space in every year. The FinanzOnline view resembles the L1 form and listed only
Kennzahlen with an amount, while the form shows every field, many of them 0,00.

## Decision

- **Hiding is the owner's choice per year**, stored in `hidden_items` (year, item key), apart from the rules: it
  doesn't make the year's rules the owner's version, and new built-in rules don't undo it. Each switch is one
  undoable op. The switches sit in the year's sheet in Settings → Tax Rules and behind "Choose Categories…"
  in the tax year's … menu.
- **Only the Categories view reads it.** A hidden category with an entry still shows; the others wait behind
  "N categories hidden · Show". Pickers (Add Document, Review, the Account page) list every item, so nothing
  ever gets stuck.
- **A new year copies the previous year's choice** when it starts.
- **FinanzOnline lists every Kennzahl** in number order, with 0,00 where nothing counts, like the form.

- Rejected: hiding through `visible: false` in the rules. That hides an item everywhere, including
  FinanzOnline and the pickers, and turns the year's rules into the owner's version.

## Consequences

- The table is a view preference: closing a year doesn't freeze it, and changing it never changes a value.
