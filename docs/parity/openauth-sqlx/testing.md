# Testing parity: openauth-sqlx

Counts and coverage derived from **test source files**, not README claims.

## Upstream: where Kysely is actually tested

| Layer | Location | Scale |
| --- | --- | --- |
| Package smoke | `packages/kysely-adapter/src/kysely-adapter.test.ts` | **1** `it` (adapter constructs) |
| Migration unit | `packages/better-auth/src/db/get-migration-schema.test.ts` | **~10** `it` (Postgres schema / `search_path`) |
| DB wiring | `packages/better-auth/src/db/db.test.ts` | **~7** `it` |
| Shared contract definitions | `packages/test-utils/src/adapter/suites/` | **~93** object-key tests total |
| **Primary Kysely parity** | `e2e/adapter/test/kysely-adapter/*.ts` | Runs test-utils suites per database |

### test-utils suite sizes (object-key tests)

| Suite file | Cases |
| --- | --- |
| `basic.ts` | 71 |
| `case-insensitive.ts` | 10 |
| `auth-flow.ts` | 5 |
| `uuid.ts` | 3 |
| `number-id.ts` | 2 |
| `joins.ts` | 1 |
| `transactions.ts` | 1 |

### Kysely e2e files (v1.6.9)

| File | Suites typically included |
| --- | --- |
| `adapter.kysely.sqlite.test.ts` | normal, transactions, authFlow, numberId, joins, uuid, caseInsensitive |
| `adapter.kysely.mysql.test.ts` | same pattern |
| `adapter.kysely.pg.test.ts` | above + **schemaRef** + **schemaRefJoin** |
| `adapter.kysely.custom-schema-pg.test.ts` | custom `search_path` schema |
| `adapter.kysely.mssql.test.ts` | MSSQL (no OpenAuth equivalent) |
| `node-sqlite-dialect.test.ts` | Node sqlite shim |
| `schema-reference-test-suite.ts` | `internal.users`-style model names |

Postgres e2e notably sets `transactionsTestSuite({ disableTests: { ALL: true } })` while still
importing the suite — transaction behavior is not fully exercised there via test-utils.

## OpenAuth: `openauth-sqlx` crate tests

| File | `#[tokio::test]` | `#[test]` | Notes |
| --- | --- | --- | --- |
| `tests/sqlite_adapter.rs` | 34 | 0 | Default CI path; in-memory |
| `tests/postgres_adapter.rs` | 23 | 2 | URL env / defaults |
| `tests/mysql_adapter.rs` | 23 | 2 | URL env / defaults |
| **Crate total** | **80** | **4** | No `#[cfg(test)]` in `src/` |

Default adapter setup differences (from test helpers):

- **SQLite:** `RateLimitStorage::Database`, `create_schema` in `adapter()` helper.
- **Postgres/MySQL:** `unique_prefix()` table isolation on shared Docker DB; `test_pool(5)`.

### Shared harness (all dialects)

Each dialect calls once:

`openauth_core::db::adapter_harness::run_adapter_contract`

| Phase | Validates |
| --- | --- |
| Filters / sort / limit / count | Insensitive `ends_with`, `limit(1)`, `count` |
| Updates / deletes | `update`, `update_many`, **`delete`** (single), `delete_many` |
| Transactions | commit + rollback when `supports_transactions` |
| Joins | `find_one` + `account` join when `supports_joins` |

This is a **subset** of upstream `basic.ts` (71 cases), not the full e2e matrix.

### Per-dialect integration themes

| Theme | SQLite | Postgres | MySQL |
| --- | --- | --- | --- |
| `run_adapter_contract` | yes | yes | yes |
| LIKE literal wildcards | yes | yes | yes |
| `create_schema` writes file | yes | yes | yes |
| `plan_migrations` / `compile_migrations` | yes | yes | yes |
| Plugin column + table + index migrations | yes | yes | yes |
| Missing index repair | yes | yes | yes |
| Type mismatch → warning, no DDL | yes | yes | yes |
| FK mismatch warning | **yes** | no dedicated test | no dedicated test |
| `create_schema` idempotent + rate_limits table | **yes** | no | no |
| Reject apply when warnings (`run_migrations` / `create_schema`) | **yes** | no | no |
| `PRAGMA foreign_keys` on connect | **yes** | n/a | n/a |
| Rate limit: deny without increment | **yes** | no | no |
| Rate limit: reject negative count | **yes** | no | no |
| `HookedAdapter` + transaction hooks | **yes** | no | no |
| Multi-join inside transaction | **yes** | no | no |
| WHERE: `gte`, `in`, insensitive `contains` | yes (on `rate_limit`) | yes | yes |
| Joins forward/reverse/limited | yes | yes | yes |
| HTTP: sign-up, session, sign-out, sign-in, list/revoke sessions | yes | yes | yes |
| Password reset verification flow | yes | yes | yes |
| DB-generated ids | serial | uuid | serial |
| Custom physical table/column names | yes | yes | yes |
| `create_schema` + rate_limit table (postgres test) | — | partial (`create_schema_writes…`) | — |

