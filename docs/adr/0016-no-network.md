# 0016. No network

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `network-policy`); reference implementation: TimeTracker ADR 0013.

## Context

Hamster's first principle is privacy: bank statements and invoices never leave the Mac. The spec asks for a build without the outgoing-network entitlement.

## Decision

- The crate has no HTTP client: unit tests scan `Cargo.toml` and `cargo tree` for reqwest, hyper, ureq and updater crates. The webview reaches only IPC (`connect-src ipc: http://ipc.localhost`); no telemetry, update check or crash upload. PDF text is read on the Mac with PDFKit. Links (if any) open in the browser.
- Hamster is not sandboxed (`hardened-runtime` rejects App Sandbox), so there is no network entitlement to leave out. The guarantee comes from having no network code, checked by the tests above and by a network monitor before release. Settings → Privacy & Data says "None" without claiming macOS blocks it.

- Rejected: Listed hosts: nothing to talk to.
- Rejected: App Sandbox to remove the network entitlement: the family rejects it (`hardened-runtime`).

## Consequences

- Adding any network code needs a superseding ADR and a row in `docs/adapting.md` → Network hosts.
