# 0029. One tooltip host per window

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `tooltip-host`); reference implementation: Daybook ADR 0039.

## Context

Native tooltips are slow and inconsistent.

## Decision

One `TooltipHost` per window takes over every `title`: 450ms, instant within 600ms, on focus and for ellipsized text; ends with the current binding from `useShortcuts`.

- Rejected: Native `title`.
- Rejected: A `<Tooltip>` wrapper per control.

## Consequences

- No tooltip spells a key itself.
