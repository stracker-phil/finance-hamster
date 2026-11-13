# 0033. Keyboard first

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `keyboard-first`); reference implementation: Launcher ADRs 0032/0045.

## Context

The monthly ritual (import, review, attach) should take under ten minutes, mostly from the keyboard.

## Decision

Every action is a command with a menu item, a palette entry and one rebindable shortcut (⌘I Import Statement, ⌘O Add Invoice PDF, ⌘N New Subscription, ⌘F filter, ⌘K palette). Review's A, S, I, J are page keys, never menu items, and never fire in text fields. Settings → Shortcuts groups rows by where the key works, asks before taking a key another command holds, and shows Reset only off the default. Menus in Mac order; Help has Open Log Folder and Show Data Folder.

- Rejected: Unbound actions.
- Rejected: Plain letters on menu items.

## Consequences

- Each new command adds its default key and row.
