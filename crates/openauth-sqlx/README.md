# openauth-sqlx

SQLx database adapters for OpenAuth-RS.

## What It Is

`openauth-sqlx` provides OpenAuth `DbAdapter` implementations for SQLite,
Postgres, and MySQL through SQLx. Use it when your application already uses
SQLx or when SQLite is a good fit for local development and small deployments.

For Postgres production deployments that do not otherwise use SQLx,
`openauth-deadpool-postgres` may be a smaller operational fit.

## What It Provides

- `SqliteStores`, `PostgresStores`, `MySqlStores`: bundled adapter +
  SQL-backed rate-limit store sharing one pool (recommended entry point).
- `SqliteAdapter`, `PostgresAdapter`, `MySqlAdapter` behind feature flags.
- Matching `*RateLimitStore` types for BYO-pool setups.
- Schema creation, migration planning, and additive migration execution.
- SQL filter, sort, pagination, and transaction support used by OpenAuth core
  and plugins.

Migration planning types (`SchemaMigrationPlan`, `MigrationStatementKind`, …)
live in `openauth_core::db`, not in this crate.

## Quick Start

```rust
use openauth::{OpenAuth, OpenAuthOptions};
use openauth_core::db::{auth_schema, AuthSchemaOptions, RateLimitStorage};
use openauth_sqlx::SqliteStores;

let schema = auth_schema(AuthSchemaOptions {
    rate_limit_storage: RateLimitStorage::Database,
    ..AuthSchemaOptions::default()
})?;

let stores = SqliteStores::connect_with_schema("sqlite://openauth.db", schema).await?;

let auth = OpenAuth::builder()
    .options(stores.apply_to_options(
        OpenAuthOptions::new().secret("secret-a-at-least-32-chars-long!!"),
    ))
    .adapter(stores.adapter)
    .build()?;

auth.run_migrations().await?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Enable the matching crate feature for your dialect:

```toml
[dependencies]
openauth-sqlx = { version = "0.1.1", features = ["sqlite"] }
```

### BYO pool

When the application already owns a `SqlitePool` / `PgPool` / `MySqlPool`:

```rust
use openauth_sqlx::{SqliteAdapter, SqliteRateLimitStore};

let adapter = SqliteAdapter::with_schema(pool, schema);
let rate_limit = SqliteRateLimitStore::from(&adapter);
```

## Migration Notes

- `run_migrations` applies executable additive plans only.
- Type mismatches, destructive rewrites, renames, and unsafe changes are
  reported as warnings/errors instead of being applied automatically.
- `plan_migrations` and `compile_migrations` let you inspect generated SQL
  before applying it.
- `SqliteAdapter::connect` enables `PRAGMA foreign_keys = ON` on every pooled
  connection. `new(pool)` also enforces foreign keys on each checkout.

### Atomic schema application

`create_schema` and `run_migrations` apply each generated plan as one unit so a
mid-plan failure does not leave a partially applied OpenAuth schema:

- **Postgres and SQLite** run the plan inside a single database transaction.
  Failed statements roll back earlier DDL in that plan.
- **MySQL** cannot roll back DDL through a transaction because MySQL performs
  implicit commits for those statements. The adapter instead undoes successful
  statements in reverse order on failure (for example `DROP TABLE` after a
  failed later `CREATE TABLE`). This is best-effort; treat a failed migration as
  requiring inspection before retrying.

Postgres adapters in `openauth-tokio-postgres` and `openauth-deadpool-postgres`
use the same transactional execution model as `PostgresAdapter`.

## Status

Experimental beta. SQLite has the strongest local coverage. Postgres and MySQL
are covered by integration tests and should be validated against your
production schema and privileges before rollout.

## Better Auth compatibility

Server-side SQL adapter parity for SQLite, Postgres, and MySQL.
Aligned with Better Auth 1.6.9 where it matters; OpenAuth is not a line-by-line port.
For route-level parity, test counts, differences, and gaps, see [UPSTREAM.md](./UPSTREAM.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
