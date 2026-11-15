# 0047. No links

## Status

Accepted (2026-11-15). Hamster's own. Supersedes the scheme clause of 0022 and the links clause of 0008.

## Context

The `hamster://` and `hamster-debug://` schemes and `tauri-plugin-deep-link` were set up with the skeleton, but
nothing handles an opened URL, no sibling app links into Hamster, and every way in is something a privacy-first
app has to guard.

## Decision

Hamster registers no URL scheme and has no deep-link plugin. Outside callers use the CLI (`hamster run <id>`),
which asks the running app over its private socket (ADR 0009).

- Rejected: an allowlist of linkable commands (no caller needs one; any web page could open the link).

## Consequences

- `lib.rs` has no `LINK_SCHEME`; the configs override only the name and identifier for debug builds.
- A sibling that needs to link in gets a scheme back with a new record and an allowlist.
