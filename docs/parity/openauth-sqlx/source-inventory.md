# Source inventory: openauth-sqlx

File-level map verified against the tree (not README summaries). Upstream analogue:
`packages/kysely-adapter/` + `packages/better-auth/src/db/get-migration.ts` +
`e2e/adapter/test/kysely-adapter/`.

## OpenAuth (`crates/openauth-sqlx`)

| Path | Role |
| --- | --- |
| `src/lib.rs` | Feature gates; re-exports adapters, rate-limit stores, `migration` |
| `src/migration.rs` | Re-exports `SchemaMigrationPlan` types; `ensure_executable`; `write_schema_file` |
| `src/sqlite/mod.rs` | `SqliteAdapter`, `SqliteRateLimitStore`, `DbAdapter`, `run_migrations` (in tx) |
| `src/sqlite/state.rs` | `SqlAdapterRunner` over pool/tx |
| `src/sqlite/query.rs` | `SqlParam` → SQLx binds |
| `src/sqlite/row.rs` | Row decode (RFC3339 strings, JSON-in-TEXT arrays) |
| `src/sqlite/schema.rs` | Introspection (`pragma_*`), plan/apply migrations |
| `src/sqlite/support.rs` | Identifier sanitization |
| `src/sqlite/errors.rs` | `sqlx::Error` → `OpenAuthError` |
| `src/postgres/*` | Same layout; `uuid` binds; `information_schema`; `primary_key_column_exists` |
| `src/mysql/*` | Same layout; `information_schema` scoped to `DATABASE()` |
| `tests/sqlite_adapter.rs` | 34 `#[tokio::test]` — broadest coverage |
| `tests/postgres_adapter.rs` | 23 `#[tokio::test]` + 2 sync URL tests |
| `tests/mysql_adapter.rs` | 23 `#[tokio::test]` + 2 sync URL tests |
| `tests/common/mod.rs` | Postgres/MySQL URL defaults + preflight error text |

SQL generation and `DbAdapter` query types live in **`openauth-core`** (`src/db/sql/`,
`adapter_harness.rs`), not in this crate.

### Dialect-only introspection differences

| Introspection | SQLite | Postgres | MySQL |
| --- | --- | --- | --- |
| Tables | `sqlite_master` | `information_schema.tables` + schema filter | `information_schema` + `DATABASE()` |
| Columns | `pragma_table_info` | `information_schema.columns` | `information_schema.columns` |
| Indexes | `sqlite_master` / `pragma_index_list` | `pg_indexes` | `information_schema.statistics` |
| Foreign keys | `pragma_foreign_key_list` | `information_schema` FK metadata | `information_schema` |
| Primary key check for warnings | implicit in column snapshot | explicit `primary_key_column_exists` | via column snapshot |

### Public methods on adapters (beyond `DbAdapter`)

| Method | All dialects |
| --- | --- |
| `new(pool)` / `with_schema(pool, schema)` | yes |
| `connect(url)` / `connect_with_schema(url, schema)` | yes (SQLite: `PRAGMA foreign_keys=ON` in `after_connect`) |
| `plan_migrations(&DbSchema)` | yes |
| `compile_migrations(&DbSchema)` | yes |

### `DbAdapter` behavior implemented here

| Method | Notes |
| --- | --- |
| `find_many` | 0–1 joins: dialect SQL; 2+ joins: `JoinAdapter` (core) |
| `transaction` | Real SQLx transaction; failed callback rolls back |
| `create_schema` | Plans + `ensure_executable` + apply **without** wrapping tx (per statement) |
| `run_migrations` | Plan + `ensure_executable` + apply **inside** one SQLx transaction |
| `delete` | Single-row via core `delete_one_statement` (`LIMIT 1` / `ctid` / `rowid`) |

## Upstream Kysely path

| Path | Role |
| --- | --- |
| `packages/kysely-adapter/src/kysely-adapter.ts` | Custom adapter: WHERE, joins, CRUD, capabilities flags |
| `packages/kysely-adapter/src/dialect.ts` | `createKyselyAdapter`, driver detection (pg, mysql2, sqlite, D1, …) |
| `packages/kysely-adapter/src/query-builders.ts` | Case-insensitive helpers (`LOWER`, `ILIKE`) |
| `packages/kysely-adapter/src/bun-sqlite-dialect.ts` | Bun sqlite driver |
| `packages/kysely-adapter/src/d1-sqlite-dialect.ts` | Cloudflare D1 |
| `packages/kysely-adapter/src/node-sqlite-dialect.ts` | Node `node:sqlite` |
| `packages/kysely-adapter/src/kysely-adapter.test.ts` | **1** smoke test only |
| `packages/better-auth/src/db/get-migration.ts` | `getMigrations`: introspect, `toBeCreated`/`toBeAdded`, Kysely DDL builders |
| `packages/better-auth/src/db/get-migration-schema.test.ts` | Postgres `search_path` / schema isolation (~10 cases) |
| `packages/test-utils/src/adapter/suites/*.ts` | ~93 named contract cases (`basic` alone ≈ 71) |
| `e2e/adapter/test/kysely-adapter/*.ts` | **Real** Kysely parity tests: runs full test-utils suites per DB |

### Upstream e2e Kysely entrypoints (v1.6.9)

| File | Dialect / topic |
| --- | --- |
| `adapter.kysely.sqlite.test.ts` | SQLite + 7 suites |
| `adapter.kysely.pg.test.ts` | Postgres + `schemaRefTestSuite` + `schemaRefJoinTestSuite` |
| `adapter.kysely.mysql.test.ts` | MySQL |
| `adapter.kysely.mssql.test.ts` | MSSQL |
| `adapter.kysely.custom-schema-pg.test.ts` | Postgres non-`public` schema via `search_path` |
| `schema-reference-test-suite.ts` | `internal.users`-style model names |
| `node-sqlite-dialect.test.ts` | Node sqlite dialect shim |

Typical sqlite e2e wiring (from source):

```ts
tests: [
  normalTestSuite(),
  transactionsTestSuite(),
  authFlowTestSuite(),
  numberIdTestSuite(),
  joinsTestSuite(),
  uuidTestSuite(),
  caseInsensitiveTestSuite(),
],
```

Postgres e2e additionally runs `schemaRefTestSuite()` and `schemaRefJoinTestSuite()`, and
disables **all** transaction suite cases (`disableTests: { ALL: true }`) while still
including the suite file.

## Related OpenAuth tests (not in `openauth-sqlx/tests/`)

| Location | What it exercises |
| --- | --- |
| `openauth-core/src/db/adapter_harness.rs` | Shared contract (used by all 3 dialect tests) |
| `openauth/tests/public_api.rs` | `OpenAuth::run_migrations` + plugin schema + HTTP per dialect |
| `openauth-scim/tests/scim/db_adapters.rs` | ~12 tests: SQLite/Postgres/MySQL + deadpool/tokio variants |
| `openauth-passkey/tests/passkey/sql.rs` | Postgres/MySQL `create_schema` + unique index on passkey table |
| `openauth-plugins/tests/integration_matrix/` | Postgres/MySQL adapters in plugin matrix |
