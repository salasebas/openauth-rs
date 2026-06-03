# Parity gaps (deep-dive pass)

Items verified in **source** on a third review. These are easy to miss in READMEs
because behavior spans `openauth-sqlx`, `openauth-core`, and `openauth` builder
wrapping.

## Resolved: `find_many` with 2+ joins (2026-06)

### What changed

- SQL adapters (`openauth-sqlx`, `openauth-tokio-postgres`, `openauth-deadpool-postgres`)
  always route `find_many` / tx `find_many` through `SqlAdapterRunner` (no
  `joins.len() <= 1` gate).
- `AdapterCapabilities::supports_native_joins` + `with_native_joins()` marks adapters
  that compile multi-join SQL in one statement.
- `JoinAdapter` delegates when the inner adapter has `supports_native_joins`; memory
  adapters still use fallback unless `experimental.joins` is on.
- `ExperimentalOptions::joins` defaults to **`true`** (can be disabled explicitly).

### Tests

- `openauth-core/tests/db/sql.rs`:
  `find_many_with_joins_statement_compiles_account_and_session_joins`
- `openauth-core/tests/db/adapter_factory.rs`:
  `join_adapter_passes_multi_joins_when_inner_supports_native_joins`
- Adapter contract + sqlx capability tests assert `with_native_joins()`.

---

## SQLite timestamp DDL vs upstream

| | Upstream `get-migration` (sqlite) | OpenAuth `SqlDialect::Sqlite::sql_type` |
| --- | --- | --- |
| Date/timestamp fields | `date` type in migration map | `DbFieldType::Timestamp` → **`TEXT`** |
| Read path | driver-dependent | RFC3339 **string** bind/parse in `sqlite/query.rs` + `row.rs` |

Consistent internally, but **not** the same on-disk type as Better Auth’s Kysely migrations
(`date` / `timestamptz` elsewhere).

---

## Migration / CLI strictness

| Behavior | Upstream `getMigrations` | OpenAuth |
| --- | --- | --- |
| Type mismatch on existing column | `logger.warn`, may still build DDL | `SchemaMigrationWarning`, blocks apply |
| CLI migrate with warnings | N/A (runtime migrate) | `DbCliError::UnsafeMigration` |

FK mismatch warnings are covered on **SQLite, Postgres, and MySQL** integration tests.

---

## Adapter factory features not in sqlx

Better Auth `createAdapterFactory` (`factory.ts`) includes behaviors **outside** the
Kysely custom adapter, implemented in OpenAuth elsewhere or not at all:

| Upstream factory | OpenAuth |
| --- | --- |
| `withApplyDefault` / field transforms on write | Service layer + `IdPolicy` / hooks |
| `disableTransformInput/Output/Join` | No equivalent flags on `DbAdapter` |
| `debugLogs` / `isRunningAdapterTests` | **No** — sqlx errors embed SQL in `OpenAuthError::Adapter` strings instead |
| Default `findMany` limit **100** | **No** default limit in core query types |
| `forceAllowId` on create | `Create::force_allow_id()` in core (used by routes, not sqlx-specific) |
| `usePlural` config on Kysely adapter | `AuthSchemaOptions` table naming |

---

## SQL planner tests (shared with sqlx)

`openauth-core/tests/db/sql.rs`: **25+** `#[test]` functions covering dialect WHERE,
migrations plan, joins grouping, rate-limit SQL, delete-one strategies, etc.

These validate logic sqlx delegates to `openauth-core`; failures here break all SQL
adapters.

---

## Minor / edge

| Topic | Detail |
| --- | --- |
| Empty `INSERT` | Postgres/SQLite: `DEFAULT VALUES`; MySQL: `INSERT () VALUES ()` |
| SQLite `update` one row | `rowid` subquery + `RETURNING` (not identical SQL shape to Kysely) |
| `count` negative / overflow | Core maps scalar to `u64` with `NumericOutOfRange`; Kysely coerces bigint to number |
| `indexmap` in `openauth-sqlx/Cargo.toml` | Used only in integration tests, not in `src/` |
| Pool configuration | `connect` only sets SQLite `foreign_keys`; no WAL/busy_timeout helpers |
| Rate limit row `key` in `sqlite_record` | Set to empty; `consume_sql_rate_limit_record` uses input key |

Rate-limit deny-without-increment is tested on **SQLite, Postgres, and MySQL** (sqlx)
and on tokio-postgres / deadpool-postgres adapters.

---

## Upstream-only surfaces (unchanged)

- `e2e/adapter` **node-sqlite** + **mssql** Kysely tests (~93 cases; not fully ported)
- Kysely **D1 / Bun** dialect shims
- Join column alias `_Joined${Capitalize...}` workaround (Kysely-specific row shape)
- Drizzle / Prisma / Mongo adapter packages
