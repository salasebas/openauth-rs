# Changelog

All notable changes to `openauth` are documented in this file.

## [Unreleased]

## [0.1.1] - 2026-06-09

### Added

- `openauth::prelude` for the recommended app-dev import surface.

### Changed

- **Breaking:** Email/password authentication is disabled by default. Enable it
  with `OpenAuthBuilder::email_password(EmailPasswordOptions::new().enabled(true))`.
- **Breaking:** `OpenAuthBuilder::build()` is now `async` and wires telemetry when
  the `telemetry` feature is enabled. Removed `build_async` and all `open_auth*`
  initializer free functions; use the builder instead.
- **Breaking:** Removed flat root re-exports of `openauth-core` item types. Import
  from `openauth::prelude` or focused modules (`openauth::api`, `openauth::db`,
  `openauth::options`, `openauth::plugin`, …).
- **Breaking:** Removed `OpenAuth::router()` from the public API. Use
  `handler` / `handler_async`, or mount through `openauth-axum`.

### Fixed

- Umbrella README SQLx quick start now imports `openauth::sqlx::SqliteAdapter`
  behind the documented `sqlx-sqlite` feature instead of requiring a separate
  `openauth-sqlx` dependency.
- SQL/memory/Postgres adapter constructors apply `database_hooks` once instead of
  wrapping the inner adapter on every `new`.

## [0.1.0] - 2026-06-08

### Changed

- Workspace **0.1.0** release: see repository root `CHANGELOG.md` for MCP
  unification, `snake_case` logical database schema names, and breaking plugin
  surface changes.

## [0.0.6] - 2026-05-24

### Added

- Added umbrella feature wiring for `openauth-i18n`.
- Added optional umbrella exports for the split OIDC, SAML, and SCIM crates.
- Added public API and feature-flag coverage for the expanded crate surface.

### Changed

- Kept the top-level crate aligned with the workspace feature split and new
  integration crates.

## [0.0.5] - 2026-05-19

### Added

- Published the beta umbrella crate release line.
