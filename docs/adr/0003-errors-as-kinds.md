# 0003. Errors cross the boundary as kinds

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `errors-as-kinds`); reference implementation: Taskboard ADR 0003.

## Context

The UI must react differently to a refused duplicate, a failed balance check, a locked document and a missing file, without parsing English messages.

## Decision

Every command returns `Result<T, AppError>`; `AppError` (thiserror) serializes to `{ kind, message, field? }`. Hamster adds kinds as features need them (for example `duplicate`, `balance_mismatch`, `locked`, `year_closed`). The UI branches on `kind` only; `toAppError` maps unknown kinds to `io`. The socket and `--json` carry the same shape.

- Rejected: Matching on message text: fragile.

## Consequences

- A new failure the UI must treat specially adds a kind and a test.
