# 0061. Switch profiles in place

## Status

Accepted (2026-11-16). Hamster's own. Supersedes the restart in ADR 0060; the folder layout, `profiles.json`
and everything else there stay.

## Context

ADR 0060 switched profiles by starting a new Hamster and quitting the old one. That was simple, but the
window closed and reopened, macOS brought the new process to the front, and its log no longer went where the
first launch sent it.

## Decision

- The running app switches. `AppState` holds the open profile's database behind one swappable handle;
  `Db` knows its profile folder, and a command takes the handle once and uses its database and folder
  together, so a command that started before a switch finishes in the profile it started in.
- A switch opens the new database first (migrations, journal catch-up, built-in rules, the current tax year),
  then writes `active` to `profiles.json`, and only then swaps the database, the profile half of the settings
  and the launch notices. A failure before the swap leaves the open profile as it was.
- Statements and PDFs read for the Import or Add sheet are let go; their files stay where they were dropped.
  The native PDF preview closes, the file check runs again for the new profile.
- The page gets `profile_switched`: menus and sheets close, the route goes back to Today with no history,
  the remembered tab, year and filter reset and every query is dropped and fetched again.
- `hamster profile switch` and `add --switch` ask the running app to switch in place; arguments and output
  are unchanged.

## Consequences

- Switching takes as long as opening a database, without the window going away.
- Background work reads the open profile each time it runs; a file check that finishes after a switch is
  dropped instead of reporting about the other profile.
- The old database closes when the last command holding it finishes.
