# Deadpool Postgres Upstream Parity Audit

## Summary

Target: `crates/openauth-deadpool-postgres`.

Better Auth does not ship a Deadpool-specific adapter package. The closest
server-side upstream references are the generic adapter contract, the Postgres
paths of the Kysely and Drizzle adapters, migration generation, and upstream
adapter conformance suites. The OpenAuth crate is intentionally a pooled
Postgres adapter that delegates SQL semantics to the shared
`openauth-tokio-postgres` driver and `openauth-core` SQL runner.

The first pass found the target already covered most upstream SQL adapter
behavior with focused Postgres conformance tests, plus Rust-specific pool, TLS,
migration, and rate-limit coverage. A second pass found one real server-side
integration gap at the OpenAuth core/schema boundary: runtime
`user.additional_fields` and `session.additional_fields` were accepted by route
validation, but `create_auth_context` did not project them into `db_schema`.
Database-backed auth flows that built migrations from `context.db_schema` could
therefore fail at insert/output time. That gap is now fixed in core and locked
by Deadpool-backed and plugin-schema tests.

## Files Inspected

Upstream Better Auth files:

- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/create-test-suite.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/test-adapter.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/basic.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/case-insensitive.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/joins.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/transactions.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/number-id.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/uuid.ts`
- `upstream/better-auth/1.6.9/repository/packages/test-utils/src/adapter/suites/auth-flow.ts`
- `upstream/better-auth/1.6.9/repository/packages/kysely-adapter/src/kysely-adapter.ts`
- `upstream/better-auth/1.6.9/repository/packages/kysely-adapter/src/query-builders.ts`
- `upstream/better-auth/1.6.9/repository/packages/drizzle-adapter/src/drizzle-adapter.ts`
- `upstream/better-auth/1.6.9/repository/packages/drizzle-adapter/src/query-builders.ts`
- `upstream/better-auth/1.6.9/repository/packages/core/src/db/adapter/*`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/db/schema.ts`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/db/get-migration.ts`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/db/with-hooks.ts`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/db/adapter-base.ts`

OpenAuth files:

- `crates/openauth-deadpool-postgres/src/lib.rs`
- `crates/openauth-deadpool-postgres/src/adapter.rs`
- `crates/openauth-deadpool-postgres/src/config.rs`
- `crates/openauth-deadpool-postgres/src/transaction.rs`
- `crates/openauth-deadpool-postgres/src/rate_limit.rs`
- `crates/openauth-deadpool-postgres/src/migration.rs`
- `crates/openauth-deadpool-postgres/tests/postgres_adapter.rs`
- `crates/openauth-deadpool-postgres/Cargo.toml`
- `crates/openauth-deadpool-postgres/README.md`
- `crates/openauth-deadpool-postgres/examples/deadpool_postgres.rs`
- `crates/openauth-tokio-postgres/src/driver.rs`
- `crates/openauth-tokio-postgres/src/query.rs`
- `crates/openauth-tokio-postgres/src/row.rs`
- `crates/openauth-tokio-postgres/src/schema.rs`
- `crates/openauth-tokio-postgres/src/adapter.rs`
- `crates/openauth-tokio-postgres/src/transaction.rs`
- `crates/openauth-core/src/db/sql/executor.rs`
- `crates/openauth-core/src/db/sql/statements.rs`
- `crates/openauth-core/src/db/sql/dialect.rs`
- `crates/openauth-core/src/db/sql/joins.rs`
- `crates/openauth-core/src/db/sql/rate_limit.rs`
- `crates/openauth-core/src/db/sql/migrations.rs`
- `crates/openauth-core/src/db/factory.rs`
- `crates/openauth-core/src/db/transform.rs`
- `crates/openauth-core/src/db/adapter_harness.rs`
- `crates/openauth-core/src/context/builder.rs`
- `crates/openauth-core/src/context/plugins.rs`
- `crates/openauth-core/tests/api/plugin_router.rs`
- `tests/support/postgres_adapter_conformance.rs`

## Confirmed Matches

- CRUD behavior matches the upstream adapter contract: create, find one, find
  many, count, update, update many, delete, and delete many.
- Select projections, sorting, limit/offset, empty `IN` and `NOT IN`, empty
  updates, single-row delete, and null predicates are covered and match the
  upstream observable behavior.
- Case-insensitive string operators match upstream Kysely/Drizzle behavior for
  `eq`, `ne`, `in`, `not_in`, `contains`, `starts_with`, and `ends_with`.
- Postgres JSON values round-trip as JSONB and OpenAuth array fields use native
  Postgres array columns.
- Database-generated UUID and serial IDs are supported and tested.
- Physical table and column names from schema options are honored for CRUD,
  migrations, and rate limiting.
- Additive migration planning and execution covers table creation, plugin
  tables, plugin columns, indexes, foreign keys, type mismatch warnings, and
  `current_schema()`/`search_path`.
- Transactions commit successful callbacks, roll back callback errors and SQL
  errors, and reject nested transactions with explicit adapter errors.
- Native joins work for a single join and the fallback join adapter handles
  multi-join behavior, missing child rows, and transaction-local joins.
- SQL-backed rate limiting uses an explicit transaction plus Postgres
  `FOR UPDATE` locking through the shared SQL rate-limit plan.
- Pool checkout and connection failures are surfaced as explicit
  `OpenAuthError::Adapter` values with Deadpool context.
- Additional user and session fields configured through OpenAuth options or
  plugin init output are projected into runtime schema metadata, so DB-backed
  migrations and route flows share the same field contract.

## Confirmed Differences

- Upstream Kysely and Drizzle adapters expose transactions as an opt-in adapter
  configuration. OpenAuth Deadpool advertises native transactions unconditionally
  because it owns a Postgres pool and can open a transaction for each callback.
- Better Auth has no Deadpool package. `DeadpoolPostgresAdapter`,
  `connect_checked`, TLS constructors, `from_config*`, and
  `DeadpoolPostgresRateLimitStore` are Rust-specific public APIs.
- Better Auth's JavaScript adapter factory applies dynamic default values and
  ID-generation hooks around each adapter. OpenAuth models these concerns in
  Rust schema/options and core call sites instead of duplicating the TypeScript
  adapter factory in the Deadpool crate.
- Nested transactions return an explicit OpenAuth adapter error instead of using
  savepoints. This is documented in the crate README and locked by tests.
- Kysely marks `supportsArrays: false` even for Postgres because its adapter
  path stringifies arrays. OpenAuth Deadpool intentionally advertises arrays
  because the Rust Postgres driver binds and decodes native arrays.

## Risks

- Any bug in `openauth-core` SQL planning, schema projection, or
  `openauth-tokio-postgres` driver semantics affects this crate because
  Deadpool delegates most behavior there.
- The upstream adapter suite is TypeScript-level and includes factory behavior
  that is not one-to-one with OpenAuth's Rust API boundary; future audits should
  continue separating adapter behavior from core hook/default behavior.
- Tests require a reachable Postgres database. When
  `OPENAUTH_TEST_POSTGRES_URL` is unset they use
  `postgres://user:password@localhost:5432/openauth`.

## Implemented Fixes

- Project `OpenAuthOptions.user.additional_fields` and
  `OpenAuthOptions.session.additional_fields` into `AuthContext.db_schema` when
  building the context.
- Preserve additional field database metadata in that projection: logical name,
  optional/required state, input/generated state, returned/hidden state, field
  type, and custom `db_name`.
- Apply the same schema projection for plugin init outputs that register
  `user_additional_fields` or `session_additional_fields`.

If a future audit identifies a shared SQL behavior bug, add the failing
regression first in `tests/support/postgres_adapter_conformance.rs` when it
applies to all Postgres adapters, or in
`crates/openauth-deadpool-postgres/tests/postgres_adapter.rs` when it is
Deadpool-specific.

## Tests

Existing focused coverage already exercises the upstream-relevant behavior:

- `deadpool_postgres_adapter_filters_sorts_limits_counts_and_mutates`
- `deadpool_postgres_adapter_applies_case_insensitive_string_operators`
- `deadpool_postgres_adapter_supports_empty_mutations_and_delete_one`
- `deadpool_postgres_adapter_handles_null_predicates_in_groups_and_updates`
- `deadpool_postgres_adapter_round_trips_json_arrays_and_create_select`
- `deadpool_postgres_adapter_creates_native_postgres_array_columns`
- `deadpool_postgres_adapter_returns_database_generated_uuid_ids`
- `deadpool_postgres_adapter_returns_database_generated_serial_ids`
- `deadpool_postgres_adapter_supports_forced_uuid_ids`
- `deadpool_postgres_adapter_uses_physical_names_from_auth_schema`
- `deadpool_postgres_adapter_plans_and_runs_migrations`
- `deadpool_postgres_adapter_run_migrations_adds_plugin_columns_to_existing_tables`
- `deadpool_postgres_adapter_run_migrations_creates_plugin_tables_with_indexes_and_foreign_keys`
- `deadpool_postgres_adapter_supports_native_and_fallback_joins`
- `deadpool_postgres_adapter_returns_empty_or_null_for_missing_join_rows`
- `deadpool_postgres_adapter_rolls_back_failed_transactions`
- `deadpool_postgres_adapter_commits_successful_transactions`
- `deadpool_postgres_adapter_rolls_back_after_sql_error_in_transaction`
- `deadpool_postgres_adapter_rejects_nested_transactions`
- `deadpool_postgres_rate_limit_store_is_atomic_and_uses_physical_names`
- `deadpool_postgres_adapter_supports_core_auth_route_flows`
- `deadpool_postgres_adapter_supports_additional_user_fields_route_flow`
- `deadpool_postgres_adapter_supports_password_reset_verifications`
- `plugin_additional_fields_update_runtime_options_and_schema`

The new Deadpool test was first run against the pre-fix implementation and
failed with a 500 response from sign-up. After the schema projection fix, it
passes and verifies that additional user fields are migrated, persisted, and
returned through the Postgres-backed sign-up route.

## Server-Side Parity Estimate

Estimated server-side parity for the Deadpool Postgres target against the
applicable upstream Better Auth SQL/Postgres adapter behavior: **94%**.

This estimate is high because the observable SQL adapter contract, Postgres
data mapping, migrations, transactions, joins, rate limiting, and core
email/password auth flows are covered. It is not 100% because Better Auth has no
Deadpool adapter package and some upstream JavaScript factory behavior remains
intentionally represented elsewhere in OpenAuth instead of in this crate.

Remaining meaningful gaps or intentional differences:

- No Deadpool-specific upstream package exists, so pool construction,
  `connect_checked`, TLS configuration, and the Deadpool rate-limit store are
  Rust-only APIs without one-to-one upstream parity.
- OpenAuth still does not duplicate Better Auth's dynamic JavaScript adapter
  factory inside `openauth-deadpool-postgres`; ID/default handling lives in
  Rust schema/options and core stores/routes.
- Nested transactions remain explicit errors rather than savepoints.
- Migration output is behaviorally additive and Postgres-focused, but not an
  exact clone of Better Auth's TypeScript migration file generation surface.
- Direct low-level `DbAdapter` calls can bypass route/core validation hooks that
  Better Auth's JavaScript adapter factory wraps dynamically.

## Items Intentionally Left Unchanged

- Transaction support remains always advertised for this adapter.
- Nested transactions remain explicitly unsupported.
- Deadpool-specific constructors and rate-limit store APIs remain documented as
  Rust-specific extensions.
- Shared SQL query planning and Postgres driver behavior were not modified; the
  concrete gap was schema projection from runtime options into context schema.
