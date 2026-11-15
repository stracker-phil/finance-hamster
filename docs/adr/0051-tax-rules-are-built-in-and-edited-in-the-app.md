# 0051. Tax rules are built into the app and edited in the app

## Status

Accepted (2026-11-15). Hamster's own. Supersedes the file handling of 0040: one rules file per year in the data
folder, Reload, Import, Duplicate, Open in Editor. Everything else in 0040 stays: the JSON format, the checks,
one copy value per Kennzahl, frozen rules in a closed year.

## Context

In 1.1.0 the checked rules files were copied into the data folder once, on the first launch. From then on that
folder copy was the source: a release with corrected rules changed nothing until the owner imported or edited
the file in an outside editor and then pressed Reload. The owner's 2026 year stayed on its first version
without any hint outside Settings. Import, Open in Editor, Reload and Duplicate also overlapped: each was a
way of putting JSON into the folder.

## Decision

- **Built-in rules are the default.** The checked `resources/rules/rules-<year>.json` files are compiled into
  the binary (`taxrules::files::bundled`). The running app never reads the source folder. A year starts from
  its built-in rules once the calendar reaches it.
- **Open years follow newer built-in versions.** At launch, an open year that uses the built-in rules and
  whose built-in text differs from the applied one takes it on (`taxrules::adopt_builtin`). The same checks
  apply, including keys that recorded data uses. The change is one journaled op made by the app (so, like
  starting a year, not on the Undo stack), and the window says so once. A version that fails the checks is
  kept out and explained in Settings and Status.
- **The owner's version lives in the year row.** `tax_years.rules_source` is `builtin` or `own`. Changing a
  value in Settings → Tax Rules (`taxrules::edit::set_value`) or saving edited JSON (`edit::set_json`) makes
  the year `own`. The ruleset name gains " (own)", and `basedOn` keeps the built-in version it started from.
  A newer built-in version never replaces the owner's version: Settings offers it, and "Back to the built-in
  rules" returns to it. Each of these is one undoable op.
- **A year without built-in rules** starts with Start <year> in Settings. It takes the built-in rules when
  the app has them, otherwise a copy of the newest year (`files::next_year_text`) as the owner's version.
- **No rules folder.** The folder from earlier versions is left untouched, and the app no longer reads it.
  Years from earlier versions count as `builtin`, so they move to the current built-in rules at the next launch.

- Rejected: keeping the file per year and watching it for changes. The source would still live outside the app,
  editing would still need an outside editor, and the "applied rules differ from the file" state would remain.
- Rejected: only a form for every rule. Structural changes such as a new Kennzahl are rare, and the in-app
  JSON editor under Advanced covers them with the same checks.

## Consequences

- Updating the law for everyone means editing the JSON in the repository and releasing. Updating it for one
  installation before a release is a value change in Settings.
- An automatic update isn't undone with ⌘Z; the next launch would take it on again anyway. To keep older
  rules, the owner edits the year (or saves the older JSON), which makes it the owner's version.
- The rules README in `resources/rules/` is for editing the files in the repository; the app no longer ships it
  to the data folder.
