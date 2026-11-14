# Hamster design language

The family's shared design language (from the `app-blueprint` catalog; full token sets and CSS rules live in
Taskboard's `docs/ux.md` and `src/styles/`) plus Hamster's own window. The main window follows the approved
mockup "Hamster – Simplified Concept" (`docs/plan/mockups/`, ADR 0024); Settings and behavior follow
the family language.

## Hamster's window

One main window (1440×900 by default, min 1100×680) and the Settings window. No calendar, no global date,
no context filter: time appears only as tax years and statement coverage.

### Sidebar (236px, on `--window-bg`)
Top to bottom: the traffic-light strip (drags the window) · Today · Subscriptions · Statements (count badge =
lines waiting for review) · Documents (all years) · section **Tax years**, one row per year (Open, or a lock
when closed) · the **doc zone** pinned at the bottom (drop target plus one Choose File… button; it highlights
while a PDF is dragged anywhere over the window) · footer with the sidebar toggle and Settings. Rows 34px, 8px radius, 14px text;
the selected row is an accent wash with accent text.

### Header (58px, same on every page)
Title (19px bold) · one page-specific control (for example the By Kennzahl / By month switch) · the filter
field (200px, ⌘F, Esc clears; filters the current list as you type; when the text also matches elsewhere a
small menu lists those matches plus Search everywhere ⌘↩; on pages without a list it reads Search everywhere;
on Documents it and the big search field are one search)
· the **+ New** menu: Import Statement… ⌘I, Add Invoice PDF… ⌘O, New Subscription ⌘N.

### Pages
| Page | Shows |
|---|---|
| Today | one headline next step with one primary button (review → attach invoices → import overdue statement → finish drafts), other open items as quiet rows, the next 5 charges, the current tax year's total and progress, one line on statements; the hamster with "You're up to date" when nothing is open |
| Review | one undecided line at a time: "What is this line?" ANV (A) · Subscription (S) · Ignore (I) picks (the suggestion starts picked), only the pick's detail, "Also decide every line containing TERM this way", one sentence of what will happen beside the confirm button (↩); Skip (J), Previous, progress bar; the Plan changed? card; a done state with counts |
| Subscriptions | grouped by next charge (next 7 days · next 30 days · later, Ended collapsed); header: active count and cost per month; a detail panel with a 12-month plan timeline |
| Statements | one coverage bar per account over 12 months (imported, partly, missing, before the first statement) and a status tag per account (Up to date, N to review, Statement missing, N days behind) |
| Account | the account's lines grouped by statement, with a note where a statement is missing between two groups |
| Tax year | hero with the copy total, ruleset version and progress; **Questions for <year>** (number fields from the rules file; empty = unanswered); Needs attention (missing invoices, drafts, undated scans); By Kennzahl: one copy value per Kennzahl with a copy button, its rows below (documents, amounts, trips, slices with a "From 2025" pill; row amounts only when a Kennzahl has several rows), one note line from estimate rules, "+ Add" (Document, Amount or Trip) in an open year, trips added inline (date, destination and purpose, km); By month: Every month, Whole year (amounts and earlier slices), then months with trips in theirs; … menu: Log, Archive, Export Summary PDF, Open Ruleset JSON; Close Year… / Open Snapshot PDF |
| Document | the edit form (Payment → Classification with the item picker and work share → Result → Notes → More), a GWG line or the Depreciation block (cost, useful life, slices per year, frozen ones marked), PDF preview, History column; breadcrumb "Documents /" or the tax year it was opened from |
| Documents | sidebar entry; a large "Is it already here?" field, documents by tax year plus Inbox |
| Log, Archive | a tax year's journal entries; its excluded documents and the delete flow |

