# openauth-axum

Axum integration for OpenAuth-RS.

## Status

This package is in experimental beta. Router composition, request extraction,
and adapter options may change before stable release.

## What It Provides

`openauth-axum` mounts the framework-neutral OpenAuth HTTP core into an Axum
application. It provides a ready-to-use router, route helpers for manual
composition, and an adapter-specific request body limit.

Use `router(auth)` for the common case. It reads `OpenAuthOptions::base_path`
from the auth instance, defaults to `/api/auth`, and returns an Axum `Router`
that can be merged into the rest of the application.

Use `routes(auth)` when the application already owns the mount point and wants
to nest OpenAuth manually. The returned routes are unmounted catch-all auth
routes, so nest them at the same path configured in OpenAuth, for example
`Router::new().nest("/api/auth", routes(auth))`.

Use `handle_ref(&auth, request)` or `handle_ref_with_options` for custom Axum
integration code that wants to adapt a single `Request<Body>` without building
a router. These helpers preserve request and response metadata, including
headers, HTTP version, and extensions.

Use `OpenAuthAxumOptions` for adapter-only behavior such as body limits or
whether Axum `ConnectInfo<SocketAddr>` should be copied into OpenAuth's
framework-neutral request extensions.

## Example

```rust
use openauth::OpenAuth;
use openauth_axum::{router_with_options, OpenAuthAxumOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .build()?;

let app = router_with_options(
    auth,
    OpenAuthAxumOptions::new().body_limit(1024 * 1024),
)?;
```

When Axum is exposed directly to clients, run the service with connection info
so OpenAuth can use the real peer socket IP for rate limiting.

```rust
use std::net::SocketAddr;

use axum::serve;
use tokio::net::TcpListener;

# async fn run(app: axum::Router) -> Result<(), Box<dyn std::error::Error>> {
let listener = TcpListener::bind("127.0.0.1:3000").await?;
serve(
    listener,
    app.into_make_service_with_connect_info::<SocketAddr>(),
)
.await?;
# Ok(())
# }
```

By default this adapter copies Axum `ConnectInfo<SocketAddr>` into OpenAuth's
framework-neutral request extensions. That gives rate limiting a real socket IP
without trusting spoofable request headers. If the application runs behind a
trusted reverse proxy, configure OpenAuth's `advanced.ip_address` header list
explicitly and terminate untrusted traffic at the proxy boundary. Do not trust
`x-forwarded-for` from direct public clients.

Request bodies are collected before entering OpenAuth core and are capped at
10 MiB by default. Use `OpenAuthAxumOptions::body_limit` to lower this for
public deployments. Oversized requests are rejected by the adapter with a
stable JSON `413 Payload Too Large` response before auth handlers run.

## Storage Smoke Testing

The Axum package keeps storage logic in `openauth-core` and adapter crates.
Its always-on smoke coverage uses `MemoryAdapter`; SQL and Redis/Valkey
contracts live in their concrete adapter crates and can be exercised with the
repository `docker-compose.yml`. MongoDB is present in Docker Compose for
ecosystem parity, but this workspace does not yet ship a Rust MongoDB adapter.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
