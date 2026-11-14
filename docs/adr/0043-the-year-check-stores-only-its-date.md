# 0043. The year check asks from the ruleset and stores only its date

## Status

Accepted (2026-11-14). Hamster's own.

## Context

Deductions get forgotten when nothing reminds you of them: a laptop bought in March, a conference trip, union
dues. The tax year page shows what was added, not what is missing. A reminder needs to ask, and the questions
change with the law, so they belong with the year's rules (ADR 0040), not in code.

## Decision

- **Questions come from the ruleset.** An item or question in `rules-<year>.json` with an `ask` block
  (`order`, `question`, `hint`) is asked; without one it never is. Reload refuses `ask` on a hidden item or an
  item reported by others.
- **A run is stateless.** Every run starts with all questions open; Yes opens the item's usual input (Add
  Document, the inline amount or trip row), No and Later only change the page's own list. Leaving the page
  discards the run. Entries made from a Yes are ordinary writes.
- **Only the time of the last completed run is stored** (`year_checks`, one row per year), written as one
  journaled op ("Complete Year Check <year>") and undoable like any other. Nothing per question.
- **The date decides the prompts.** Close Year asks for a run first while none completed after 31 Dec; closing
  anyway is journaled as "Close Tax Year <year> without year check". Today and the year check reminder offer it
  from 1 November until a run, and again after 31 Dec until a run after it.

- Rejected: storing answers, No's and the Later list across runs: a stale "No" from June would hide the
  laptop bought in October, and stored answers would need their own editing and undo.

## Consequences

- A ruleset without `ask` blocks gives an empty check that says so; older rules files need the blocks added.
- Number questions in the check are the year's questions; the check and the tax year page show the same
  answer.
