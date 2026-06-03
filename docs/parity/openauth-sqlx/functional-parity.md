# Functional parity: openauth-sqlx

Feature matrix against Better Auth **v1.6.9** Kysely SQL + `getMigrations`.
Status legend:

| Status | Meaning |
| --- | --- |
| **Parity** | Observable server behavior matches upstream Kysely path |
| **Partial** | Supported with documented differences |
| **Intentional** | Differs by OpenAuth/Rust design (documented in [design-decisions.md](design-decisions.md)) |
| **N/A** | Out of scope (other adapter, client-only, or not SQL) |
| **Gap** | Not implemented; may be future work |

## AdapterCapabilities flags (reported, not just behavior)

Kysely sets flags in `kysely-adapter.ts` `adapterOptions.config`; OpenAuth sets them in
each dialect’s `capabilities()`.

| Flag | Kysely SQLite | Kysely Postgres | OpenAuth SQLite | OpenAuth Postgres | OpenAuth MySQL |
| --- | --- | --- | --- | --- | --- |
| `supportsBooleans` | false | true | **true** (default) | true | true |
| `supportsDates` | false | true | **true** (default) | true | true |
| `supportsJSON` | false | true | **true** (`with_json`) | true | true |
| `supportsArrays` | false | false | **true** (`with_arrays`) | true | true |
| `supportsUUIDs` | false | true | false | **true** | false |
| `supportsJoins` | via factory | via factory | true | true | true |
| `transaction` | config, default false | config | **always true** | true | true |

MySQL/SQLite still store array fields as JSON text at the wire level (`row.rs`); Postgres
uses native arrays. Flags describe what the **adapter reports**, not identical storage.

## DbAdapter / CRUD

