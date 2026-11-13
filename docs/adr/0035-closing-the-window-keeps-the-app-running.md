# 0035. Closing the window keeps the app running in the Dock

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `closing-keeps-running`); reference implementation: Taskboard ADR 0016.

## Context

Hamster is opened for the monthly ritual and now and then to add an invoice; reminders are local notifications.

## Decision

- Hamster is a Dock app. Closing the main window or Settings hides it; only Quit ends the app; `RunEvent::ExitRequested` without a code is ignored. Windows are declared `visible: false` and shown by Rust. The Dock icon, `hamster show` or `focus` brings the window back. No tray, no login item.
- Reminders are scheduled ahead with the macOS notification center, so they arrive while Hamster is not running; each launch reschedules them.

- Rejected: Menu-bar app: nothing needs to run all day.

## Consequences

- Notifications need the owner's permission once per signing identity.
