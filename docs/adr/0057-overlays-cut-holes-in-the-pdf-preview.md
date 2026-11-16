# 0057. Overlays cut holes in the PDF preview instead of hiding it

## Status

Accepted (2026-11-16). Hamster's own. Supersedes the overlay part of 0037; the rest of 0037 stands.

## Context

0037 hid the whole native preview while anything the page draws could overlap it. Every save shows an undo
toast for ten seconds, so the PDF went blank after each typed value, exactly while the owner was copying the
next one from it. Hovering a control with a tooltip, or opening a menu beside the PDF, blanked it as well.

## Decision

- Each overlay says what it is (`useCoversPdf(kind, ref)`): a **sheet** (modal, palette) hides the preview as
  before; a **popover** (menus, the filter's other matches, the bank-line picker) or a **float** (toasts,
  tooltips) is cut out of it.
- `usePdfCovers` sends the overlays' boxes, grown by their shadow, with `pdf_view_cover` whenever one opens,
  closes, moves or changes size. Rust masks the view's layer so it draws nothing inside them
  (`CAShapeLayer`, even-odd fill), and the view (`PreviewView`, a `PDFView` subclass) hands clicks inside them
  to the webview below, so a toast's Undo still works.
- While a popover is open, every click on the preview goes to the page, so clicking beside a menu closes it as
  anywhere else.

- Rejected: moving toasts and tooltips away from the preview: menus would still blank it, and the toast's place
  would differ by page.
- Rejected: drawing toasts natively: a second toast implementation for one page type.

## Consequences

- The PDF stays readable while values are typed, undone, or a menu is open.
- `scripts/dev/e2e.sh` checks that sheets hide the preview and that menus and toasts leave it showing.
- Overlay hosts pass a ref to the element they draw; one with `display: contents` is measured by its children.
