# Hamster

A private macOS app that hoards every receipt and never loses one. It does three jobs for one person on one
Mac:

- **ANV documents.** Collect invoices for Werbungskosten with the real EUR amount paid, filed by payment
  date, and end each tax year with Kennzahl totals to copy into FinanzOnline.
- **Bank statements.** Import the bank's text PDF statements, check their balances and chain, and keep them
  as a permanent record. A short review asks only what the app can't know: ANV, subscription or ignore.
- **Subscriptions.** Track subscriptions with a plan history, next charges and cost per month.

Hamster makes no network requests. Original PDFs are never modified, every change is written to an
append-only journal, finished documents and closed tax years are locked, and deleting is a slow, logged act.

Not included: budgeting, bookkeeping, tax calculation, bank APIs, multiple users or sync.

## Status

Early development; built step by step. macOS on Apple Silicon only.

## Install

```sh
bun install
bun run release patch   # builds, signs and installs ~/Applications/Hamster.app and ~/.local/bin/hamster
```

See [AGENTS.md](AGENTS.md) for prerequisites and [docs/](docs/README.md) for the technical docs.

## CLI

The app binary is also its CLI and talks to the running app:

```sh
hamster status [--json]
hamster show
hamster quit
hamster commands [--json]   # every command id with its shortcut
hamster run <command-id>    # e.g. go.statements, settings.open
```

## License

MIT
