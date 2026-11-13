# 0009. The CLI asks the running app

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `cli-asks-the-running-app`); reference implementation: Taskboard ADR 0008.

## Context

A second process writing the database would race the app and bypass the journal.

## Decision

The `hamster` binary is also the CLI. The first process binds `hamster.sock` (0600, peer-uid check) and holds `hamster.lock` in the config dir. A second process parses argv with clap, sends one JSON line, prints the reply (`--json` for machines) and exits. `show`, `focus` and bare launch start the app; other subcommands exit 2 with "isn't running". `status [--json]`, `quit`, `--hidden`; debug builds accept `eval` and `dev seed`.

- Rejected: A CLI that opens the database itself.
- Rejected: `tauri-plugin-single-instance`.

## Consequences

- Every CLI write goes through the journal and undo like a click.
