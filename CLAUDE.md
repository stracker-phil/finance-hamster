# Hamster

A private, offline macOS app for one person with three jobs: collect ANV documents (Werbungskosten) with the
real EUR amount paid, keep bank statements as a permanent record, and track subscriptions. It imports the
bank's text PDF statements, asks only what it cannot know (ANV, subscription or ignore), files invoice PDFs by
payment date, and ends each tax year in Kennzahl totals to copy into FinanzOnline. It does no budgeting, no
bookkeeping, no tax calculation, no bank APIs and makes no network request at all. Four principles decide
every trade-off, earlier wins: privacy, persistence (nothing disappears by accident; an append-only journal),
frozen history (finished documents and closed years are locked), simplicity (the monthly ritual takes under
ten minutes).

The spec (`docs/plan/spec.md`), the mockups (`docs/plan/mockups/`, from the canvas "Finance Helper –
Simplified Concept"; `Main.dc.html` holds every screen) and the step plan (`docs/plan/steps.md`) live in
`docs/plan/` (gitignored). Build only what the current step asks for.

Hamster is part of the owner's family of macOS apps and inherits its architecture, process and look from the
family catalog (the user-level `app-blueprint` skill). The sibling apps (leave out this app's own entry):

- **Taskboard** (`../taskboard`): the newest decisions, scripts and process
- **Daybook** (`../daybook`): the command bus, extra windows, the menu bar
- **TimeTracker** (`../timetracker`): SQLite with sqlx, undo as ops with inverses, seeds, macOS date formatting
- **DataBee** (`../databee`): plugins and ports
- **Launcher** (`../project-launcher`): Settings search over every row, Shortcuts page, the accent choice

Read them; never write to them. Hamster differs only where its job differs; each deviation has an ADR.

## Rules

- Every session or step ends in a commit, so it can be reverted. Commit each finished part of a step as soon as
  it is green, not only at the end.
- Never push. Only commit locally.
- Write unit tests for every change and run them (through the `ci` agent) before every commit; tests and
  lint must be green.
- Tests pin behavior, not internals: public interfaces only, no log assertions, no call counts.
- Refactor continuously: when a file grows large (over ~400 lines of production code) or complex, simplify it
  and split it into focused modules in the same step, without asking.
- Keep modules small, isolated and replaceable (ADR 0006): by feature, pure cores, a
  trait only where a second implementation exists today. No speculative abstractions.
- Demo data only: code, docs, tests, commits and screenshots never contain the owner's real data
  (`.claude/rules/demo-data.md`). The debug build runs on its own data folder with the demo seed.
- UI changes follow `docs/ux.md` and `.claude/rules/stable-controls.md`.
- The implementation plan lives in `docs/plan/` (gitignored). Work one step per session; set its status from
  PENDING to DONE when it is committed.
- Changes to the family catalog go through the owner (`app-blueprint`, mode `record`); never edit it from here.

## Docs
Technical docs live in `docs/` (index: `docs/README.md`). Read `docs/architecture.md` and the page of the
layer you change before changing it; `docs/ux.md` is the design language; `docs/adapting.md` lists everything
macOS- or name-specific. Decisions are in `docs/adr/` (never edited once accepted; a new record supersedes).
Update the affected docs page in the same commit when behavior, commands, files or extension points change.
Document by file and symbol, never by line number.

## Stack
- Rust backend in `src-tauri/` (rustup stable, edition 2024; Homebrew rustup is keg-only:
  `/opt/homebrew/opt/rustup/bin`)
- Tauri 2 + React 19 + TypeScript in `src/`, bun as package manager, zustand, lucide-react, Vitest
- macOS, Apple Silicon only

## Architecture
- Rust owns all logic; React only renders (ADR 0001). Only `api.ts` files call
  `invoke`/`listen` (ADR 0002).
- Errors cross the boundary as `{ kind, message }` (ADR 0003). IPC types are generated with
  ts-rs (ADR 0004).
- Commands live in a Rust bus; menus, shortcuts, palette, CLI and links run them by id
  (ADR 0008).
- The CLI is the same binary and asks the running app over a Unix socket (ADR 0009).
- No color literal outside `src/styles/palettes/` (ADR 0025); the look is native
  (ADR 0024, `docs/ux.md`).
- Debug builds: `Hamster (Debug)`, `com.philippstracker.hamster.debug`, own data and socket, demo seed, dev
  hooks; never installed (ADR 0022).
- Storage: one SQLite file through rusqlite plus the original PDFs, all in the config dir; every write is an op
  through `Db::write`, mirrored to the append-only `journal.jsonl` (ADR 0007, ADR 0012). Money is integer
  cents plus a currency code.
- Undo: ops with inverses; undo is a new journaled op (ADR 0015).
- Reads: TanStack Query hooks in `src/queries/`, invalidated by Rust events (ADR 0014).
- Appearance: Automatic / Light / Dark, one palette each, plus an accent (ADR 0026).
- Network: none; no HTTP client in the crate, tests check the dependency tree (ADR 0016). PDF text and
  previews come from PDFKit: a native `PDFView` over the page, so text can be selected (ADR 0018,
  ADR 0037).
- Tax rules are versioned data per year: items with stable keys, rules marked "in the copy value" or
  "preview only", depreciation schedules; no tax constant in code (ADR 0038).
- A Dock app: closing hides the window; reminders are scheduled macOS notifications (ADR 0035). No status
  bar (ADR 0036).
- Repository map: `docs/architecture.md`.

## Commands
Test, lint and format-check commands run through the `ci` agent, never in the main context.
- `bun run test`: Vitest and Rust unit tests
- `bun run lint`: IPC types drift check, tsc, ESLint and clippy on production code (warnings are errors)
- `bun run release major|minor|patch`: bumps the version, builds the signed release `.app`, installs
  it (`scripts/install.sh`), commits "Release X.Y.Z", tags it and cleans the build folder (`scripts/release.sh`,
  `scripts/clean-build.sh`, ADR 0020); major only with the owner's approval
- `bun run build:app` + `scripts/dev/launch.sh`: debug build, restarted next to the terminal without stealing focus
- `scripts/stop-app.sh <debug CLI path>`: whoever launches the debug app for a check quits it when done
- `scripts/dev/js.sh` / `scripts/dev/screenshot.sh`: drive (store: `window.hamster`) and capture the running
  debug window in the background; `scripts/dev/e2e.sh` checks that nothing jumps
- Signing prefers an Apple Development certificate (Xcode, free Apple ID; ADR 0019); without one,
  `scripts/create-signing-cert.sh` (once per Mac) makes the local fallback that keeps privacy grants

## Workflow
Finish every step with the `ship-step` skill. Rules in `.claude/rules/`; `testing.md` says which agents in
`.claude/agents/` write and run tests.
