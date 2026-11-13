# 0006. Small feature modules

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `small-feature-modules`); reference implementation: Taskboard ADR 0014.

## Context

Hamster has several independent features (statements, review and rules, documents, tax years, subscriptions, reminders) that should stay changeable on their own.

## Decision

Folders by feature: `src-tauri/src/<feature>/` with pure logic, a thin `commands.rs` and its tests, and a matching `src/<feature>/`. The core never imports a feature; features register in `handlers.rs` and `src/modules.ts`. I/O at the edges, pure cores. Each statement layout is its own parser module behind one `StatementParser` trait, because a second layout is expected. Platform code (PDFKit, notifications through objc2) sits behind one `platform/` module. Files over ~400 lines of production code are split in the same step.

- Rejected: Layering by type (all commands in one place).
- Rejected: A trait per module.

## Consequences

- Features can be removed as folders.
- Cross-feature reads (Today, the tax year page) go through each feature's public functions.
