# 0018. PDFs and bank text are untrusted content

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `untrusted-content`); reference implementation: Taskboard ADR 0042.

## Context

Invoices and statements come from outside. Their text ends up in the UI, and the document page previews the PDF.

## Decision

- Bank text and PDF text are plain text in Rust and rendered as text, never HTML.
- The PDF preview never embeds a PDF in the webview (frames and objects stay off). Rust renders pages to PNG with PDFKit and serves them through an app URI scheme (`register_asynchronous_uri_scheme_protocol`) that answers only for document and statement ids it knows; never base64 over IPC.
- The webview sends ids, never paths.

- Rejected: `<embed>`/`<iframe>` with the WebKit PDF viewer: needs frames or objects in the CSP.
- Rejected: pdf.js in the page: a large dependency that parses untrusted files in the webview.

## Consequences

- The preview is images; selecting text in the preview is not offered.
