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
- **JSON stores** (`JsonStore`: atomic, 0600, defaults for missing files, unparsable files moved aside) hold
  settings, keybindings and UI state.

## Repository map

Filled in by feature folder, one line each, as the code is added (step 1 onward).

## Webview hardening

- CSP in `src-tauri/tauri.conf.json` (`csp` and `devCsp`): `script-src 'self'`, `style-src-elem 'self'`, no
  frames, objects, base URI or form actions, `connect-src` only to IPC (ADR 0017).
- PDFs are never embedded in the webview; previews are PNG pages rendered by PDFKit in Rust and served by id
  through an app URI scheme (ADR 0018).
- `window/navigation.rs` keeps every webview on the app's own pages; web links go through `open_url`.
- The bundle is signed with the hardened runtime (`scripts/sign-app.sh`).

## Platform

macOS on Apple Silicon only; see [adapting.md](adapting.md) for every macOS- or name-specific place.
