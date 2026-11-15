# 0046. Commands are a catalog with colon ids; page keys are fixed

## Status

Accepted (2026-11-15). Inherited from the family catalog (app-blueprint: `commands-in-rust`, the *catalog*
option; reference: DataBee ADR 0036). Supersedes the bus description and the "rebindable page keys" clause of
0008; the rest of 0008 stands.

## Context

ADR 0008 described the full bus (help, enabled checks, `CommandBus<H: Host>`) and named colon ids, but the code
grew the catalog's smaller variant with dotted ids (`go.today`). No Hamster command takes an argument or needs
an enabled state. Review's A, S, I, J and the Year Check's Y, N, L are mnemonic letters printed on the very
buttons they press, built per line from the choices.

## Decision

- `commands::COMMANDS` is a catalog: id, title, default accelerator and who runs it (`Runs::Page`,
  `Runs::Settings`, `Runs::Rust` for actions that work with the window hidden, such as Help → Open Log Folder
  and Show Data Folder). Menus, the palette, shortcuts and `hamster run <id>` run commands by id.
- Ids use the family's colon form: `go:today`, `statement:import`, `edit:undo`, `app:settings`, `app:logs`,
  `app:data`. `keybindings.json` written with the old dotted ids is carried over on load.
- Page keys are fixed and listed read-only on Settings → Shortcuts, grouped by the page they work on.
- Settings → Shortcuts asks before taking a key another command holds and moves it in one save (Reassign).
- Rejected: the full bus with a `Host` trait (nothing needs arguments or enabled states yet); keeping dotted
  ids (Hamster alone would spell ids differently); rebindable page keys (the letter on the button would no
  longer match the key).

## Consequences

- `hamster run go.today` becomes `hamster run go:today`; a minor release.
- A command that needs an argument or an enabled state is the moment to grow the full bus, with a new record.
