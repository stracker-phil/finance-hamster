# 0042. Reminders are told by the running app

## Status

Accepted (2026-11-13). Hamster's own. Supersedes the reminder clause of 0035 ("scheduled ahead with the macOS
notification center"); the rest of 0035 stands.

## Context

0035 planned to schedule reminders ahead in the notification center so they arrive while Hamster is not
running. The family's notification plugin (`tauri-plugin-notification`, as in Taskboard and Daybook) shows a
notification now; it cannot schedule one on macOS. Hamster is a Dock app that keeps running when its window
closes, so a reminder can be told when it falls due.

## Decision

- **The running app tells due reminders** at launch and every half hour (`reminders::tell`). A reminder is
  due on its day and stays due until told: renewal (the subscription's reminder before its next charge), a
  price or plan change (a line matching a subscription's text at another amount), an overdue import (N days
  after an account's last statement ends), a statement gap, the filing reminder (1 March after an open year)
  and unanswered year questions (from 1 November).
- **Each reminder is told once**, by a key in `reminders.json` beside the database. That file records
  notifications, not the owner's data, so it is not journaled.
- **The release build shows them** through `tauri-plugin-notification`; the debug build and tests only record
  them (`notify::Notifier`, ADR 0022).
- **Settings → Reminders** holds the default reminder for new subscriptions, price changes on or off, the
  import interval in days, gaps on or off, and the two tax year reminders.

- Rejected: a launch agent or helper that schedules with `UNUserNotificationCenter`: more moving parts than
  the job needs, and a second signed binary.

## Consequences

- A reminder that falls due while Hamster is quit is told at the next launch.
- Notifications need the owner's permission once per signing identity.
