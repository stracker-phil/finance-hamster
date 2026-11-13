# 0032. The menu bar is updated in place

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `menu-bar-updated-in-place`); reference implementation: Taskboard (commit "Switch views instantly; update menus in place").

## Context

Rebuilding the menu bar makes macOS redraw every title and flicker.

## Decision

The menu bar is built once and updated with `set_enabled`/`set_checked`; it is rebuilt only when its items or keys change (for example a new tax year in the Go menu or a rebound key), comparing with what the shown bar was built from.

- Rejected: Rebuilding on every state change.

## Consequences

- The menu module remembers what it built.
