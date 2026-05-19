# openauth-tokio-postgres

Minimal `tokio-postgres` database adapter for OpenAuth-RS.

## Status

This package is in experimental beta. Adapter behavior, migration planning, and
rate-limit store contracts may change before stable release.

## What It Provides

`openauth-tokio-postgres` is useful when an application already owns a
`tokio_postgres::Client` or wants the smallest async Postgres adapter. It is not
a pool; production applications that need pooling should usually prefer
`openauth-deadpool-postgres`.

## Example

```rust
use openauth::OpenAuth;
use openauth_tokio_postgres::TokioPostgresAdapter;

let adapter = TokioPostgresAdapter::connect(
    "postgres://user:password@localhost:5432/openauth",
)
.await?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .adapter(adapter)
    .build()?;
```

Use `TokioPostgresRateLimitStore::from(&adapter)` when a single client should
also back rate limiting.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
