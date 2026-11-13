# 0019. Builds are signed with an Apple Development certificate

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `signing`); reference implementation: Taskboard ADR 0053.

## Context

macOS keeps privacy grants (the notification permission) only across builds with the same Team ID.

## Decision

`scripts/sign-app.sh` signs with `$HAMSTER_SIGN_IDENTITY`, else the first valid "Apple Development" identity picked by SHA-1, else "Hamster Local Signing" from `scripts/create-signing-cert.sh`, else ad hoc. Tauri builds with `signingIdentity: "-"`; install and launch re-sign.

- Rejected: Local certificate only.
- Rejected: Ad hoc only.
- Rejected: Developer ID and notarization.

## Consequences

- The first build with a new identity resets the notification permission.