### Hamster-specific rules
- List rows are 44px with 14px text (the mockup's body size); amounts right-aligned in tabular figures,
  credits in the success hue, foreign amounts as muted meta beside the EUR amount.
- The detail panel (document preview, subscription details) is 520px on the right, over the list.
- A finished document shows a blue banner and read-only fields; Unlock… asks first. A closed year shows a lock.
- The ANV amount is computed text with its formula as a hint, never an input. Work share is pre-filled from
  the item's private share (169: 60 %) and editable per document.
- Each Kennzahl shows one amount, the copy value. Rules the tax office applies only add one plain note line;
  the €132 Werbungskostenpauschale note appears only when the total is below it.
- Copy buttons put the plain number on the clipboard (`1228,40`) and confirm with a toast.
- The mascot from the app icon appears only in empty and success states.
- A missing or changed PDF shows a red banner at the top of the content until resolved.

## Principles
- Calm, light and native: it looks like the Mac apps beside it. The window opens at once; nothing animates
  for show.
- Fully keyboard-usable: every action is a command with a menu item, a palette entry and a rebindable
  shortcut (ADR 0033).
- One word for each thing in every label, menu item, palette title, toast and the docs (ANV, Document,
  Statement, Line, Subscription, Plan, Pool, Kennzahl, Tax year); accessible names follow the visible labels.
- Demo data in every picture.
- All configuration lives in Settings.

## Stable controls
- A click never changes what is under the cursor; a toggle toggles back; only closing an overlay or
  navigating away may replace what was clicked.
- Lists don't re-sort while in use; a decided or removed row keeps its place (grayed) until the page is left.
- Controls keep their size: state shows by color and outline, never an added icon or text; buttons that only
  sometimes apply keep their slot, invisible.
- Nothing jumps, things move: fade in, glide (`useGlide`, `useListGlide`), animate height (`useHeightGlide`).
- Folding into "…" follows the window width only.
- Before finishing a UI change: click every new or changed control twice and run `scripts/dev/e2e.sh`.

## Text and controls never wrap
Buttons, tabs, chips, column headers and table cells stay on one line; long text truncates with an ellipsis
and shows its full text in a tooltip. When space runs out the flexible element shrinks, never a button. Only
descriptions, bodies and empty-state text wrap.

## Type and metrics
- System font (SF Pro), SF Mono only for machine text (IBANs, file names, hashes), no bundled fonts, tabular
  figures for numbers.
- Scale 14 (list rows, body) / 13 (controls, forms) / 11–12 (meta) / 16 (section) / 19 (page title).
- 8px grid, 24px content margins. Radii: 6 (chips, tiles), 7 (pills, small buttons, menu rows), 8 (sidebar
  rows, buttons), 10–12 (groups, panels, sheets, popovers), 16 (palette), 50% (dots).
- Traffic lights native inside the sidebar's top strip (`trafficLightPosition`); every empty strip drags the
  window.

## Tokens
Palettes define the same token set (ADR 0025); groups: Surfaces (`--window-bg`, `--panel-bg`,
`--fill-subtle`, `--fill-hover`, `--segmented-bg`, `--scrim`), Text (`--text`, `--text-secondary`,
`--text-tertiary`, `--text-muted`, `--text-faint`, `--danger-text`, `--text-on-accent`, `--link`), Lines
(`--border`, `--border-strong`, `--hairline`), Accent (`--accent`, `--selection-*`, `--button-primary-*`,
`--focus-ring`), Messages (`--banner-{info,warn,error}-*`), Shadows (`--shadow-{panel,popover,palette,…}`),
Hover (`--hover-bg`, `--row-hover-bg`), Settings (`--settings-*`), Tooltip (`--tooltip-bg`), plus Hamster's
tag hues (ANV, Subscription, Transfer, Missing, Draft, Finished, Closed) and the coverage bar states. Contrast:
4.5:1 body, 3:1 meta and large text. Accent: System or one of nine hues (ADR 0026).

## Motion
`--motion-fast` 120ms (hover, press), `--motion-base` 150ms (fades, chevrons, toggles), `--motion-slow` 200ms
(expand/collapse, popover entry), `--motion-highlight` (accent wash), `--ease-out`, `--ease-in-out`; no literal
durations elsewhere; `prefers-reduced-motion` makes movement instant. A page switch is instant; Review's next
card fades in.

## Scrollbars
None are styled; every scroll box has `.scroller` and starts below its title row; menus and modals portal to
`<body>` (ADR 0027).

## Menus (ADR 0030)
12px radius, `--border`, `--shadow-popover`, 6px padding; rows 30px, 7px radius, 13px text; optional icon,
hint or key (muted), check or ›; caps caption; hairline between groups; destructive rows in `--danger-text`.
Opens at the mouse or under its button, flips, then shifts inside the window; closes on Esc, outside press,
scroll, blur, resize or a choice; ↑↓, Home/End, ↩/Space, letters jump, →/← submenus. Pickers with long lists
(bank line, vendor, subscription) get a search field.

## Tooltips (ADR 0029)
One host per window; 450ms, instant within 600ms, on focus; below or above, ≤420px; ends with the current
binding.

## Overlays inside a window
Toasts bottom right, self-dismissing (success 2.5s, info 4s, error 8s, with an action such as Undo 10s);
errors read "Something went wrong" with Details. Sheets over `--scrim` (Import, Add document, Edit/New
subscription, Change Plan, confirms) take Esc first, trap Tab and return focus. A click beside a sheet
does nothing (only the palette closes on it), and the scrim over the toolbar still drags the window. A sheet
grows to the window's height; a long one keeps its head and buttons and scrolls only its body. A sheet opened
from a bank line shows that line at its top. Review never decides on one click: A, S, I or a click picks, the
confirm button (or Return) beside one sentence of what will happen decides, and Undo follows. Anything that loses the
owner's data asks first in a `ConfirmSheet` that names what goes and what stays; deleting a document also
needs a typed reason and an "I understand this can't be undone" checkbox. The content never blanks. Empty
states say why in few words.

## Command palette (ADR 0031)
⌘K; panel 640px, 16px radius, 110px from the top, over `--scrim`; lists commands plus jump targets (tax
years, accounts, subscriptions, documents); ↑↓ wrap, ↩ runs, Esc closes.

## Settings (ADR 0034)
Own window in System Settings layout: 190px page list with drag strip and search; page title in the strip,
an optional one-line intro; groups of rows (label and description left, control right). The current page is
filled with the accent. Pages, Finance: Rules, Vendors, Accounts, Tax Rules; App: Reminders, Privacy & Data,
Appearance, Shortcuts, Status, About. The search finds every row.

## Keys
App actions are Rust commands; page keys (Review's A, S, I, J) live in the page registry and never fire in
text fields. Menus in Mac order; View holds window appearance only; Help holds Open Log Folder and Show Data
Folder. One binding per command.
