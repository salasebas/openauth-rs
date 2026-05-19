# openauth-deadpool-postgres

Pooled Postgres database adapter for OpenAuth-RS.

## Status

This package is in experimental beta. Pool configuration, migration behavior,
and adapter contracts may change before stable release.

## What It Provides

`openauth-deadpool-postgres` is the recommended Postgres adapter for production
deployments that want pooling without taking a SQLx dependency. It uses
`deadpool-postgres` for pooling and reuses OpenAuth-RS shared SQL planning.

## Example

```rust
use openauth::OpenAuth;
use openauth_deadpool_postgres::DeadpoolPostgresAdapter;

let adapter = DeadpoolPostgresAdapter::connect(
    "postgres://user:password@localhost:5432/openauth",
)
.await?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .adapter(adapter)
    .build()?;
```

Use `DeadpoolPostgresRateLimitStore::from(&adapter)` when you want the same
database to provide distributed rate limiting.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
