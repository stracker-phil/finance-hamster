# 0060. One folder per profile; switching starts the app anew

## Status

Accepted (2026-11-16). Hamster's own.

## Context

Hamster kept one person's data. The owner wants to keep more than one person's documents, statements and tax
years apart (a profile each, with its own name) and switch between them. All data already sat under one
folder, opened once at launch.

## Decision

- Each profile is a folder `profiles/<id>/` in the config dir with everything that was the person's: the
  database, the journal, the PDFs, the tax rule versions, the reminders told and its half of the settings.
  `profiles.json` lists the profiles (id, name) and the active one.
- The app keeps what belongs to the app: the lock, the socket, the shortcuts and the app-wide settings
  (language, appearance, accent, number format), so a switch doesn't change the look or the language.
- One process works in one profile. Switching or making a profile writes the new active id, removes the socket,
  starts a new Hamster through LaunchServices (`open -n`) and quits; the new process waits for the old one's
  lock instead of handing its launch over.
- The first launch with profiles moves the existing data into the first profile ("My Profile", renameable).
- The profile's display name lives in `profiles.json`, so the switcher shows it without opening a database;
  details about the person (a tax number) belong inside the profile.
- Only the active profile's reminders are told.
- In the debug build only the first profile gets the demo data; a new one starts empty.

- Rejected for now: switching inside the running process. It needs the database and every holder of a path
  to be swappable, background work and imports to stop, and every cache to reset. A restart gives all of that
  for free; an in-place switch can replace it later without changing the folder layout.

## Consequences

- A switch closes and reopens the window (about a second).
- Profiles separate data, they don't lock it: anyone who can open the app can open every profile.
- Reminders of other profiles wait until that profile is opened; a top-level reminder store could collect them
  later.
