# 0054. Import a rules file for any year

## Status

Accepted (2026-11-15). Hamster's own. Brings back one part of the file handling 0051 removed: importing a
rules file, now as a one-time copy into the year row rather than a file the app keeps reading.

## Context

Since 0051 the only way to put a prepared rules file into Hamster is to start the year and paste the JSON
into Advanced → Edit as JSON. Start only reaches the year after the newest. An earlier year (a late filing)
or a year further ahead can't be set up from a file at all.

## Decision

- **Import JSON…** at the end of Settings → Tax Rules asks for a file and reads the year from it
  (`taxrules::import`). An open year of that number takes the file as its rules; a year that doesn't exist
  starts with it, before the oldest year or after a gap; a closed year refuses it.
- The owner sees what happens before it does: which year starts, or the changes to an existing year's rules.
  The same checks apply as for any rules. Replacing a year's rules also keeps every item key recorded data uses.
- The import is one undoable op. A new year carries over the previous year's hidden categories (0053).
- A file identical to the built-in rules for that year keeps the year following them; anything else is the
  owner's version, as with any edit in Settings (0051).

- Rejected: keeping the import inside the year page. It can't create a year that doesn't exist yet.

## Consequences

- Start stays the usual way into the next year; Import is for prepared files and years Start can't reach.
- Start always continues after the newest year, so a gap stays until the owner imports the missing year or
  the calendar reaches a year Hamster has built-in rules for.
