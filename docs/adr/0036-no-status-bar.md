# 0036. No status bar

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `status-bar`); reference implementation: DataBee ADR 0043.

## Context

The mockup has no status bar; Hamster's app-wide facts (statement freshness, files to verify) live on Today, the sidebar badge and banners.

## Decision

The main window has no status bar.

- Rejected: A status bar for app facts: nothing runs in the background that needs one.

## Consequences

- A missing or changed file shows as a red banner, not a status item.
