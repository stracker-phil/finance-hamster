# 0030. Menus are drawn in the page

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `menus-in-the-page`); reference implementation: Taskboard ADR 0034.

## Context

The + New menu, the tax year's … menu, pickers (bank line, vendor, subscription) and context menus need one look and keyboard behavior.

## Decision

One in-app menu component: a store slice holds the open menu, `MenuHost` draws it, `openMenu`/`openContextMenu` open it; macOS behavior and ARIA roles; a `PopUpButton` replaces `<select>`. The menu bar stays native.

- Rejected: Native context menus.

## Consequences

- Menus follow the palettes and are testable.
