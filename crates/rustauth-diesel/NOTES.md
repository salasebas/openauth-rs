# Plan 013 implementation decision

## Crate naming and scope

- Public crate name: `rustauth-diesel`
- Async-only on `diesel-async` (no sync Diesel adapter)
- Dependency line: Diesel `2.3`, diesel-async `0.9`
- Initial backends: Postgres (`postgres` feature) and MySQL (`mysql` feature)
- SQLite deferred: `diesel-async` SQLite uses a sync wrapper; RustAuth already ships SQLx SQLite

## Parameter binding

Dynamic `SqlParam` values bind through Diesel's `sql_query(...).into_boxed()` API.
Each value uses typed `.bind::<ST, _>(...)` — no SQL string interpolation.

Supported mappings mirror `rustauth-sqlx`:

- Postgres: text, bigint, bool, timestamptz, jsonb, text[], bigint[], uuid ids
- MySQL: text, bigint, bool, timestamp, json; string/number arrays bind as JSON

## Row decoding

**Chosen:** direct alias decoding with `NamedRow::get` for each `SqlSelectedField.alias`.

**Rejected:** JSON payload projection (`jsonb_build_object` / `JSON_OBJECT`). Diesel can
decode aliased raw SQL rows by column name, so a JSON wrapper would add query rewriting
without improving dynamic projection support.

## Pooling shape

Public types use `diesel_async::pooled_connection::deadpool::Pool`:

- `DieselPostgresAdapter` / `DieselMysqlAdapter`
- `DieselPostgresStores` / `DieselMysqlStores`
- `DieselPostgresRateLimitStore` / `DieselMysqlRateLimitStore`

No sync connection types are exposed.

## Gate status

Feasibility tests in `tests/diesel_feasibility.rs` prove binding and alias decoding for
both backends. Full CRUD adapter work belongs to plans 014–015.
