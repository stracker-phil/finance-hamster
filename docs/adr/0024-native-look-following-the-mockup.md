# 0024. The native look, with the main window following the mockup

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `native-look`); reference implementation: Taskboard ADRs 0010 and 0017.

## Context

Hamster sits beside the owner's other Mac apps. The owner approved a mockup (Finance Helper – Simplified Concept) for the main window.

## Decision

- System font at macOS text styles, tabular figures, SF Mono only for machine text (IBANs, file names), neutral greys, inset groups without borders, overlay title bar, AppKit control metrics, the accent only on interactive things, a sheet's default button the only filled one (`docs/ux.md`).
- The main window follows the approved mockup in pixels where it shows a detail; Settings and behavior follow the family language. The hamster mascot from the app icon appears in empty and success states.

- Rejected: Cards with borders, filled web buttons, chips.

## Consequences

- Departures from the mockup need the owner's nod.
