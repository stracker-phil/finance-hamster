# 0021. Test doubles live behind test-util

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `test-doubles-behind-test-util`); reference implementation: Taskboard ADR 0006.

## Context

Integration tests and the debug build need fakes and the demo seed; release must not ship them.

## Decision

Test doubles, fixtures and the demo seed compile under `cfg(any(test, feature = "test-util", debug_assertions))` in `src-tauri/src/test_support/`. The crate lists itself as a dev-dependency with `test-util`, so `src-tauri/tests/` sees them.

- Rejected: Compiling them unconditionally.
- Rejected: A separate crate.

## Consequences

- Release builds carry no demo data.
