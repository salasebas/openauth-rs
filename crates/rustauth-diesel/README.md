# rustauth-diesel

Diesel database adapters for RustAuth.

This crate is the async-only Diesel integration for RustAuth. It builds on
[`diesel-async`](https://docs.rs/diesel-async) deadpool pooling and the shared
SQL runner in `rustauth-core`.

## Features

- `postgres` — production Postgres [`DbAdapter`](https://docs.rs/rustauth-core/latest/rustauth_core/db/trait.DbAdapter.html), schema migrations, plugin migrations, SQL-backed rate limits, and `DieselPostgresStores`
- `mysql` — feasibility stubs only (full adapter in plan 015)

SQLite and sync Diesel are intentionally out of scope for the first rollout.

## Postgres adapter

```rust
use rustauth_diesel::DieselPostgresAdapter;

let adapter = DieselPostgresAdapter::connect("postgres://user:password@localhost:5432/rustauth").await?;
```

Bundled adapter + rate-limit store:

```rust
use rustauth_diesel::DieselPostgresStores;

let stores = DieselPostgresStores::connect("postgres://user:password@localhost:5432/rustauth").await?;
let options = stores.apply_to_options(rustauth_core::options::RustAuthOptions::default());
```

Adapter id: `diesel-postgres`.

## Row decoding

Dynamic query results use [`DieselPostgresRow`](src/postgres/row.rs): a
[`QueryableByName`](https://docs.rs/diesel/latest/diesel/deserialize/trait.QueryableByName.html)
type that captures column bytes and type OIDs at build time, then decodes through
the shared [`SqlRowReader`](https://docs.rs/rustauth-core/latest/rustauth_core/db/trait.SqlRowReader.html)
boundary via `tokio_postgres` [`FromSql`](https://docs.rs/tokio-postgres/latest/tokio_postgres/types/trait.FromSql.html).

See [NOTES.md](./NOTES.md) for the plan 013 feasibility decision record.

## Tests

```bash
./scripts/ensure-test-services.sh postgres
cargo nextest run -p rustauth-diesel --features postgres --test diesel_feasibility
cargo nextest run -p rustauth-diesel --features postgres --test postgres_adapter
```

For route-level parity, test counts, differences, and gaps, see [UPSTREAM.md](./UPSTREAM.md).
