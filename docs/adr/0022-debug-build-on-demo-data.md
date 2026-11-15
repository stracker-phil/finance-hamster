# 0022. The debug build is a separate app on demo data

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `debug-build-on-demo-data`); reference implementation: Taskboard ADR 0011, TimeTracker ADR 0031. Scheme clause superseded by 0047.

## Context

The repo is public. Screenshots, e2e runs and agents must only ever see made-up statements, invoices and amounts.

## Decision

- `Hamster (Debug)`, `com.philippstracker.hamster.debug`, scheme `hamster-debug`, its own config dir, socket, lock and logs; never installed. `lib.rs` `IDENTIFIER`, `APP_NAME`, `LINK_SCHEME` are cfg consts tested against both configs.
- On first launch with an empty folder it seeds made-up data (`test_support/demo/`): three example accounts at an invented bank, statements with a gap, invented vendors, invoices, subscriptions with a plan change, an open and a closed tax year. Statement and invoice PDFs are generated from invented data in the bank's layout. `hamster dev seed --yes` starts over.
- The parser fixtures are synthetic statements in the same layout. Real sample statements never enter the repo; an ignored test can read them from a folder named in `HAMSTER_REAL_FIXTURES`.
- Outside effects are faked by default: notifications are recorded, not posted (`HAMSTER_REAL_NOTIFICATIONS=1` posts them).

- Rejected: One shared identity.
- Rejected: Committing the real sample statement as a fixture.

## Consequences

- The demo generator has to keep up with every parser change.
