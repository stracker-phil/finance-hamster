# Architecture

Hamster is a private, offline Dock app that imports bank statement PDFs, sorts their lines with rules and a
short review, files invoice PDFs as ANV documents per tax year, and tracks subscriptions with a plan history.
Its moving parts are the statement parsers, the rule engine, the document store (SQLite plus PDF files), the
append-only journal, the tax-year rulesets and the reminder scheduler.

## Design rules

These rules hold everywhere in the code. Keep them when you change things.

1. **Rust owns the logic; React only renders** (ADR 0001). Validation, ordering,
   filtering, grouping, dates and the CLI are Rust. The page asks, renders and forwards intent.
2. **IPC only in `api.ts` files** (ADR 0002). Only `src/api/index.ts` (and
   `src/<module>/api.ts`) imports `@tauri-apps/*`. Everything else imports the `api` object, which is also
   what UI tests mock. `src/test/boundary.test.ts` enforces it.
3. **Errors are data** (ADR 0003). Every command returns `Result<T, AppError>`, serialized as
   `{ kind, message, field? }`. The UI branches on `kind` only.
4. **Types are generated** (ADR 0004). Every type crossing IPC derives `ts_rs::TS` and
   exports to `src/api/types.gen.ts` (camelCase). Never edit the generated file.
5. **Panics unwind** (ADR 0005). A bug fails one command, not the app. Locks are taken with the
   poison-tolerant helpers in `lock.rs`.
6. **No color literal outside `src/styles/palettes/`** (ADR 0025). A Vitest scan enforces it.
7. **The look is native and light** (ADR 0024, [ux.md](ux.md)).
8. **One write path into the data** (ADR 0007). Every change goes through the data layer's one
   write function, which validates, saves and announces it as an event. No other code writes the data.
9. **Commands live in Rust** (ADR 0008). Menus, shortcuts, the palette, the CLI and links run
   commands of one bus by id.
10. **Small feature modules** (ADR 0006): by feature, pure cores, ports only where a
    swap is real, files under ~400 lines.
11. **Debug is a separate app on demo data** (ADR 0022); nothing in the repo carries
    real data ([`demo-data.md`](../.claude/rules/demo-data.md)).
12. **Local first** (ADR 0016): no network code at all; Hamster keeps no secrets.

## Process model

- One process holds the window(s); the same binary is the CLI. The first process binds `hamster.sock` and
  holds `hamster.lock` in the config dir; a second one hands its request over and exits
  (ADR 0009).
- Windows are declared `visible: false` and shown by Rust, so `--hidden` never flashes them. Closing hides;
  only Quit ends the app.
- Launch requests are queued in Rust and pulled by the page (`take_launch_events`).
- `hamster://` links arrive through `tauri-plugin-deep-link` and run an allowlist of commands.
- At launch a supervised task checks every referenced PDF (exists, SHA-256 matches) and appends journal rows
  missing from `journal.jsonl`; findings become a banner, nothing is repaired silently.
- Reminders are (re)scheduled with the macOS notification center at launch and after every change that moves
  a date; they fire while the app is not running (ADR 0035).

## Data

Everything lives in the config dir (ADR 0012), 0700 with files 0600:

```
~/Library/Application Support/com.philippstracker.hamster/
  hamster.sqlite      accounts, statements, transactions, rules, decisions, vendors, documents,
                      document_links, deletions, answers, entries, subscriptions, plans, tax_years,
                      snapshots, journal
  journal.jsonl       append-only copy of the journal table, one line per changed field, never rewritten
  docs/<year>/        <yyyy-mm-dd>-<vendor>.pdf by payment date; -2, -3 on collision; evidence/ for amounts
  docs/inbox/         undated documents under their original name
  statements/<year>/  <account>-<nr>.pdf, year of the statement's end date
  snapshots/          <year>-summary.pdf, written when a year is closed; -2, -3 after a reopen
  rules/              rules-<year>.json per tax year, rules.schema.json, README.md (ADR 0040)
  reminders.json      the keys of the reminders already told (ADR 0042)
  keybindings.json    the owner's own shortcuts: { command id: accelerator | null } (`commands::keys`)
  settings.json, ui-state.json
```

- **SQLite** through rusqlite (bundled), WAL, one writer and readers on blocking threads, numbered embedded
  migrations under `PRAGMA user_version`; a newer file is refused.
