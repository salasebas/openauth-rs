# openauth-sqlx

SQLx database adapters for OpenAuth-RS.

## Status

This package is in experimental beta. SQL planning, migration output, feature
flags, and adapter behavior may change before stable release.

## What It Provides

`openauth-sqlx` provides SQLite, Postgres, and MySQL adapters for OpenAuth-RS,
plus SQL-backed rate-limit stores. Use the crate feature matching your database:
`sqlite`, `postgres`, or `mysql`.

## Example

```rust
use openauth::OpenAuth;
use openauth_sqlx::SqliteAdapter;
use sqlx::sqlite::SqlitePoolOptions;

let pool = SqlitePoolOptions::new()
    .connect("sqlite://openauth.db")
    .await?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .adapter(SqliteAdapter::new(pool))
    .build()?;
```

For Postgres production deployments that do not otherwise use SQLx,
`openauth-deadpool-postgres` may be the smaller operational fit.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
