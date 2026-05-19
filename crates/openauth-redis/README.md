# openauth-redis

Redis and Valkey integrations for OpenAuth-RS using `redis-rs`.

## Status

This package is in experimental beta. URL handling, key layout, Lua script
behavior, and rate-limit contracts may change before stable release.

## What It Provides

`openauth-redis` provides a distributed `RateLimitStore` backed by Redis or
Valkey through `redis-rs`. It uses Lua scripting for atomic consume decisions
and accepts `valkey://` and `valkeys://` aliases.

## Example

```rust
use openauth::{OpenAuth, RateLimitOptions};
use openauth_redis::RedisRateLimitStore;

let store = RedisRateLimitStore::connect("redis://127.0.0.1:6379").await?;

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

Use this crate when your application already uses `redis-rs`; use
`openauth-fred` when you prefer the `fred` client.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
