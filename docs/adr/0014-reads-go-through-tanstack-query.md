# 0014. Reads go through TanStack Query

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `reads`); reference implementation: Taskboard ADR 0018.

## Context

Hamster's pages pull lists (lines, documents, subscriptions, tax-year groups) and change them through writes that touch several pages at once.

## Decision

Page reads go through hooks in `src/queries/` (`staleTime: Infinity`, no retry, one `queryClient` per window); `useEvents.ts` maps Rust events to the query keys they invalidate. zustand keeps UI-only state.

- Rejected: Store and pushes: Rust pushes little here; the page pulls.
- Rejected: Hand-written re-reads with version counters.

## Consequences

- A write's event refreshes every page that shows the data.
