# 0026. Automatic, Light or Dark with one palette each, plus an accent

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `appearance`); reference implementation: Launcher ADR 0042.

## Context

The mockup's Settings → Appearance offers Mode (Automatic, Light, Dark) and an accent color.

## Decision

`light.css` and `dark.css` cover the whole window; `settings.appearance` = system | light | dark; `lib/theme.ts` sets `data-scheme` before first render and follows macOS on Automatic. `settings.accent` is System or one of nine hues, changed through the one writer; the current Settings page is filled with the accent.

- Rejected: One palette each without an accent: the mockup asks for one.
- Rejected: A scheme per macOS mode: no dark sidebar look here.
- Rejected: Editor schemes.

## Consequences

- Every accent hue passes the contrast test in both palettes.
