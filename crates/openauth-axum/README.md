# openauth-axum

Axum adapter for OpenAuth-RS.

## What It Is

`openauth-axum` mounts the framework-neutral OpenAuth handler into an Axum
application. Use it when your server is built with Axum and you want OpenAuth
routes under a path such as `/api/auth`.

## What It Provides

- [`OpenAuthAxumExt`](crate::OpenAuthAxumExt) — `into_router`, `into_router_with`,
  `into_routes`, and `into_routes_with` for mounting.
- [`handle`](crate::handle) and [`handle_with_options`](crate::handle_with_options) —
  escape hatches for custom wiring.
- [`OpenAuthAxumOptions`](crate::OpenAuthAxumOptions) — request body limits, ConnectInfo
  propagation, and optional base URL inference behind explicit proxy trust.
- Request and response conversion that preserves headers, extensions, and HTTP
  metadata.

## Quick Start

```rust
use openauth::prelude::*;
use openauth_axum::OpenAuthAxumExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = OpenAuth::builder()
        .secret("secret-a-at-least-32-chars-long!!")
        .base_url("https://app.example.com/api/auth")
        .build()
        .await?;

    auth.run_migrations().await?;

    let app = auth.into_router_with(OpenAuthAxumOptions::new().body_limit(1024 * 1024))?;
    # let _ = app;
    Ok(())
}
```

When Axum is exposed directly to clients, run it with connection info so
OpenAuth can use the real peer socket IP for rate limiting:

```rust
use std::net::SocketAddr;
use axum::serve;
use tokio::net::TcpListener;

# async fn run(app: axum::Router) -> Result<(), Box<dyn std::error::Error>> {
let listener = TcpListener::bind("127.0.0.1:3000").await?;
serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
# Ok(())
# }
```

## Notes

- Default mount path comes from `OpenAuthOptions::base_path`, falling back to
  `/api/auth`.
- `base_path("/")` and `base_path("")` mount OpenAuth routes at the application
  root.
- Request bodies are collected before core and capped at 10 MiB by default.
- Configure `OpenAuthOptions::base_url` for production deployments. If you
  intentionally need request-derived public URLs, enable
  `OpenAuthAxumOptions::infer_base_url_from_request(true)` and configure
  trusted origins explicitly.
- Public `x-forwarded-host` and `x-forwarded-proto` headers are ignored unless
  both base URL inference and
  `OpenAuthAxumOptions::trust_proxy_headers_for_base_url(true)` are enabled.
- Do not trust public `x-forwarded-for` headers unless traffic is terminated by
  a trusted reverse proxy.
- Do not run Tower/Axum body-consuming middleware on auth routes before
  `openauth-axum` (same idea as avoiding `express.json()` before Better Auth on
  Express).

## Status

Experimental beta. Router composition, request extraction, and adapter options
may change before stable release.

## Better Auth compatibility

Server-side Axum HTTP adapter for mounting OpenAuth routes. Aligned with Better
Auth **1.6.9** where it matters for this crate; OpenAuth is not a line-by-line port.

For route-level parity, test counts, intentional differences, and known gaps, see
[UPSTREAM.md](./UPSTREAM.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
