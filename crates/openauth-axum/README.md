# openauth-axum

Axum integration for OpenAuth-RS.

## Status

This package is in experimental beta. Router composition, request extraction,
and adapter options may change before stable release.

## What It Provides

`openauth-axum` mounts the framework-neutral OpenAuth HTTP core into an Axum
application. It provides a ready-to-use router, route helpers for manual
composition, and an adapter-specific request body limit.

## Example

```rust
use openauth::OpenAuth;
use openauth_axum::router;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .build()?;

let app = router(auth)?;
```

When Axum is exposed directly to clients, run the service with connection info
so OpenAuth can use the real peer socket IP for rate limiting.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
