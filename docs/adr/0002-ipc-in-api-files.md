# 0002. IPC only in api.ts files

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `ipc-in-api-files`); reference implementation: Taskboard ADR 0002.

## Context

UI tests need one mockable contract, and a module must be removable as a folder.

## Decision

Only `src/api/index.ts` and `src/<module>/api.ts` import `@tauri-apps/*`. Everything else imports the `api` object. UI tests mock exactly that with `createApiMock()` (`src/test/apiMock.ts`); `src/test/boundary.test.ts` fails on any other import.

- Rejected: `invoke`/`listen` scattered through components.
- Rejected: One shared API file for all modules.

## Consequences

- Every IPC call is visible in one place per module.
- A new command touches `api.ts` and `apiMock.ts` together.
