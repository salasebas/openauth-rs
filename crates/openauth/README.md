# openauth

Main application crate for OpenAuth-RS.

## What It Is

`openauth` is the crate most applications should start with. Use [`prelude`](crate::prelude)
for the app-dev surface, then reach into focused modules (`openauth::db`, `openauth::plugin`,
`openauth::api`, …) when you extend adapters, plugins, or endpoints.

Depend on `openauth-core` directly only for adapter/plugin internals or very small binaries
that do not need the umbrella crate.

## Quick Start

```rust
use openauth::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = OpenAuth::builder()
        .secret("secret-a-at-least-32-chars-long!!")
        .base_url("https://app.example.com/api/auth")
        .email_password(EmailPasswordOptions::new().enabled(true))
        .rate_limit(RateLimitOptions::memory().enabled(true).window(60).max(100))
        .build()
        .await?;

    # let _ = auth;
    Ok(())
}
```

Attach an adapter when you need durable users, sessions, accounts, plugin data,
or migrations. Enable the matching SQLx dialect on the `openauth` crate
(`sqlx-sqlite`, `sqlx-postgres`, or `sqlx-mysql`):

```toml
[dependencies]
openauth = { version = "0.1.0", features = ["sqlx-sqlite"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use openauth::prelude::*;
use openauth::sqlx::SqliteAdapter;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePoolOptions::new().connect("sqlite://openauth.db").await?;

    let auth = OpenAuth::builder()
        .secret("secret-a-at-least-32-chars-long!!")
        .base_url("https://app.example.com/api/auth")
        .email_password(EmailPasswordOptions::new().enabled(true))
        .adapter(SqliteAdapter::new(pool))
        .build()
        .await?;

    auth.run_migrations().await?;
    Ok(())
}
```

Mount into Axum with [`openauth-axum`](../openauth-axum/README.md):

```rust
use openauth::prelude::*;
use openauth_axum::OpenAuthAxumExt;

let app = auth.into_router()?;
```

## Feature Flags

- `i18n`: re-export `openauth-i18n`.
- `plugins`: re-export `openauth-plugins`.
- `passkey`: re-export `openauth-passkey`.
- `sso`: re-export `openauth-sso`.
- `oidc`: re-export relying-party OIDC helpers.
- `saml` and `saml-signed`: re-export experimental SAML helpers.
- `scim`: re-export server-side SCIM provisioning.
- `stripe`: re-export server-side Stripe billing integration.
- `telemetry`: re-export the telemetry surface from
  [`openauth-telemetry`](../openauth-telemetry/README.md) (`create_telemetry`,
  `get_telemetry_auth_config`, `TelemetryContext`, `TelemetryEvent`,
  `TelemetryPublisher`, `TelemetryTestHooks`, `CustomTrackFn`) and wire the
  publisher during [`OpenAuthBuilder::build`](crate::OpenAuthBuilder::build).
  This feature also enables `openauth-telemetry/oauth` so social-provider config
  snapshots match Better Auth parity.
- `sqlx-sqlite`, `sqlx-postgres`, `sqlx-mysql`: SQLx adapters.
- `tokio-postgres` and `deadpool-postgres`: Postgres adapters.

## Choosing The Right Crate

- Start with `openauth` for applications.
- Use `openauth-core` for adapter/plugin internals.
- Use `openauth-sso` to consume external enterprise IdPs.
- Use `openauth-oauth-provider` when your app must issue OAuth/OIDC tokens.
- Use `openauth-axum` to mount OpenAuth in Axum.

## Status

Experimental beta. Public re-exports, feature flags, and crate boundaries may
change before stable release.

## Better Auth compatibility

Server-side public entry crate (builder, handler, re-exports). Aligned with
Better Auth **1.6.9** where it matters for this crate; OpenAuth is not a
line-by-line port.

For route-level parity, test counts, intentional differences, and known gaps, see
[UPSTREAM.md](./UPSTREAM.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
