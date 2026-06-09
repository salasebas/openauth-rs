# Changelog

All notable changes to `openauth-deadpool-postgres` are documented in this file.

## Unreleased

## [0.1.1] - 2026-06-09

### Added

- `DeadpoolPostgresStores` bundles the pooled adapter and SQL-backed rate-limit
  store with `apply_to_options`.
- `DeadpoolPostgresBuilder` replaces the previous matrix of `connect_*` and
  `from_config_*` constructors (`.database_url()`, `.schema()`, `.checked()`,
  `.max_size()`, `.config()`, `.connect()`, `.connect_tls()`, `.build_adapter()`).

### Changed

- **Breaking:** Removed `DeadpoolPostgresAdapter::{connect, connect_checked,
  connect_with_schema, connect_with_schema_checked, connect_tls,
  connect_tls_checked, connect_with_schema_tls, connect_with_schema_tls_checked,
  from_config, from_config_tls, from_config_with_schema,
  from_config_with_schema_tls}`. Use `DeadpoolPostgresAdapter::builder()` or
  `DeadpoolPostgresStores::connect*`.
- **Breaking:** Removed the public `migration` module. Import planning types from
  `openauth_core::db`.

### Fixed

- Reject schema migrations whose plan carries non-executable warnings before any
  statement runs, matching the SQLx Postgres preflight. Because the pooled
  adapter reuses the `openauth-tokio-postgres` schema path, `create_schema` and
  `run_migrations` no longer mutate the schema when the planner reports warnings
  such as column type drift.
- Fixed rate-limit persistence so negative stored counts are rejected instead
  of wrapping to huge values when decoded as `u64`.
- Roll back in-flight transactions when `transaction()` or rate-limit `consume()`
  is dropped before explicit `COMMIT`/`ROLLBACK`, keeping the checked-out pool
  connection until cleanup completes so recycled connections cannot commit
  orphaned writes from an aborted request.

## [0.0.6] - 2026-05-24

### Added

- Added focused adapter, configuration, rate-limit, and transaction modules.
- Added expanded Postgres adapter conformance coverage.

### Changed

- Reworked the deadpool-postgres adapter surface around the shared
  tokio-postgres implementation.

## [0.0.5] - 2026-05-19

### Added

- Published the beta deadpool-postgres adapter release line.

