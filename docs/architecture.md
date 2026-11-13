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
  hamster.sqlite      accounts, statements, transactions, rules, vendors, documents, document_links,
                      subscriptions, subscription_plans, tax_years, rulesets, pools, year_snapshots, journal
  journal.jsonl       append-only copy of the journal table, one line per changed field, never rewritten
  docs/<year>/        <yyyy-mm-dd>-<vendor>.pdf by payment date; -2, -3 on collision
  docs/inbox/         undated documents under their original name
  statements/<year>/  <account>-<nr>.pdf, year of the statement's end date
  snapshots/          <year>-tax-year.pdf, written when a year is closed
  rules/              rules-<year>.json per tax year, rules.schema.json, README.md (ADR 0040)
  settings.json, keybindings.json, ui-state.json
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
| `commands/` | the bus: `COMMANDS` (id, title, accelerator, who runs it), `run`, `list`, `shortcut_label`; page commands go to the main window as `run_command` |
| `menu/` | the menu bar: macOS's standard items around `MENUS` (command ids); a menu item's id is its command |
| `db/` | `hamster.sqlite`: `migrations.rs` (numbered, `user_version`), `change.rs` (row changes over JSON rows with inverses and field-level journal entries), `journal.rs` (the `journal` table and its `journal.jsonl` mirror, caught up at launch), `undo.rs`, `Db::write`/`undo`/`redo`, `commands.rs` (`undo`, `redo`, the `data_changed` event) |
| `money.rs`, `date.rs` | cents plus currency, amount labels by the number format, the FinanzOnline copy value, typed amounts; `YYYY-MM-DD` dates and their labels in the region's order |
| `accounts/` | bank accounts: IBAN check and mask, `add`/`edit`/`list` and their commands |
| `taxyears/` | the `tax_years` rows (state, applied rules text), `start_from_rules` at launch, `list_tax_years` |
| `taxrules/` | the rules files: `model.rs` (typed file, unknown fields fail), `check.rs` (schema constraints and Reload checks), `files.rs` (the `rules/` folder, seed, backups, next year's draft), `diff.rs` (the Reload preview lines), `overview`/`preview`/`apply`/`import`/`duplicate`, `commands.rs` (Settings → Tax Rules) |
| `../resources/rules/` | the schema, README and seed `rules-2026.json` the app writes to `rules/` |
| `statements/` | bank statements: `parse/` (one parser per bank layout in `PARSERS`; `sparkasse.rs` reads the Sparkasse/Erste text layer into `ParsedStatement` with lines, card and FX details), `check.rs` (balance and line-sum checks naming the page, the chain to the previous statement), `file_hash`; `store.rs` (the `statements` and `transactions` rows), `import.rs` (`prepare` reads and checks a PDF and refuses duplicates and unknown IBANs, `ImportQueue` holds it by token, `commit` copies it to `statements/<year>/` and stores its lines as one undoable op, tagging transfers between own accounts), `view.rs` (the Statements coverage and the Account page lines, formatted), `commands.rs` (choose, commit, cancel, coverage, lines; `dropped` offers PDFs dropped on the main window through `import_ready`) |
| `platform/` | macOS frameworks: `pdf/` (PDFKit: `Pdf` text with the layout kept via `layout.rs`, pages as PNG; `write.rs`, a small PDF writer), `pdf_view.rs` (the native `PDFView` over the page placeholder, links swallowed) |
| `preview/` | PDFs by id (`PdfSource`, `resolve`), the `pdf_view_*` commands and the `pdf-page://` scheme serving page PNGs (in the CSP's `img-src`) |
| `notify.rs` | `Notifier`: notifications recorded, not posted (the real center comes with reminders) |
| `settings/` | `settings.json`: appearance, accent, number format, original currency, own transfers (`SettingsStore`, `get_settings`, `update_settings`) |
| `instance.rs`, `ipc/` | single instance: lock file, Unix socket, one JSON line per request |
| `remote/` | handles each `IpcMessage` from another process |
| `launch.rs` | launch events queued for the page (`take_launch_events`) |
| `window/` | showing and hiding windows; `settings.rs` opens Settings at a page; `navigation.rs` keeps webviews on app pages |
| `error.rs`, `events.rs`, `lock.rs`, `state.rs`, `handlers.rs` | `AppError`, event names, poison-tolerant locks, managed state, the command list |
| `store/` | `json_file.rs`: private dir creation, atomic JSON files |
| `dev/` | debug-only hooks: window frame, background launches, `eval` |
| `test_support/` | fixtures for tests; `statements.rs` builds synthetic statements in the bank's layout (fixtures, demo data, the debug build's sample document); `demo/` seeds the debug build's made-up data on its first launch (`SEEDERS`; `demo/statements.rs` imports a year of statements through the real pipeline, Household missing February) and `reset` backs `hamster dev seed --yes` |
| `../tests/no_network.rs` | no network crate in `Cargo.toml` or the resolved dependency tree (ADR 0016) |

Frontend (`src/`):

| Path | Holds |
|---|---|
| `api/` | the `api` object (only place that imports `@tauri-apps/*`), `toAppError`, generated `types.gen.ts` |
| `state/` | the zustand store (route and history, filter, toasts, confirmation; `window.hamster` in debug builds), `route.ts` (every page), `useAppEvents` |
| `shell/` | `Window`, `Sidebar` (pages, tax years, drop zone), `Header` (crumb, title, page controls, filter field, + New), page titles |
| `pages/` | each page's body (`PageBody`), the per-page header texts, `FirstRun` (no account yet), `StatementsPage` (coverage per account), `AccountPage` (lines by statement) |
| `statements/` | `ImportSheet` (what was read and checked; nothing is stored until Import), `importFlow.ts` (`startImport` from ⌘I, + New, the drop line and the first run; `useDroppedStatements`) |
| `accounts/` | `AccountForm` (first run and Settings) and `AccountSheet` |
| `commands/` | `pageCommands.ts` (what the main window does for each page command), `pageKeys.ts` (single keys a page answers to) |
| `palette/` | the ⌘K command palette |
| `queries/` | TanStack Query: the client, `useEvents` (event → stale keys, `data_changed` table → stale keys), settings, commands, accounts, tax years and rules, statements (coverage, account lines), `useLook` |
| `settings/`, `windows/settings/` | the Settings window: page registry and search, row controls, the pages |
| `components/` | `PdfPreview` (the placeholder the native preview follows), `Modal`, `ConfirmSheet`, `Toasts`, `TooltipHost`, `ErrorBoundary`, `icons.tsx` (lucide icons by role) |
| `lib/` | `pdfCover.ts` (overlays hide the native preview: `useCoversPdf`, `usePdfCovers`), `theme.ts` (scheme and accent on `<html>`), `motion.ts` (`usePresence`, `useGlide`, `useFlash`, `useListGlide`) |
| `styles/` | `palettes/` (every color, light and dark), accents, motion tokens, base, controls, shell, overlays, statements |
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
