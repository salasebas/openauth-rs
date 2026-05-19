# openauth

Public entry crate for OpenAuth-RS.

## Status

This package is in experimental beta. Public APIs, re-exports, feature flags,
and crate boundaries may change before stable release.

## What It Provides

`openauth` is the main crate applications should start with. It exposes the
builder, options, HTTP handler, core types, and optional re-exports for selected
integration crates behind feature flags.

## Example

```rust
use openauth::{OpenAuth, RateLimitOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .rate_limit(RateLimitOptions::memory().enabled(true).window(60).max(100))
    .build()?;
```

Enable feature flags such as `passkey`, `plugins`, `sso`, `sqlx-sqlite`,
`sqlx-postgres`, `sqlx-mysql`, `deadpool-postgres`, or `tokio-postgres` when
you want the top-level crate to re-export those packages.

## Links

- [Root README](../../README.md)
- [openauth](README.md) - main auth crate.
- [openauth-core](../openauth-core/README.md) - core contracts.
- [openauth-axum](../openauth-axum/README.md) - Axum adapter.
- [openauth-cli](../openauth-cli/README.md) - CLI tools.
- [openauth-plugins](../openauth-plugins/README.md) - auth plugins.
- [openauth-passkey](../openauth-passkey/README.md) - passkeys.
- [openauth-oauth](../openauth-oauth/README.md) - OAuth primitives.
- [openauth-oauth-provider](../openauth-oauth-provider/README.md) - OAuth/OIDC provider.
- [openauth-social-providers](../openauth-social-providers/README.md) - social OAuth providers.
- [openauth-sso](../openauth-sso/README.md) - enterprise SSO.
- [openauth-scim](../openauth-scim/README.md) - SCIM support.
- [openauth-stripe](../openauth-stripe/README.md) - Stripe integration.
- [openauth-i18n](../openauth-i18n/README.md) - localized auth.
- [openauth-telemetry](../openauth-telemetry/README.md) - telemetry hooks.
- [openauth-sqlx](../openauth-sqlx/README.md) - SQLx adapters.
- [openauth-deadpool-postgres](../openauth-deadpool-postgres/README.md) - pooled Postgres.
- [openauth-tokio-postgres](../openauth-tokio-postgres/README.md) - minimal Postgres.
- [openauth-redis](../openauth-redis/README.md) - Redis rate limits.
- [openauth-fred](../openauth-fred/README.md) - Fred rate limits.
- [Repository](https://github.com/sebasxsala/openauth-rs)
