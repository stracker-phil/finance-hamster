# 0005. Panics unwind

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `panics-unwind`); reference implementation: Taskboard ADR 0005.

## Context

A parser bug on one odd statement line must not take down the app or lose an open edit.

## Decision

The release profile keeps `panic = "unwind"`. Locks go through the poison-tolerant `MutexExt`/`RwLockExt` (`lock.rs`); a reachable `unwrap()` becomes an `AppError`; long-running tasks (the launch file check, reminder scheduling) are supervised and restarted.

- Rejected: `panic = "abort"`.
- Rejected: Manual `catch_unwind`.

## Consequences

- A bug fails one command with an error toast.