| Capability | Upstream (Kysely + factory) | openauth-sqlx | Status | Notes |
| --- | --- | --- | --- | --- |
| `create` | yes | yes | Parity | MySQL uses post-insert SELECT (upstream same pattern) |
| `findOne` | yes | yes | Parity | |
| `findMany` | yes | yes | Partial | Default limit: upstream factory **100**; OpenAuth **no limit** (core contract) |
| `count` | counts `id` | counts `*` | Parity | Equivalent for auth tables |
| `update` (single row) | yes | yes | Parity | |
| `updateMany` | yes | yes | Parity | |
| `delete` | **all matching rows** | **first matching row** | Intentional | Use `delete_many` for bulk |
| `deleteMany` | yes | yes | Parity | |
| `transaction` | optional per config | yes (all dialects) | Parity | SQLite/Postgres/MySQL pools use real txs |
| Logical → physical table/field names | yes | yes | Parity | |
| `select` field projection | yes | yes | Parity | |
| `offset` / `sortBy` | yes | yes | Parity | |
| WHERE: `eq`, `ne`, comparisons | yes | yes | Parity | |
| WHERE: `in` / `not in` | yes | yes | Parity | |
| WHERE: null checks | yes | yes | Parity | `IS NULL` / `IS NOT NULL` |
| WHERE: `contains`, `starts_with`, `ends_with` | yes | yes | Partial | OpenAuth escapes `%`, `_`, `\` + `ESCAPE` |
| Case-insensitive operators | yes | yes | Parity | `ILIKE` / `LOWER()` per `query-builders.ts` |
| WHERE: `AND` / `OR` connectors | yes | yes | Parity | `Connector` in core `Where` |
| Array field filters | yes | yes | Partial | Storage model differs (see below) |
| `updateMany` / `deleteMany` row count | capped at `MAX_SAFE_INTEGER` | u64 `rows_affected` | Partial | Extreme counts may differ |

## Joins

See **[parity-gaps.md](parity-gaps.md)** for the full `find_many` vs `find_one` analysis.

| Capability | Upstream | openauth-sqlx | Status | Notes |
| --- | --- | --- | --- | --- |
| One-to-one join | yes | yes | Parity | Missing row → `null` |
| One-to-many join | yes | yes | Parity | Missing → `[]`; default join limit **100** (core + fallback) |
| Join `limit` | default 100 | yes | Parity | `resolve_native_joins` / `resolve_fallback_joins` |
| **`find_one` with 2+ joins** | one SQL query | one SQL query | Parity | Runner path; no `joins.len() <= 1` gate |
| **`find_many` with 2+ joins** | one SQL query | one SQL query | Parity | `supports_native_joins`; see [parity-gaps.md](parity-gaps.md) |
| Multi-join inside transaction | one query (Kysely) | one SQL query | Parity | Covered on SQLite, Postgres, MySQL integration tests |
| `experimental.joins` (app default **on**) | N/A | outer `JoinAdapter` | Parity | SQL adapters use native joins via `supports_native_joins`; memory uses fallback when disabled |
| Join column alias edge case (`_JoinedFooBar`) | Kysely workaround | N/A | **Gap** | Different row-shape pipeline |
| Schema-qualified model (`internal.users`) | e2e `schemaRefTestSuite` | Partial | `tokio-postgres` test; sqlx pg + `public_api` `search_path` |

## Schema & migrations

| Capability | Upstream `getMigrations` | openauth-sqlx | Status | Notes |
| --- | --- | --- | --- | --- |
| `toBeCreated` tables | yes | `plan.to_be_created` | Parity | |
| `toBeAdded` columns | yes | `plan.to_be_added` | Parity | |
| Deferred indexes | yes (end of migration list) | `indexes_to_be_created` + ordered `statements` | Parity | |
| `compileMigrations()` SQL string | yes | `compile_migrations()` → `plan.compile()` | Parity | Empty plan → `";"` |
| `runMigrations()` apply | yes | `run_migrations` / `create_schema` | Partial | OpenAuth **aborts** on plan warnings |
| Type mismatch detection | **log warning**, still may run | **warning in plan**, apply blocked | Intentional | Safer additive-only policy |
| Plugin tables / columns / FKs | yes | yes | Parity | Plugin-aware additive diff |
| Missing index repair on existing column | **no** (indexes only on new cols/tables) | yes | **Beyond upstream** | See [openauth-beyond-upstream.md](openauth-beyond-upstream.md) |
| Migration warnings (FK, PK, nullability, generated id) | type log only | 5 kinds + block apply | **Beyond upstream** | core `SchemaMigrationWarning` |
| `run_migrations` in one DB transaction | sequential execute | yes | **Beyond upstream** | `create_schema` apply is **not** wrapped in tx |
| Postgres `search_path` / schema | yes (`getPostgresSchema`) | yes | Parity | Scoped introspection |
| Date column `CURRENT_TIMESTAMP` default in DDL | yes when `defaultValue` is function (pg/mysql/mssql) | not in core DDL planner | **Gap** | Upstream `get-migration.ts`; OpenAuth uses explicit timestamps in app layer |
| External table FK path fallback | `getReferencePath` try/catch | schema must resolve | Partial | Plugin tables must be in `DbSchema` |
| Custom schema file write | via compile + file | `create_schema(Some(path))` | Parity | Writes compiled SQL + metadata |
| Destructive migrations | not in getMigrations | not supported | Parity | Additive-only by design |
| MSSQL CRUD + migrations | yes | — | Gap | No `mssql` feature |
| MSSQL `offset` without `sortBy` | special case in Kysely | n/a | N/A | |

## Types & storage (per dialect)

| Topic | Upstream Kysely migrations | openauth-sqlx | Status |
| --- | --- | --- | --- |
| Postgres arrays | JSONB in migration map | Native `TEXT[]` / `BIGINT[]` | Intentional |
| Postgres `supportsArrays` capability | false in Kysely adapter | **true** on Postgres adapter | Intentional |
| SQLite/MySQL arrays | JSON-like | JSON/text bindings | Partial |
| MySQL timestamps | `timestamp(3)` | `DATETIME(6)` | Intentional |
| UUID primary keys (Postgres) | yes | yes | Parity |
| Serial ids (SQLite/MySQL) | yes | yes | Parity |

## Rate limiting

| Capability | Upstream | openauth-sqlx | Status |
| --- | --- | --- | --- |
| SQL-backed rate limit table | yes | `*RateLimitStore` | Parity |
| Physical column names from schema | yes | yes | Parity |
| Denied request must not increment | split read/write timing | single tx `consume` | Intentional | Rust `RateLimitStore` contract |
| Negative count rejection | — | SQLite test | Intentional | Extra validation |

## Connection / runtime

| Capability | Upstream `createKyselyAdapter` | openauth-sqlx | Status |
| --- | --- | --- | --- |
| Postgres pool | pg / Kysely | `sqlx::PgPool` | Parity |
| MySQL pool | mysql2 | `sqlx::MySqlPool` | Parity |
| SQLite file/memory | multiple dialect shims | `sqlx::SqlitePool` | Partial | No D1/Bun/node:sqlite shims |
| SQLite `foreign_keys` | app responsibility | `connect` sets PRAGMA | Intentional | Documented in README |
| MSSQL | yes | — | Gap |
| Cloudflare D1 | yes (sqlite) | — | N/A | Server deployment model |

## Factory-only behavior (lives in core, not sqlx)

These are **not** implemented inside `openauth-sqlx` but affect parity when comparing full stacks:

| Upstream factory behavior | OpenAuth |
| --- | --- |
| Default field values / `onUpdate` in adapter | Set in service layer before adapter calls |
| `generateId` in adapter | Explicit ids / DB-generated ids in routes |
| `usePlural` table names | `AuthSchemaOptions` in core |
| Debug SQL logs | Not in sqlx crate |

## ORM adapters (explicit non-goals)

| Upstream package | openauth-sqlx |
| --- | --- |
| Drizzle CRUD + schema push | **N/A** — use SQLx adapter or separate future crate |
| Prisma Client | **N/A** |
| MongoDB | **N/A** — would be `openauth-mongo` pattern if added |

## Extended API (OpenAuth-only on adapter type)

Methods on `SqliteAdapter` / `PostgresAdapter` / `MySqlAdapter` **not** on `DbAdapter`:

| Method | Upstream equivalent | Purpose |
| --- | --- | --- |
| `plan_migrations(&DbSchema)` | `getMigrations` inspection | Inspect `to_be_*` + warnings before apply |
| `compile_migrations(&DbSchema)` | `compileMigrations()` | Get SQL string without executing |

These are re-exported migration types from `openauth_sqlx::migration` for CLI and tooling.
