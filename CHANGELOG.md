# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-29

### Added

- `Phoenix` checker for soroban-sdk 29 test environments: watch contracts, optionally include temporary entries.
- `check_live_for` / `assert_live_for`: every watched instance, code and persistent entry must stay live for a number of ledgers.
- `simulate` / `assert_survives`: run a usage pattern over time, stopping at the first step before which state would be archived or deleted.
- Readable reports with owner, key, archival ledger, approximate time and fix hints.
- `days`, `hours`, `weeks` helpers (5 seconds per ledger).
- Example vault contract whose TTL policy is pinned down by Phoenix tests.

[Unreleased]: https://github.com/Ike-Nathan/phoenix-ttl/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Ike-Nathan/phoenix-ttl/releases/tag/v0.1.0
