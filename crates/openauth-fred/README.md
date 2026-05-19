# openauth-fred

Redis and Valkey integrations for OpenAuth-RS using `fred`.

## Status

This package is in experimental beta. URL handling, key layout, Lua script
behavior, and rate-limit contracts may change before stable release.

## What It Provides

`openauth-fred` provides the same distributed `RateLimitStore` contract as the
Redis integration, but through the `fred` client. It supports Redis and Valkey
URLs and uses Lua scripting for atomic consume decisions.

## Example

```rust
use openauth::{OpenAuth, RateLimitOptions};
use openauth_fred::FredRateLimitStore;

let store = FredRateLimitStore::connect_valkey("valkey://127.0.0.1:6379").await?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .rate_limit(
        RateLimitOptions::secondary_storage(store)
            .enabled(true)
            .window(60)
            .max(100),
    )
    .build()?;
```

Use this crate when your application already uses `fred`; use `openauth-redis`
when you prefer `redis-rs`.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
