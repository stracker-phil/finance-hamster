# 0031. The palette runs the menus

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `palette-runs-the-menus`); reference implementation: Taskboard ADR 0043.

## Context

⌘K must offer every action without a second list to maintain.

## Decision

⌘K lists every enabled menu-bar item from the `MENUS` table (`menu_commands`), menus components offer while on screen (`usePaletteMenu`), Settings pages and jump targets (tax years, accounts, subscriptions). Ranked on the page: prefix, word start, every word, letters in order.

- Rejected: A separate palette list.

## Consequences

- A new menu item is in the palette for free.
