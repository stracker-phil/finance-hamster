# 0055. Payslips as their own record; home office days from the latest one

## Status

Accepted (2026-11-15). Hamster's own.

## Context

The bank statement shows only the salary's payout. The payslip carries what the tax year needs or what the
owner wants to see: gross, social insurance, income tax, the employer's tax-free home office allowance, and
the home office days (Telearbeitstage). The owner typed the home office days into the tax year by hand.

Payslips print the home office days under their annual sums (L16), so each one carries the year so far, not
the month.

## Decision

- **A record of its own** (`payslips/`, table `payslips`), not a kind of document. A payslip has no vendor,
  item, work share or depreciation, and folding it into `documents/` would add a condition to each of those.
  One payslip per year and month. Every value is optional, so it can be typed without its PDF and filled bit
  by bit. Each change is one undoable op. A closed year freezes its payslips (`db/frozen.rs`).
- **The PDF** is copied to `docs/<year>/payslips/<yyyy-mm>.pdf`, from the payslip panel or from the sheet
  for a dropped PDF, which offers Invoice or Payslip and guesses the month from a "<month> <year>" at the
  top.
- **The home office days of the tax year are the latest payslip's**, never a sum of the months. They answer
  the year's `telearbeitstage` question. "Your year" then shows them read-only, with the month they come
  from. A typed answer counts only while no payslip prints the days.
- **A panel beside the tax year page** lists the twelve months in their order and the year's sums. It opens
  and closes on request (View menu, ⌥⌘L, or the button in the year's header), and the choice is kept in
  `settings.json`.
- Union dues aren't a payslip field. The owner pays them privately; if an employer withholds them, they
  are already counted in payroll and must not be entered in KZ 717 again.

- Rejected: payslips as documents with a kind column, because of the conditions above.
- Rejected: summing the months' home office days. Payslips print the year so far, so a sum counts them many
  times over.

## Consequences

- Nothing here changes a copy value. Payslips only answer a question and give the owner the year's overview.
- Reading the numbers from the PDF (a parser for the payroll layout), linking the payout to the salary line,
  and using the employer's allowance in the KZ 169 note are possible next steps.
