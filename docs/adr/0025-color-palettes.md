# 0025. Every color is a palette token

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `color-palettes`); reference implementation: Taskboard ADR 0007.

## Context

Light and dark looks, an accent choice and data colors (plan timelines, coverage bars) must stay consistent and readable.

## Decision

Every color is a CSS custom property in `src/styles/palettes/`; palettes define the same tokens; everything else uses `var()`, `color-mix()` or relative OKLCH. `src/styles/colors.test.ts` fails on a stray literal or a missing token, `src/styles/contrast.test.ts` on low contrast (4.5:1 body, 3:1 meta). Data colors (subscription tiles, plan timelines) are named Apple system hues.

- Rejected: Literal colors in components.

## Consequences

- A new color is a token in every palette.
