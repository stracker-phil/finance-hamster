# 0058. Data commands run off the main thread

## Status

Accepted (2026-11-16). Hamster's own; follows the family default, where most commands are `async`.

## Context

Tauri runs a command written as a plain `fn` on the main thread. Hamster's commands were almost all plain:
every save, file hash, PDF text read and journal append blocked the main thread, and with it every redraw of
the window and the native PDF preview. Each write also re-read all of `journal.jsonl` to find its last line,
so a save got slower with every entry ever made.

## Decision

- Every command that reads or writes data, files or PDFs is an `async fn`, so Tauri runs it on its thread
  pool; the window keeps drawing while it works. Commands that only drive the window stay plain: the PDF
  preview's (their order matters), Settings, shortcuts and the command catalog.
- A write appends to `journal.jsonl` without reading it: `Db` remembers the last line's `seq` after launch and
  after each append (`journal::append_after`). After a failed append it forgets it, and the next write reads
  the file once to catch up, as at launch.

## Consequences

- A save costs the same whether the journal has a hundred lines or a million.
- Two commands can now run at once; `Db` already takes one lock per read and per write, so each stays whole.
- A line removed from `journal.jsonl` while the app runs is not restored by the next write; the next launch
  restores it, and Settings → Status reports it until then.
