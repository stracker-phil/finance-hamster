# 0013. JSON stores repair themselves on load

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `json-stores-repair-on-load`); reference implementation: TimeTracker ADR 0006 / Daybook ADR 0013.

## Context

Settings must load before the database and survive hand edits and old versions.

## Decision

Settings, keybindings and UI state go through `JsonStore` over `JsonFile`: atomic writes, 0600, a missing file loads defaults, unknown keys are ignored, an unparsable file is moved aside as `<name>.<timestamp>.corrupt`. One typed camelCase `Settings` model, ts-rs generated, repaired on every load and save (`Settings::repaired`).

- Rejected: Failing on an invalid file.
- Rejected: Settings in SQLite.

## Consequences

- A bad value falls back to its default instead of breaking the app.