### Cross-dialect parity tests (SQLite, Postgres, MySQL)

- `*_adapter_plan_migrations_warns_for_foreign_key_mismatch`
- `*_rate_limit_store_denies_without_incrementing_denied_requests`
- `*_rate_limit_store_rejects_negative_persisted_counts`
- `*_hooked_adapter_preserves_native_transaction_rollback`
- `*_hooked_adapter_runs_after_hooks_after_native_transaction_commit`
- `*_adapter_supports_multi_joins_inside_transactions`

### SQLite-only tests (5)

1. `sqlite_connect_enables_foreign_keys_for_pooled_connections`
2. `sqlite_adapter_create_schema_is_idempotent_and_creates_rate_limit_table`
3. `sqlite_adapter_run_migrations_applies_plugin_aware_schema`
4. `sqlite_adapter_run_migrations_rejects_type_warnings_without_applying_statements`
5. `sqlite_adapter_create_schema_rejects_type_warnings_without_applying_statements`

### What integration tests assert on HTTP (all dialects)

From `*_supports_core_auth_route_flows`:

- `POST /api/auth/sign-up/email` → 200
- `GET /api/auth/get-session` → 200
- `POST /api/auth/sign-out` → 200
- `POST /api/auth/sign-in/email` → 200
- `POST /api/auth/update-session` with `{}` → **400**
- `GET /api/auth/list-sessions` → 200
- `POST /api/auth/revoke-other-sessions` → 200
- `POST /api/auth/revoke-session` → 200

Password reset tests cover verification identifier flow per dialect.

## Cross-crate tests using `openauth-sqlx`

| Crate / file | Count (approx.) | Focus |
| --- | --- | --- |
| `openauth/tests/public_api.rs` | 3 migration+HTTP tests | `OpenAuth::run_migrations` + plugin field + sign-up (sqlite, postgres w/ isolated schema, mysql) |
| `openauth-scim/tests/scim/db_adapters.rs` | 12 `#[tokio::test]` | SCIM tables via `SqliteAdapter` / `PostgresAdapter` / `MySqlAdapter` (+ tokio/deadpool postgres) |
| `openauth-passkey/tests/passkey/sql.rs` | 2 `#[ignore]` e2e | Unique index on passkey `credential_id` (Postgres/MySQL `create_schema`) |
| `openauth-plugins/tests/integration_matrix/` | matrix | Postgres/MySQL adapters with plugins |

Postgres **schema-qualified** table names (`internal.users`) are covered in
`openauth-tokio-postgres/tests/postgres_adapter.rs`, not in `openauth-sqlx` tests —
see [functional-parity.md](functional-parity.md) gap row.

## Shared SQL planner tests (`openauth-core`)

`crates/openauth-core/tests/db/sql.rs` contains **25** unit tests for dialect SQL,
migration planning, join row grouping, rate-limit statements, and delete-one plans.
These apply to **all** SQL adapters including sqlx; they are not duplicated inside
`openauth-sqlx/tests/`.

## Parity assessment (testing)

| Dimension | Upstream Kysely | OpenAuth sqlx |
| --- | --- | --- |
| Full test-utils contract per DB | yes (`e2e/adapter/test/kysely-adapter/`) | **no** — harness + custom tests |
| Multi-join `find_many` same SQL as Kysely | yes | **no** — fallback path; see [parity-gaps.md](parity-gaps.md) |
| Organization plugin models in adapter tests | in `basic.ts` | **not** in sqlx tests |
| Dedicated case-insensitive suite | 10 cases | partial (harness + 1 operator test) |
| MSSQL | e2e file | none |
| Schema-reference (`internal.*`) e2e | pg e2e | tokio-postgres only |
| Migration warning / index repair | not tested upstream same way | **explicit** sqlx tests |
| Plugin additive migrations + HTTP | via migrations in e2e | sqlx + `public_api` |

## Commands

```bash
cargo nextest run -p openauth-sqlx

OPENAUTH_TEST_POSTGRES_URL=postgres://user:password@localhost:5432/openauth \
  cargo nextest run -p openauth-sqlx --no-default-features --features postgres

OPENAUTH_TEST_MYSQL_URL=mysql://user:password@localhost:3306/openauth \
  cargo nextest run -p openauth-sqlx --no-default-features --features mysql

cargo test -p openauth-sqlx --all-features --no-run
```

Upstream e2e (requires their Docker services):

```bash
# From Better Auth clone — not run in OpenAuth CI by default
cd reference/upstream-src/1.6.9/repository/e2e/adapter
# See package scripts / vitest config for kysely-adapter project
```
