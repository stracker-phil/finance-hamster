# 0011. Sibling apps talk through the CLI

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `sibling-apps-talk-through-the-cli`); reference implementation: Daybook ADRs 0022/0031.

## Context

Hamster talks to no sibling app today. If one ever reads Hamster's data, it must not open the database.

## Decision

Any integration with a family app goes through the `hamster` CLI (`--json`) or the URL scheme, never by reading Hamster's files. The fields a sibling reads become a contract pinned by tests; breaking it needs the owner's approval. Debug builds talk only to debug siblings.

- Rejected: Reading another app's files or database.

## Consequences

- Nothing to do until a sibling needs Hamster's data.