- **Write path** (ADR 0007, ADR 0015): every change is an `Op` with an inverse, applied by `Db::write` in one
  transaction that also writes its `journal` rows and any atomic file move of a draft document. After the
  commit the rows are appended to `journal.jsonl` and the op's events are emitted.
- **Money** is integer cents plus a currency code; foreign-currency lines also keep the original amount,
  rate and fee. Bank dates are `YYYY-MM-DD` text.
- **Statement lines** are parsed once at import, stored with the parser layout version and never re-parsed.
- **Tax rules** are files edited outside the app (ADR 0040). Each tax year's row keeps the rules text it
  applies: a year starts from its valid file once the calendar reaches it, an edited file takes effect only
  through Reload (checked, previewed, journaled, undoable), and closing the year freezes that text. Files are
  checked at launch and whenever Settings → Tax Rules shows them.
- **JSON stores** (`JsonStore`: atomic, 0600, defaults for missing files, unparsable files moved aside) hold
  settings, keybindings and UI state.

## Repository map

Rust (`src-tauri/src/`):

| Path | Holds |
|---|---|
| `lib.rs` | identity consts (`IDENTIFIER`, `APP_NAME`, `LINK_SCHEME`), `config_dir`, `run` (CLI or app), the config-contract tests |
| `app.rs` | the Tauri builder: plugins, window events, setup, run loop, logging |
| `cli/` | the `hamster` command line (`parse`) and how replies print (`reply`); `commands` answers without the app |
| `commands/` | the bus: `COMMANDS` (id, title, accelerator, who runs it), `run`, `list`, `shortcut_label`; page commands go to the main window as `run_command`; `keys.rs` (the owner's shortcuts in `keybindings.json`, conflicts refused; the menu bar is rebuilt and `commands_changed` sent) |
| `menu/` | the menu bar: macOS's standard items around `MENUS` (command ids); a menu item's id is its command |
| `db/` | `hamster.sqlite`: `migrations.rs` (numbered, `user_version`), `change.rs` (row changes over JSON rows with inverses and field-level journal entries), `journal.rs` (the `journal` table and its `journal.jsonl` mirror, caught up at launch), `undo.rs`, `Db::write`/`undo`/`redo`, `commands.rs` (`undo`, `redo`, the `data_changed` event) |
| `money.rs`, `date.rs` | cents plus currency, amount labels by the number format, the FinanzOnline copy value, typed amounts; `YYYY-MM-DD` dates and their labels in the region's order |
| `accounts/` | bank accounts: IBAN check and mask, the name other statements print for an account (`statement_name`, since real statements often print no counterparty IBAN), `add`/`edit`/`list` and their commands; changing an own account or its statement name decides the lines again in the same op |
| `taxyears/` | the `tax_years` rows (state, applied rules text), `start_from_rules` at launch, `list_tax_years`, `refuse_closed` (no document change with a payment date in a closed year) |
| `taxrules/` | the rules files: `model.rs` (typed file, unknown fields fail), `check.rs` (schema constraints and Reload checks), `files.rs` (the `rules/` folder, seed, backups, next year's draft), `diff.rs` (the Reload preview lines), `overview`/`preview`/`apply`/`import`/`duplicate`, `commands.rs` (Settings → Tax Rules) |
| `../resources/rules/` | the schema, README and seed `rules-2026.json` the app writes to `rules/` |
| `statements/` | bank statements: `parse/` (one parser per bank layout in `PARSERS`, reading the text layer into `ParsedStatement`: `george.rs` the Sparkasse statement as Digitales Banking prints it, with no printed period, so it runs from the first to the last value date; `booking.rs` a booking's card, FX and counterparty details), `check.rs` (balance and line-sum checks naming the page, the chain to the previous statement), `file_hash`; `store.rs` (the `statements` and `transactions` rows), `import.rs` (`prepare` reads and checks a PDF and refuses duplicates and unknown IBANs, `ImportQueue` holds it by token, `commit` copies it to `statements/<year>/` and stores its lines as one undoable op, tagging transfers between own accounts and letting the rules decide), `view.rs` (the Statements coverage with each account's lines to review; an account is behind after the import reminder's days, shows a gap when its chain breaks, and months before its first statement are neutral), `lines.rs` (the Account page lines with how each was decided, and which statements are missing between two groups), `commands.rs` (choose, commit, cancel, coverage, lines; `dropped` routes PDFs dropped on the main window: statements to `import_ready`, any other PDF to `add_ready`) |
| `platform/` | macOS frameworks: `pdf/` (PDFKit: `Pdf` text with the layout kept via `layout.rs`, which keeps touching glyphs in one word whatever the page's fonts, pages as PNG; `write.rs`, a small PDF writer), `pdf_view.rs` (the native `PDFView` over the page placeholder, links swallowed) |
| `rules/` | line rules: `store.rs` (`rules` and `decisions` rows; a decision shares its line's id, no rule = by hand), `engine.rs` (`matches`, `decide`: first matching rule wins; `pays_own`: a line paying another own account, by IBAN or statement name), `apply.rs` (the decision changes a set of rules makes, then the subscriptions' own matching; never touching hand decisions), `mod.rs` (`save`, `delete`, `rerun`, built-ins for transfers and bank fees, items from the newest open year), `view.rs` (Settings → Rules in words), `preview.rs` (what the sheet's rule would match, before saving), `commands.rs` |
| `documents/` | invoices: `store.rs` (`documents`, `vendors`, `document_links`), `add.rs` (`prepare` refuses a hash already added, `preview` for the Add sheet, `commit` copies to `docs/inbox/` and stores the draft, a new vendor and the link as one op; `AddQueue`), `guess.rs` (vendor, total, invoice number and date from the text), `payments.rs` (the bank lines that likely paid an invoice), `amounts.rs` (work share, ANV amount, GWG hint), `files.rs` (`relocate`: drafts move to `docs/<year>/<date>-<vendor>.pdf` after every write, as a non-undoable op of their own), `view.rs` (the Document page, payment first; `line_options` for the bank-line picker), `edit.rs` (`DocumentEdit`: one field of a draft per op, a new vendor created on save), `list.rs` (the Documents page by tax year plus Inbox, with "Is it already here?"), `vendors.rs` (Settings → Vendors: default item, rename, merge, delete when unused), `depreciation.rs` (the AfA schedule above the purchase year's GWG limit: useful life from the asset class or the document's own, half-year rule, yearly slices), `lifecycle.rs` (Finish, Unlock, Exclude, Restore; Delete from the Archive after `archive_days`, removing the file and keeping a `deletions` row; not undoable), `history.rs` (a document's History and a year's Log from the journal), `commands.rs` (add, edit, view, list, vendors; `offer` for dropped PDFs that aren't statements) |
| `year/` | the tax year page: `page.rs` (the page view; filtered by the header's text), `facts.rs` (what counts in the year: documents by their payments or depreciation slices, needs attention), `rows.rs` (amounts, trips and ANV lines without a document), `summary.rs` (questions, progress, Kennzahlen, months with Whole year, the Pauschale note), `notes.rs` (the one note line from the estimate rules), `answers.rs` (the `answers` table), `close.rs` (Close: drafts and undated documents block, missing invoices and questions warn; a `snapshots` row with the summary PDF in `snapshots/`; Reopen keeps it; the op says "without year check" when none ran after 31 Dec), `report.rs` (the summary PDF), `entries.rs` (the `entries` table: amounts as typed with an evidence PDF under `docs/<year>/evidence/`, trips valued at the rules' Kilometergeld rate on their date), `commands.rs` |
| `search.rs` | "matches elsewhere": documents and bank lines containing the filter text, for the filter field's menu and the palette |
| `integrity/` | `verify` (every document and statement PDF against its stored hash, at launch in the background and on Verify Now; the result in `LastCheck` with the `files_checked` event), `status.rs` (Settings → Status: statement chains and balances, checksums, journal against the database, inbox, rules files), `commands.rs` (also `settings_show` to open Settings at a row) |
| `reminders/` | `due_today` (renewals, price changes, overdue imports, gaps, filing, the year check), `tell` (each once, keys in `reminders.json` via `sent.rs`); the app runs it at launch and every half hour; `notify::Notifier::deliver` shows them in release builds (ADR 0042) |
| `yearcheck/` | the year check (ADR 0043): the questions from the ruleset's `ask` blocks in order, the `year_checks` table (the time of the last completed run per year, one undoable op), `close_needs_run` for Close Year, `due_on`/`due_years` for Today and the reminder, `commands.rs` (view, complete); the run itself lives in the page |
| `today/` | the Today page: `today_view` (the next step by priority: review, invoices to attach (opening the tax year on just those), an overdue statement or else a missing one, the year check (from 1 Nov, in place of the questions), questions, drafts (one opens, several open the tax year on drafts); the next five charges, each opening its subscription, the newest open tax year, how far the statements reach), `steps.rs` (one builder per kind of step), `commands.rs` |
| `subscriptions/` | subscriptions (ADR 0041): `store.rs` (`subscriptions` and `plans` rows), `matcher.rs` (`Matchers`: bank text plus a percent tolerance of the plan on the line's date, ADR 0044; `plan_changed`), `schedule.rs` (next charge, cost per month), `edit.rs` (New/Edit form, Change Plan, End/Resume, Delete; each re-matches the lines in the same op), `view.rs` (the list by next charge, the live match count, a New form prefilled from a line), `detail.rs` (the detail panel), `commands.rs` |
| `review/` | Review: `queue` (undecided lines newest first, with the key, suggestion and last item for it), `plan_changed` and the active subscriptions for the Subscription choice; `decide`/`decide_as` (a hand decision, optionally with an "Always do this" rule applied to the other lines in the same op; the rule matches `rule_term`: text the owner selected on the card, which must be in the line, else the title's key), `undecided_by_account` |
| `preview/` | PDFs by id (`PdfSource`, `resolve`), the `pdf_view_*` commands and the `pdf-page://` scheme serving page PNGs (in the CSP's `img-src`) |
| `notify.rs` | `Notifier`: records every notification; `deliver` also shows it in the notification center in release builds |
| `settings/` | `settings.json`: appearance, accent, number format, original currency, own transfers, days in the Archive (`SettingsStore`, `get_settings`, `update_settings`) |
| `instance.rs`, `ipc/` | single instance: lock file, Unix socket, one JSON line per request |
| `remote/` | handles each `IpcMessage` from another process |
| `launch.rs` | launch events queued for the page (`take_launch_events`) |
| `window/` | showing and hiding windows; `settings.rs` opens Settings at a page; `navigation.rs` keeps webviews on app pages |
| `error.rs`, `events.rs`, `lock.rs`, `state.rs`, `handlers.rs` | `AppError`, event names, poison-tolerant locks, managed state, the command list |
| `store/` | `json_file.rs`: private dir creation, atomic JSON files |
| `dev/` | debug-only hooks: window frame, background launches, `eval` |
| `test_support/` | fixtures for tests; `statements.rs` builds synthetic statements in the Digitales Banking layout (fixtures, demo data, the debug build's sample document); `demo/` seeds the debug build's made-up data on its first launch (`SEEDERS`; the demo rules decide some lines; `demo/documents.rs` adds three invented invoices; `demo/statements.rs` imports a year of statements through the real pipeline, Household missing February) and `reset` backs `hamster dev seed --yes` |
| `../tests/no_network.rs` | no network crate in `Cargo.toml` or the resolved dependency tree (ADR 0016) |

Frontend (`src/`):

| Path | Holds |
|---|---|
| `api/` | the `api` object (only place that imports `@tauri-apps/*`), `toAppError`, generated `types.gen.ts` |
| `state/` | the zustand store (route and history, filter, toasts, confirmation; `window.hamster` in debug builds), `route.ts` (every page), `useAppEvents` |
| `shell/` | `Window`, `FilesBanner` (red line while stored PDFs are missing or changed), `Sidebar` (pages, tax years, drop zone), `Header` (crumb, title, page controls, filter field with "matches elsewhere" and ⌘↩, + New), page titles |
| `pages/` | each page's body (`PageBody`), the per-page header texts, `FirstRun` (no account yet), `StatementsPage` (coverage per account, Start Review), `AccountPage` (lines by statement with their decision), `TodayPage` (the next step as headline, quiet rows, coming up, the tax year), `ReviewPage` (one line at a time; A, S, I pick, Return confirms, J skips), `SubscriptionsPage` (by next charge, Ended folded, the detail panel), `DocumentsPage` (by year plus Inbox), `DocumentPage` (PDF beside `DocumentForm`, History on the right, finished and excluded banners), `LogPage`, `ArchivePage`, `YearPage` (hero, questions, needs attention, By Kennzahl with copy buttons, By month, … menu) |
| `documents/` | `DocumentForm` (each field saves on its own; Mark Finished), `DepreciationBlock`, `HistoryList`, `ExcludeSheet`, `DeleteSheet`, `lifecycle.ts`, `LinePicker` (link a bank line), `AddDocumentSheet` (vendor, item, the bank line that paid it, what it counts as), `addFlow.ts` (`startAddDocument` from ⌘O, + New and the sidebar's Choose File…; `attachInvoiceTo`, the tax year's Attach PDF… for one bank line, which the sheet starts on; `useDroppedDocuments`) |
| `subscriptions/` | `flow.ts` (New, New from a line, Edit, Change Plan, End, Resume, Delete), `SubscriptionSheet` (New and Edit, with the live match count; from a line it shows `SourceLine`, whose selected text becomes the match text), `PlanSheet` (Change Plan, with the line it came from), `SubscriptionPanel` (the detail panel beside the list) |
| `review/` | `ReviewCard` (one line, "What is this line?" as a radio group, only the picked choice's details, and a confirm button beside one sentence of what it will do; its title and bank lines are selectable, and a selection becomes the term the Always rule or a new subscription matches, with a live "Matches N lines" count; a "Plan changed?" line offers Update Plan, One-off, New Subscription) |
| `year/` | `YearMenu` (Log, Archive, Export Summary PDF…, Open Ruleset JSON, Close Year… or Open Snapshot PDF and Reopen Year…), `CloseYearSheet`, `AddEntry` (a Kennzahl's "+ Document/Amount/Trip": the add flow with its item set, or the inline amount or trip row), `entryActions.ts` (delete, attach and open evidence, each with an Undo toast) |
| `statements/` | `ImportSheet` (what was read and checked; nothing is stored until Import), `importFlow.ts` (`startImport` from ⌘I, + New, the drop line and the first run; `useDroppedStatements`) |
| `accounts/` | `AccountForm` (first run and Settings) and `AccountSheet` |
| `commands/` | `pageCommands.ts` (what the main window does for each page command), `pageKeys.ts` (single keys a page answers to) |
| `palette/` | the ⌘K palette: commands, then the documents and bank lines the text matches |
| `queries/` | TanStack Query: the client, `useEvents` (event → stale keys, `data_changed` table → stale keys), settings, commands, accounts, tax years and rules, statements (coverage, account lines), line rules (`lineRules.ts`), the review queue (`review.ts`), `useLook` |
| `settings/`, `windows/settings/` | the Settings window: page registry and search, row controls, the pages (`RulesPage` with `RuleSheet`, `VendorsPage`) |
| `components/` | `PdfPreview` (the placeholder the native preview follows), `Modal`, `ConfirmSheet`, `Toasts`, `TooltipHost`, `ErrorBoundary`, `icons.tsx` (lucide icons by role) |
| `lib/` | `pdfCover.ts` (overlays hide the native preview: `useCoversPdf`, `usePdfCovers`), `theme.ts` (scheme and accent on `<html>`), `motion.ts` (`usePresence`, `useGlide`, `useFlash`, `useListGlide`) |
| `styles/` | `palettes/` (every color, light and dark), accents, motion tokens, base, controls, shell, overlays, statements, rules, review, documents |
| `test/` | `createApiMock`, the module boundary test |

## Webview hardening

- CSP in `src-tauri/tauri.conf.json` (`csp` and `devCsp`): `script-src 'self'`, `style-src-elem 'self'`, no
  frames, objects, base URI or form actions, `connect-src` only to IPC (ADR 0017).
- PDFs are never embedded in the webview. The preview is a native PDFKit `PDFView` placed over a page
  placeholder by Rust (selectable text, no links, no PDF JavaScript); overlays suspend it (ADR 0037).
  Thumbnails and the year summary are PNGs served by id through an app URI scheme (ADR 0018).
- `window/navigation.rs` keeps every webview on the app's own pages; web links go through `open_url`.
- The bundle is signed with the hardened runtime (`scripts/sign-app.sh`).

## Platform

macOS on Apple Silicon only; see [adapting.md](adapting.md) for every macOS- or name-specific place.
