# OpenAuth behavior beyond Better Auth Kysely

Features verified in **source** that go past what `getMigrations` + Kysely adapter do in
v1.6.9. These are not “missing parity”; they are deliberate extensions.

## Migration planning and safety

| Feature | Upstream | OpenAuth |
| --- | --- | --- |
| Missing index on **existing** column | Indexes only when adding new columns/tables | `indexes_to_be_created` + tests `*_repairs_missing_indexes_on_existing_columns` |
| Block apply on schema drift | Type mismatch: **log** and continue planning | `SchemaMigrationWarning` enum + `ensure_executable_migration_plan` blocks `run_migrations` / `create_schema` |
| Warning kinds | Type mismatch (logged) | Type, nullability, primary key, generated id, **foreign key** mismatch |
| `run_migrations` atomicity | Sequential `execute()` per statement | Single SQLx **transaction** per `run_migrations` (all dialects) |
| Structured plan API | `toBeCreated` / `toBeAdded` on return object | `SchemaMigrationPlan` with `indexes_to_be_created`, `warnings`, `statements`, `compile()` |

## SQL execution

| Feature | Upstream Kysely `delete` | OpenAuth |
| --- | --- | --- |
| Delete scope | All matching rows | **One** row (`delete_many` for bulk) |

| Feature | Upstream LIKE patterns | OpenAuth |
| --- | --- | --- |
| Wildcards in filter values | Unescaped `%` / `_` | Escaped + `ESCAPE` clause (core `sql/dialect.rs`) |

## Adapter capabilities reporting

Kysely adapter sets (from `kysely-adapter.ts`):

- SQLite: `supportsBooleans: false`, `supportsDates: false`, `supportsJSON: false`, `supportsArrays: false`
- Postgres: `supportsJSON: true`, `supportsUUIDs: true`, `supportsArrays: false`
- `transaction`: only if `config.transaction` is enabled

OpenAuth SQLx adapters report (from `*/mod.rs`):

- SQLite/Postgres/MySQL: `with_json()`, `with_arrays()`, `with_joins()`, `with_transactions()` (always on)
- Postgres: additionally `with_uuid_ids()`
- Default `supports_booleans` / `supports_dates`: **true** unless stripped

Runtime behavior supports booleans/dates on SQLite; flags are **more permissive** than Kysely’s
reported capabilities. Anything consuming `AdapterCapabilities` from the adapter should treat
this as an OpenAuth/Rust contract, not a copy of Kysely flags.

## Testing investment

| Area | Upstream Kysely package tests | OpenAuth |
| --- | --- | --- |
| Package-local | 1 smoke test | 0 unit tests in `src/` |
| Full contract | `e2e/adapter/test/kysely-adapter` + test-utils (~7 suites × ~93 cases) | Custom integration tests + smaller `run_adapter_contract` harness |
| Plugin migration + HTTP | Via e2e auth-flow suite | `public_api.rs` + dialect integration tests |
