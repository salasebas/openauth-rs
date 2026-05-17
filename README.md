# OpenAuth

OpenAuth is a Rust authentication toolkit.

## Status

OpenAuth is experimental. APIs, crate boundaries, and behavior may change before
the project reaches a stable release.

## Rate Limiting

OpenAuth rate limiting is route-aware and uses an async atomic consume contract.
The default `Memory` backend is a Governor-backed local limiter and is best for
development, tests, and single-instance deployments.

```rust
use openauth::{OpenAuth, RateLimitOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .rate_limit(RateLimitOptions::memory().enabled(true).window(60).max(100))
    .build()?;
```

For multi-instance deployments, use a distributed `RateLimitStore` instead of
local memory. `openauth-sqlx` provides SQLx-backed stores when the application
already depends on a SQL database. For Postgres without SQLx,
`openauth-deadpool-postgres` is the recommended pooled production adapter, while
`openauth-tokio-postgres` is the minimal no-pool adapter for applications that
already own a `tokio_postgres::Client`. `openauth-redis` provides a Redis/Valkey
store for higher-throughput shared enforcement. `openauth-fred` provides the
same rate-limit storage contract for projects that prefer the `fred` client.
Both Redis crates use RESP-compatible servers, Lua scripting for the atomic
consume operation, and core commands shared by Redis and Valkey. `valkey://`
and `valkeys://` URLs are accepted as aliases for `redis://` and `rediss://`
before connecting. Very high traffic deployments can opt into hybrid mode,
which runs a local Governor prefilter before the SQLx or Redis/Valkey store
while keeping the distributed decision authoritative.

```rust
use openauth::{HybridRateLimitOptions, OpenAuth, RateLimitOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .rate_limit(
        RateLimitOptions::secondary_storage(redis_store)
            .window(60)
            .max(100)
            .hybrid(HybridRateLimitOptions::enabled().local_multiplier(2)),
    )
    .build()?;
```

Using Fred directly:

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

Custom stores should implement `RateLimitStore::consume` atomically. The legacy
`RateLimitStorage` `get`/`set` adapter is kept for compatibility, but it is not
safe for distributed enforcement unless the underlying implementation provides
its own atomicity.

The existing initializer helpers remain available:

```rust
use openauth::{open_auth, OpenAuthOptions};

let auth = open_auth(OpenAuthOptions::new()
    .secret("secret-a-at-least-32-chars-long!!"))?;
```

## Axum

`openauth-axum` mounts the framework-neutral OpenAuth HTTP core in an Axum
application. It uses a catch-all route under the configured auth base path, so
core routes, custom endpoints, and plugin-provided endpoints are all handled by
OpenAuth's router.

```rust
use openauth::OpenAuth;
use openauth_axum::router;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .build()?;

let app = router(auth)?;
```

When Axum is exposed directly to clients, run the app with connection info so
OpenAuth can use the real peer socket IP for rate limiting:

```rust
let app = openauth_axum::router(auth)?;

axum::serve(
    listener,
    app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
).await?;
```

When OpenAuth is behind a trusted proxy or load balancer, configure the proxy
to overwrite forwarded IP headers and configure OpenAuth to trust only those
headers. Do not enable this for traffic that can reach Axum directly, because
clients can spoof forwarded headers.

```rust
use openauth::{AdvancedOptions, IpAddressOptions, OpenAuthOptions};

let options = OpenAuthOptions::new().advanced(
    AdvancedOptions::new().ip_address(
        IpAddressOptions::new().headers(["x-forwarded-for"])
    )
);
```

For manual composition, nest the unmounted routes at the same path as
`OpenAuthOptions.base_path`:

```rust
use axum::Router;
use openauth_axum::OpenAuthAxumExt;

let app = Router::new().nest("/api/auth", auth.into_routes());
```

The adapter has its own request body limit. The default is 10 MiB and can be
overridden without changing core OpenAuth options. Requests that exceed this
limit return `413 Payload Too Large`.

```rust
use openauth_axum::{router_with_options, OpenAuthAxumOptions};

let app = router_with_options(
    auth,
    OpenAuthAxumOptions::new().body_limit(2 * 1024 * 1024),
)?;
```

## License

OpenAuth is licensed under the MIT License. See [LICENSE](LICENSE).
