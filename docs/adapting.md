# Adapting Hamster

Everything that is specific to the name, to macOS or to the owner's setup, for renaming, porting or forking.
`CLAUDE.md` points to `.claude/rules/`; the `ship-step` skill runs after every step; `.claude/agents/` hold
the subagents for tests, suites and reviews. Every new name-, path- or platform-specific place gets a row here.

## Identity

| What | Release | Debug | Where |
|---|---|---|---|
| Product name | `Hamster` | `Hamster (Debug)` | `tauri.conf.json` `productName` and window title, `tauri.debug.conf.json`, `lib.rs` `APP_NAME`, `index.html` |
| Bundle identifier | `com.philippstracker.hamster` | `com.philippstracker.hamster.debug` | both configs and `lib.rs` `IDENTIFIER` (tested); names the config dir (every profile's data and PDFs) and logs |
| Binary and CLI | `hamster` | the same, called by path | Cargo package name (also the self dev-dependency), `cli/` |
| Installed links | `~/Applications/Hamster.app`, `~/.local/bin/hamster` | never installed | `scripts/install.sh` |
| Socket and lock | `hamster.sock`, `hamster.lock` in the config dir | its own | `lib.rs`, `instance.rs`, `scripts/dev/js.sh` |
| Config dir | `~/Library/Application Support/<identifier>/`, each profile's data in `profiles/<id>/` | its own | `lib.rs` `config_dir()`, `profiles::dir()` |
| Logs | `~/Library/Logs/<identifier>/` | its own | Tauri `app_log_dir` |
| Signing | `HAMSTER_SIGN_IDENTITY`, else Apple Development, else `Hamster Local Signing` | same | `scripts/sign-app.sh` |
| Env vars | `HAMSTER_SIGN_IDENTITY` | adds `HAMSTER_WINDOW_FRAME`, `HAMSTER_BACKGROUND`, `HAMSTER_DEV_TERMINAL` | `src-tauri/src/dev/`, `scripts/dev/`; `HAMSTER_REAL_FIXTURES` in `src-tauri/tests/` |
| Dev store | — | `window.hamster` (`VITE_DEV_HOOKS=1`) | `src/main.tsx` |
| Data | the owner's | the demo seed | `src-tauri/src/test_support/` |
| UI languages | English, German (de-DE, du) | same | `settings::Language`, `src-tauri/src/i18n.rs`, `src/i18n/` (ADR 0050); a third language is a new enum value plus one text per call site and catalog key |
| Author, license | Philipp Stracker <info@philippstracker.com>, MIT | | `Cargo.toml`, `LICENSE` |

## Network hosts

None: the app has no network code (tests check the dependency tree, ADR 0016). The only listener is the
CLI's Unix socket in the config dir. A new host or listener needs an ADR and a row here.

## Platform-specific code

- macOS on Apple Silicon only: the config path, Unix sockets, the overlay title bar, the Dock model,
  codesigning (`scripts/sign-app.sh`, `scripts/create-signing-cert.sh`).
- Dev scripts use zsh, Swift (`CGWindowList`) and `screencapture`.
- PDFKit through objc2 (`platform/pdf`): text extraction for statements and invoices, page rendering for
  thumbnails and the year summary PDF; the native `PDFView` over the page (`platform/pdf_view`, ADR 0037).
- `NSDateFormatter` through objc2: dates in the macOS region (ADR 0010).
- The macOS notification center (`notify.rs`): local reminders.

## Common changes

| Goal | Start here |
|---|---|
| A new command or shortcut | `src-tauri/src/commands/`, the menu table in `src-tauri/src/menu/`, `docs/ux.md` → Keys |
| A new setting | the `Settings` model in `src-tauri/src/settings/`, a page in `src/settings/pages/` (`src/settings/registry.ts`) |
| A new Tauri command | the feature's `commands.rs`, `handlers.rs`, `src/api/index.ts`, `src/test/apiMock.ts` |
| A new CLI subcommand | `src-tauri/src/cli/`, `src-tauri/src/remote/` |
| A new color | a token in every palette under `src/styles/palettes/` |
| A new bank statement layout | a new parser module beside the existing ones, its synthetic fixture, the layout id stored on each statement |
| A new tax year's Kennzahlen | add `src-tauri/resources/rules/rules-<year>.json` and list it in `taxrules::files::BUNDLED`; without a release, Settings → Tax Rules → Start <year> copies the previous year |
| A new journaled field | the op that writes it, its inverse, the History label |

## Conventions to keep

- Rust owns the logic; the page renders. IPC only in `api.ts` files; types generated with ts-rs.
- Small feature modules; split files over ~400 lines.
- No color literal outside the palettes; the native look of `ux.md`.
- Tests through public interfaces only; test code written and run by the agents.
- Debug is a separate app on demo data; nothing in the repo carries real data.
- Docs by file and symbol, never by line number; an ADR for every decision, never edited once accepted.
