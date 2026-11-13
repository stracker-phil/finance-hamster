# 0001. Rust owns the logic; React only renders

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `rust-owns-the-logic`); reference implementation: Taskboard ADR 0001.

## Context

Hamster's rules are exact and easy to get subtly wrong: balance checks, statement chains, rule matching, payment-date tax years, file names, ANV amounts, next charges. Each rule needs one tested copy.

## Decision

- Every rule lives in Rust: statement parsing and checks, rule evaluation, review queues, document lifecycle, file naming, tax-year totals, subscription plans and next charges, grouping and sorting of every list, and the text the app shows.
- The page asks Rust for ready-to-render data, renders it and forwards intent (a command id or a write with ids).

- Rejected: Logic split between Rust and the page: rules end up duplicated or untested.

## Consequences

- Rules are unit-tested without a DOM; the CLI gets the same answers as the window.
- Some view models grow in Rust that a web app would compute in the page.
