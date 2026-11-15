# 0056. Dokumente holds every PDF; one detail layout; a real tax-year sidebar

## Status

Accepted (2026-11-15). Hamster's own. Changes where ADR 0055 put payslips: the panel beside the tax year
becomes a summary, and payslips are listed and edited in Dokumente.

## Context

Each kind of PDF lived somewhere else:
- **Bank statements** disappeared after import; none could be opened.
- **Invoices** had their own page and drop zone.
- **Payslips** were listed and added in a panel inside the tax year. "+ New" didn't offer them, and a payslip
  couldn't be moved to another month or year.
- **Every preview** was a small box beside a wide form, too small to read.

The owner looks for any PDF in one place first, and wanted the tax year to show a summary of the year, not
collect more data.

## Decision

- **Dokumente** (formerly "Belege") lists every PDF Hamster keeps, under four tabs: Invoices · Statements ·
  Payslips · Other.
  - The page's search filters the open tab.
  - "+ New" adds each kind.
  - The sheet for a dropped PDF offers Invoice · Payslip · Other, and statements are recognized as before.
  - Search ("Elsewhere") finds every kind.
- **The page that showed statement coverage, Review and account lines is named "Konten" (Accounts).**
  - Its statement groups link to the statement's PDF.
  - A statement links back to its lines.
- **One detail layout** (`DetailLayout`) for every kind:
  - the PDF over most of the page;
  - the kind's details in a sidebar on the right: the form for invoices, payslips and other documents, facts
    only for a statement.
  - An invoice's History moves into that sidebar.
- **Payslips** are listed per year with all twelve months, so a missing month shows. The form gains the tax
  year and month, and a move is one undoable change that the frozen check guards on both sides.
- **Other documents** (`others/`, table `others`) keep any PDF with a title, an optional date and a note.
  Nothing reads them, and they count in no tax year.
- **The tax year's sidebar** is a full-height column beside the page, like a macOS inspector. It's toggled at
  the trailing edge of the toolbar (and ⌥⌘L), and shows the year's payslip sums only, with a link to
  Dokumente → Payslips.

- Rejected: a page per kind in the sidebar. It keeps the fragmentation this decision removes.
- Rejected: keeping the preview small and opening the PDF in a separate viewer. Reading and typing the values
  side by side is the point of the page.

## Consequences

- Statement files keep their paths; only how they're found changes.
- A new kind of document adds a tab, a detail page in `DetailLayout` and a "+ New" entry.
