# 0023. Debug hooks drive the app without focus

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `debug-hooks`); reference implementation: Taskboard ADR 0011.

## Context

Agents and scripts must check the running debug app without stealing focus or clicking with AppleScript.

## Decision

Debug-only hooks (`dev.rs`/`dev/`): `HAMSTER_WINDOW_FRAME`, `HAMSTER_BACKGROUND=1`, `eval` over the socket (`@<label>` targets a window), the zustand store on `window.hamster` with `VITE_DEV_HOOKS=1`. Scripts: `scripts/dev/launch.sh`, `js.sh`, `screenshot.sh`, `e2e.sh`. Never AppleScript clicks, never `open`.

- Rejected: AppleScript UI scripting.

## Consequences

- `e2e.sh` and the README-screenshot script are adapted when the main window is built.
