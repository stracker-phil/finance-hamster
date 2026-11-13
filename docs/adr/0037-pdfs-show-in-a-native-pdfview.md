# 0037. PDFs show in a native PDFView over the page

## Status

Accepted (2026-11-13). Supersedes the preview part of 0018; the rest of 0018 stands. Hamster's own.

## Context

0018 rendered PDF pages to images, so text in an invoice or statement could not be selected. The owner copies
amounts, dates and invoice numbers out of text-layer PDFs; typing them by hand is slower and error-prone. The
webview's CSP keeps frames and objects off (ADR 0017), so the WebKit PDF viewer would need an exception.

## Decision

- The PDF preview is a native PDFKit `PDFView` that Rust adds to the main window's content view above the
  webview (`platform/pdf_view`, objc2, main thread). It offers text selection, copy, ⌘F search and zoom, as in
  Preview. The page renders an empty placeholder; a `ResizeObserver` reports its frame, and Rust keeps the
  view on it while scrolling, resizing or switching pages.
- The page asks for a document or statement by id (`pdf_view_show(id, frame)`, `pdf_view_hide()`); Rust opens
  only files it knows, read-only. The view never navigates, follows no links and runs no PDF JavaScript
  (link clicks are swallowed by its delegate).
- Anything the page draws over the preview (menus, sheets, the palette, toasts, tooltips) hides the view
  first, or the native view would cover it: overlay hosts call `pdf_view_suspend`/`resume`, and the e2e
  script checks that the view is hidden whenever an overlay is open.
- Thumbnails and the year summary keep rendering to PNG through PDFKit and the app URI scheme.

- Rejected: the WebKit viewer in an `<iframe>` with a CSP exception for the app's own scheme: opens a hole in
  the policy, and its behavior inside WKWebView varies between macOS versions.
- Rejected: page images with an invisible text layer (pdf.js style): imprecise selection and the most code.

## Consequences

- Selecting and copying from PDFs works as in Preview; the CSP stays strict.
- The native view sits outside the DOM: screenshots and e2e runs must account for it, and every overlay host
  has to suspend it.
- The objc2 code is covered by `docs/adapting.md` → Platform-specific code.
