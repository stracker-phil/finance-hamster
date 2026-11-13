# 0034. Settings is a window of its own

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `settings-window`); reference implementation: Taskboard ADR 0010, Launcher ADRs 0033/0045/0046.

## Context

Hamster's Settings hold rules, vendors, accounts, rulesets, reminders and privacy pages the owner keeps open beside the main window.

## Decision

A window labelled `settings` (~880×720, min 760×560, hidden at start, overlay title bar) in System Settings layout with a 190px page list and a search that finds every row (drift-tested index, two-line hits, accent wash). Pages: Rules, Vendors, Accounts, Tax Rules; Reminders, Privacy & Data, Appearance, Shortcuts, Status, About. Changes apply at once; Rust broadcasts `settings_changed`. Status values copy on click.

- Rejected: A modal sheet.
- Rejected: Settings as a page of the main window.

## Consequences

- Every row needs an anchor and an index entry.
