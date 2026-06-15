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

## Plan 019 decision (2026-06-15): SQLite rejected

**Verdict: REJECT** — do not add a `sqlite` feature to `rustauth-diesel`.

### Evaluation summary

1. **SyncConnectionWrapper vs auth workloads** — `diesel-async` SQLite uses
   `SyncConnectionWrapper`, which runs sync Diesel I/O via `tokio::task::spawn_blocking`
   on every query. RustAuth auth paths need concurrent HTTP handlers, transactions, and
   SQL-backed rate limits. SQLx SQLite uses native async pools and passes concurrent
   rate-limit serialization tests; Diesel SQLite would add blocking-thread overhead with
   no latency or throughput win.

2. **Pool / blocking-thread policy** — Postgres/MySQL Diesel adapters use deadpool over
   true async connections. SQLite via `SyncConnectionWrapper` would compete with Tokio's
   blocking pool under burst traffic; no safe, documented pool policy exists and SQLite
   serializes writes regardless.

3. **Deadlock / starvation risk** — Mixing async handlers with per-query `spawn_blocking`
   can exhaust the blocking pool and delay unrelated async work. Upstream diesel-async
   examples use extra `spawn_blocking` for migrations and concurrent transaction loops.

4. **Migration atomicity** — SQLx SQLite applies each migration plan in a single
   transaction (`pool.begin()` → statements → commit). Diesel SQLite could replicate this
   only with additional wrapper work; parity is achievable but costly with zero user upside.

5. **Incremental value over SQLx** — None for realistic scenarios. `rustauth-sqlx` already
   ships production SQLite (CLI migrations, transactional apply, ~1900 lines of adapter
   tests, 39 passing SQLite tests). Diesel-first apps use Postgres/MySQL in production;
   local SQLite via SQLx for the auth slice, or Docker Postgres, is acceptable and documented.

### Recommended user path

- **SQLite (any use case):** `rustauth-sqlx` with `sqlite` feature and `database.adapter = "sqlx"`.
- **Diesel app on Postgres/MySQL:** `rustauth-diesel` with matching feature and CLI provider.
- **Do not** expect `database.adapter = "diesel"` + `provider = "sqlite"` — CLI rejects this
  with `UnsupportedProvider` (see `crates/rustauth-cli/tests/db.rs`).
