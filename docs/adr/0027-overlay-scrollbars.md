# 0027. Overlay scrollbars only

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `overlay-scrollbars`); reference implementation: DataBee ADR 0032.

## Context

WebKit draws permanent bars as soon as a scrollbar is styled.

## Decision

No `::-webkit-scrollbar` rule and no `scrollbar-gutter`. Every scroll box has `.scroller` (`overflow: auto; isolation: isolate`) and starts below its title row; menus and modals portal to `<body>`.

- Rejected: Styled bars.
- Rejected: A stable gutter.

## Consequences

- An exception needs its own ADR.
