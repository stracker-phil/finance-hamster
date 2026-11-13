# 0017. Hardened runtime, strict CSP and no navigation

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `hardened-runtime`); reference implementation: Taskboard ADR 0049.

## Context

The webview shows text from bank statements and invoices and can call commands.

## Decision

`scripts/sign-app.sh` signs with `--options runtime`; Hamster needs no entitlements (no AppleScript, no calendars). Strict CSP in `csp` and `devCsp` (`script-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'`, `style-src-elem 'self'` in `csp`), `img-src 'self' data:` plus the app's file scheme. A navigation guard (`window/navigation.rs`) keeps every webview on app pages.

- Rejected: App Sandbox: breaks the CLI socket and the family tooling.
- Rejected: CSP alone.

## Consequences

- Adding an entitlement needs its own ADR.
