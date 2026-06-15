# Upstream Parity: rustauth-diesel

| Field | Value |
| --- | --- |
| Parity pin | Better Auth `1.6.9` (`reference/upstream-better-auth/VERSION.md`) |
| Upstream package/path | `@better-auth/kysely-adapter` at `reference/upstream-src/1.6.9/repository/packages/kysely-adapter/` |
| Rust crate | `rustauth-diesel` |
| Parity level | High for Postgres SQL adapter contract (mirrors `rustauth-sqlx` Postgres surface) |
| Scope | Async Diesel Postgres adapter: CRUD, joins, transactions, migrations, plugin migrations, SQL rate limits |

`rustauth-diesel` maps Better Auth's Kysely Postgres runtime behavior onto
`diesel-async` with the same shared SQL planning layer as `rustauth-sqlx`.

Status symbols are defined in the [parity index](../../docs/parity/README.md#status-symbols).

## Feature Parity (Postgres)

| Area | Status | Notes |
| --- | --- | --- |
| Adapter export | ✅ | `DieselPostgresAdapter`, `DieselPostgresRateLimitStore`, `DieselPostgresStores` behind `postgres` feature |
| CRUD operations | ✅ | Shared `SqlAdapterRunner` + dynamic `DieselPostgresRow` decoding |
| Filters and sorting | ✅ | Same SQL planner as SQLx |
| Joins | ✅ | Native joins via shared planner |
| Transactions | ✅ | Pooled connection held across callback; `BEGIN`/`COMMIT`/`ROLLBACK` via `batch_execute` |
| Schema creation / migrations | ✅ | Catalog introspection via typed `QueryableByName` rows |
| Rate-limit storage | 🎯 | Transactional consume; mirrors SQLx semantics |
| MySQL | ➖ | Plan 015 |
| SQLite | ➖ | Deferred (SQLx SQLite adapter covers this) |

## Test Parity

Integration tests in `tests/postgres_adapter.rs` are ported from `rustauth-sqlx`
with Diesel pool connect and SQLx used only for independent verification queries.
