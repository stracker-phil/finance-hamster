# 0028. Motion tokens and stable controls

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `motion-and-stable-controls`); reference implementation: DataBee ADR 0041 (Taskboard `src/lib/motion.ts`).

## Context

Review runs through lines one at a time; the owner's hand must never land on a different control after a click.

## Decision

Motion tokens only (`src/styles/motion.css`: `--motion-fast`, `--motion-base`, `--motion-slow`, `--motion-highlight`, eases, a reduced-motion rule) and the helpers in `src/lib/motion.ts`. A click never changes what is under the cursor; lists keep their order while open (a decided Review line or a reviewed row stays in place, grayed); toggles keep their size; optional buttons keep their slot. `scripts/dev/e2e.sh` checks it.

- Rejected: Literal durations.
- Rejected: Re-sorting lists while in use.

## Consequences

- Every UI step clicks new controls twice and runs `e2e.sh`.
