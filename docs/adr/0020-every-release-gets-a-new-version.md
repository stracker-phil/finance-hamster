# 0020. Every release build gets a new version

## Status

Accepted (2026-11-13). Inherited from the family catalog (app-blueprint: `release-versions`); reference implementation: Taskboard ADR 0054.

## Context

An installed build must name one commit.

## Decision

`bun run release major|minor|patch|X.Y.Z` (`scripts/release.sh`) is the only way to release: clean tree, version bump in `package.json`, `tauri.conf.json`, `Cargo.toml` and `Cargo.lock`, build, install (`scripts/install.sh`), commit "Release X.Y.Z", local tag `vX.Y.Z`, `scripts/clean-build.sh`. Minor for features and visible or breaking changes, patch otherwise; major only with the owner's approval. A Rust test keeps the version strings equal. Semver from 1.0.0.

- Rejected: A fixed version.
- Rejected: Build numbers or dates.
- Rejected: Strict semver majors for breaking changes.

## Consequences

- Tags are never pushed; a tagged version is never built again.
