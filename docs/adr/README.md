# Architecture decision records

Why Hamster is built the way it is. One record per decision, in Nygard format, at most one page.

- An accepted record is never edited. A changed decision gets a new record that supersedes the old one; the
  old one's Status only gains "Superseded by NNNN". A dated amendment under Status may fix facts that changed.
- Status is dated. The Decision names the alternatives it rejected.
- Records taken from the family catalog (the `app-blueprint` skill) say so under Status, with the slug and the
  sibling record that implements it; they state the decision as it holds for Hamster.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-rust-owns-the-logic.md) | Rust owns the logic; React only renders | Accepted (2026-11-13) |
| [0002](0002-ipc-in-api-files.md) | IPC only in api.ts files | Accepted (2026-11-13) |
| [0003](0003-errors-as-kinds.md) | Errors cross the boundary as kinds | Accepted (2026-11-13) |
| [0004](0004-generated-ipc-types.md) | IPC types are generated with ts-rs | Accepted (2026-11-13) |
| [0005](0005-panics-unwind.md) | Panics unwind | Accepted (2026-11-13) |
| [0006](0006-small-feature-modules.md) | Small feature modules | Accepted (2026-11-13) |
| [0007](0007-one-write-path.md) | One write path, mirrored to an append-only journal | Accepted (2026-11-13) |
| [0008](0008-commands-in-rust.md) | Commands live in a Rust bus | Accepted (2026-11-13); bus, page-key and links clauses superseded by 0046, 0047 |
| [0009](0009-cli-asks-the-running-app.md) | The CLI asks the running app | Accepted (2026-11-13) |
| [0010](0010-rust-formats-dates-and-amounts.md) | Rust formats dates and amounts with the macOS region | Accepted (2026-11-13) |
| [0011](0011-sibling-apps-talk-through-the-cli.md) | Sibling apps talk through the CLI | Accepted (2026-11-13) |
| [0012](0012-one-sqlite-file-through-rusqlite.md) | One SQLite file through rusqlite, PDFs as plain files beside it | Accepted (2026-11-13) |
| [0013](0013-json-stores-repair-on-load.md) | JSON stores repair themselves on load | Accepted (2026-11-13) |
| [0014](0014-reads-go-through-tanstack-query.md) | Reads go through TanStack Query | Accepted (2026-11-13) |
| [0015](0015-undo-runs-ops-with-inverses.md) | Undo runs ops with inverses | Accepted (2026-11-13) |
| [0016](0016-no-network.md) | No network | Accepted (2026-11-13) |
| [0017](0017-hardened-runtime-strict-csp-and-no-navigation.md) | Hardened runtime, strict CSP and no navigation | Accepted (2026-11-13) |
| [0018](0018-pdfs-and-bank-text-are-untrusted.md) | PDFs and bank text are untrusted content | Accepted (2026-11-13); preview superseded by 0037 |
| [0019](0019-signing.md) | Builds are signed with an Apple Development certificate | Accepted (2026-11-13) |
| [0020](0020-every-release-gets-a-new-version.md) | Every release build gets a new version | Accepted (2026-11-13) |
| [0021](0021-test-doubles-behind-test-util.md) | Test doubles live behind test-util | Accepted (2026-11-13) |
| [0022](0022-debug-build-on-demo-data.md) | The debug build is a separate app on demo data | Accepted (2026-11-13); scheme clause superseded by 0047 |
| [0023](0023-debug-hooks.md) | Debug hooks drive the app without focus | Accepted (2026-11-13) |
| [0024](0024-native-look-following-the-mockup.md) | The native look, with the main window following the mockup | Accepted (2026-11-13) |
| [0025](0025-color-palettes.md) | Every color is a palette token | Accepted (2026-11-13) |
| [0026](0026-automatic-light-or-dark-plus-an-accent.md) | Automatic, Light or Dark with one palette each, plus an accent | Accepted (2026-11-13) |
| [0027](0027-overlay-scrollbars.md) | Overlay scrollbars only | Accepted (2026-11-13) |
| [0028](0028-motion-and-stable-controls.md) | Motion tokens and stable controls | Accepted (2026-11-13) |
| [0029](0029-tooltip-host.md) | One tooltip host per window | Accepted (2026-11-13) |
| [0030](0030-menus-are-drawn-in-the-page.md) | Menus are drawn in the page | Accepted (2026-11-13) |
| [0031](0031-the-palette-runs-the-menus.md) | The palette runs the menus | Accepted (2026-11-13) |
| [0032](0032-the-menu-bar-is-updated-in-place.md) | The menu bar is updated in place | Accepted (2026-11-13) |
| [0033](0033-keyboard-first.md) | Keyboard first | Accepted (2026-11-13) |
| [0034](0034-settings-window.md) | Settings is a window of its own | Accepted (2026-11-13) |
| [0035](0035-closing-the-window-keeps-the-app-running.md) | Closing the window keeps the app running in the Dock | Accepted (2026-11-13); reminder clause superseded by 0042 |
| [0036](0036-no-status-bar.md) | No status bar | Accepted (2026-11-13) |
| [0037](0037-pdfs-show-in-a-native-pdfview.md) | PDFs show in a native PDFView over the page | Accepted (2026-11-13) |
| [0038](0038-tax-rules-are-versioned-data-per-year.md) | Tax rules are versioned data per year, split into copy values and preview | Superseded by 0040 |
| [0039](0039-werbungskosten-only-open-for-other-l1-sections.md) | Werbungskosten only, open for other L1 sections | Accepted (2026-11-13); partly superseded by 0040 |
| [0040](0040-tax-rules-live-in-a-json-file-per-year.md) | Tax rules live in a JSON file per year; the tax year shows one copy value per Kennzahl | Accepted (2026-11-13); file handling superseded by 0051 |
| [0041](0041-subscriptions-match-their-own-charges.md) | Subscriptions match their own charges, by plan period | Accepted (2026-11-13); tolerance superseded by 0045 |
| [0042](0042-reminders-are-told-by-the-running-app.md) | Reminders are told by the running app | Accepted (2026-11-13) |
| [0043](0043-the-year-check-stores-only-its-date.md) | The year check asks from the ruleset and stores only its date | Superseded by 0052 |
| [0044](0044-subscription-tolerance-is-a-percentage.md) | A subscription's amount tolerance is a percentage | Accepted (2026-11-14); superseded by 0045 |
| [0045](0045-subscriptions-ask-about-another-amount.md) | Subscriptions ask about another amount and take one charge per period | Accepted (2026-11-14) |
| [0046](0046-commands-are-a-catalog-with-colon-ids.md) | Commands are a catalog with colon ids; page keys are fixed | Accepted (2026-11-15) |
| [0047](0047-no-links.md) | No links | Accepted (2026-11-15) |
| [0048](0048-closed-years-are-frozen-in-the-write-path.md) | Closed years are frozen in the write path and show what was filed | Accepted (2026-11-15) |
| [0049](0049-refunds-net-against-charges.md) | Refunds net against charges; one line pays one document | Accepted (2026-11-15) |
| [0050](0050-german-as-a-second-ui-language.md) | German as a second UI language | Accepted (2026-11-15) |
| [0051](0051-tax-rules-are-built-in-and-edited-in-the-app.md) | Tax rules are built into the app and edited in the app | Accepted (2026-11-15) |
| [0052](0052-record-in-plain-words-file-in-finanzonline-terms.md) | Record in plain words, file in FinanzOnline terms; no year check | Accepted (2026-11-15) |
| [0053](0053-hide-unused-categories-per-year.md) | Hide unused categories per year; FinanzOnline lists every Kennzahl | Accepted (2026-11-15) |
