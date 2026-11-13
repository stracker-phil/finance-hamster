# 0004. IPC types are generated with ts-rs

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `generated-ipc-types`); reference implementation: Taskboard ADR 0004.

## Context

Hand-written TypeScript mirrors of Rust types drift.

## Decision

Every IPC type derives `ts_rs::TS` and exports camelCase to `src/api/types.gen.ts` (one file per module). `bun run types` regenerates; `bun run lint` fails when it differs from the committed file. `.cargo/config.toml` sets `TS_RS_EXPORT_DIR` and `TS_RS_LARGE_INT = "number"`. Amounts cross as integer cents plus a currency code. Hand-mirrored constants get a Rust test comparing them.

- Rejected: Hand-written TS types.
- Rejected: specta/tauri-specta.
- Rejected: JSON Schema.

## Consequences

- The generated file is never edited by hand.
- Tests and lint never run at the same time (both regenerate it).
