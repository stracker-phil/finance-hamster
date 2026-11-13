# 0008. Commands live in a Rust bus

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `commands-in-rust`); reference implementation: Taskboard ADR 0012 (Daybook ADR 0017 is the most complete).

## Context

Import Statement, Add Invoice PDF and New Subscription are reachable from the + New menu, the menu bar, the palette, shortcuts and the CLI; they must not drift.

## Decision

App-level actions are commands of one Rust bus with colon ids (`statement:import`, `document:add`, `subscription:new`, `go:today`, `app:settings:<page>`), title, help, enabled check and handler; `CommandBus<H: Host>` is testable without Tauri. Menu bar, palette, shortcuts, CLI and links run them by id. In-window keys (the Review choices A, S, I, J) live in the page registry as `page:<id>`, rebindable in Settings → Shortcuts. Links run an allowlist of command ids.

- Rejected: A handler wired per surface.
- Rejected: Everything in the page.

## Consequences

- Actions work with the window hidden.
- Each new action is one bus entry plus its menu row.
