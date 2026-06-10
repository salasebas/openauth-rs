# openauth-redis

Redis and Valkey integrations for OpenAuth-RS using `redis-rs`.

## What It Is

`openauth-redis` provides Redis-compatible backing stores for OpenAuth rate
limiting and secondary key-value storage. Use it when your application already
uses `redis-rs` or wants a small Redis integration.

Use `openauth-fred` instead when your application standardizes on the `fred`
client.

## What It Provides

- `RedisOpenAuthStores`: one shared `ConnectionManager` for rate limiting and
  secondary storage (recommended entry point).
- `RedisRateLimitStore`: distributed atomic rate limiting through Lua.
- `RedisSecondaryStorage`: secondary storage for sessions, verification state,
  SSO state, and plugin data that opt into secondary storage.
- `list_keys()` / `clear()` on secondary storage (`SCAN`, matching `openauth-fred`).
- `redis://`, `rediss://`, `valkey://`, and `valkeys://` URL support. Valkey
  schemes are normalized internally. TLS schemes (`rediss://`, `valkeys://`)
  require enabling a TLS feature; see [TLS](#tls).

## Quick Start

```rust
use openauth::{OpenAuth, OpenAuthOptions};
use openauth_redis::RedisOpenAuthStores;

let stores = RedisOpenAuthStores::connect("redis://127.0.0.1:6379").await?;

let auth = OpenAuth::builder()
    .options(stores.apply_to_options(
        OpenAuthOptions::new().secret("secret-a-at-least-32-chars-long!!"),
    ))
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`apply_to_options` wires both `secondary_storage` and distributed rate limiting
in one call.

### Individual stores

```rust
use openauth::{OpenAuth, OpenAuthOptions, RateLimitOptions};
use openauth_redis::RedisRateLimitStore;
use std::sync::Arc;
use openauth_redis::RedisSecondaryStorage;

let store = RedisRateLimitStore::connect("redis://127.0.0.1:6379").await?;
let storage = RedisSecondaryStorage::connect("redis://127.0.0.1:6379").await?;

let auth = OpenAuth::builder()
    .options(
        OpenAuthOptions::new()
            .secret("secret-a-at-least-32-chars-long!!")
            .secondary_storage(Arc::new(storage))
            .rate_limit(RateLimitOptions::secondary_storage(store).enabled(true)),
    )
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## TLS

TLS connections are opt-in. `rediss://` and `valkeys://` URLs only work when a
redis-rs TLS backend is compiled in through one of these crate features:

```toml
# rustls backend (pure Rust)
openauth-redis = { version = "0.1.1", features = ["rustls"] }

# or native-tls backend (system TLS)
openauth-redis = { version = "0.1.1", features = ["native-tls"] }
```

Without a TLS feature, opening a `rediss://` or `valkeys://` URL fails with an
`InvalidClientConfig` error because the TLS backend is not enabled.

## Status

Experimental beta. URL handling, key layout, Lua script behavior, and storage
contracts may change before stable release.

## Better Auth compatibility

Server-side Redis/Valkey secondary storage and rate limiting via `redis-rs`.
Aligned with Better Auth **1.6.9** where it matters for this crate; OpenAuth is
not a line-by-line port.

For route-level parity, test counts, intentional differences, and known gaps, see
[UPSTREAM.md](./UPSTREAM.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
