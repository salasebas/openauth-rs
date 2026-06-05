# openauth-core

Core contracts and server primitives for OpenAuth-RS.

## What It Is

`openauth-core` contains the framework-neutral pieces shared by the workspace:
API routing, auth context, cookies, crypto helpers, database adapter traits,
schema planning, errors, options, plugin contracts, sessions, users,
verification storage, and rate limiting.

Application code usually starts with `openauth`. Adapter and plugin crates use
`openauth-core` directly.

## What It Provides

- Core email/password, session, account, social sign-in, and verification route
  contracts.
- Database adapter traits and schema/migration metadata.
- `MemoryAdapter` for tests and local prototypes.
- Plugin, endpoint, hook, schema, and rate-limit extension contracts.
- Cookie, JWT/JWE, secret-rotation, and request/response primitives.

## Quick Start

```rust
use openauth_core::db::{auth_schema, AuthSchemaOptions};

let schema = auth_schema(AuthSchemaOptions::default());
let user_table = schema.table_name("user")?;
assert_eq!(user_table, "users");
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a full auth server, prefer the `openauth` builder:

```rust
use openauth::OpenAuth;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Feature Flags

Default features preserve the broad compatibility surface:

- `jose`: JOSE/JWE helpers backed by `josekit`.
- `oauth`: OAuth/social route support and OAuth helper re-exports.
- `social-providers`: social provider re-exports.

Use `default-features = false` for a smaller core build when you do not need
JOSE or social provider support.

## Production Notes

- Configure a strong secret and explicit `base_url`.
- Use a durable adapter such as SQLx, `tokio-postgres`, or
  `deadpool-postgres`; `MemoryAdapter` is not persistent.
- Use distributed rate-limit storage for multi-instance deployments.
- Keep browser/client SDK behavior outside core; core owns server boundaries.

## Status

Experimental beta. Adapter, plugin, option, and route contracts may change
before stable release.

## Upstream parity (Better Auth 1.6.9)

Parity pin: [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md)
(commit `f484269`). Upstream splits contracts (`@better-auth/core`) from runtime
(`packages/better-auth/src`); OpenAuth merges both into this crate. The `openauth`
facade re-exports core plus optional integrations.

| Upstream | OpenAuth |
| --- | --- |
| `@better-auth/core` (types, DB, endpoints, utils) | `openauth-core` modules |
| `better-auth` server runtime (routes, cookies, crypto) | `openauth-core` (`api`, `cookies`, `crypto`, …) |
| `@better-auth/core/oauth2` | `openauth-oauth` (feature `oauth`) |
| `@better-auth/core/social-providers` | `openauth-social-providers` |
| Product plugins (`admin`, `organization`, …) | `openauth-plugins` and sibling crates |
| `@better-auth/core/instrumentation` | Not in core (`openauth-telemetry` is separate) |
| JS/React/Vue clients | N/A (server-only) |

**Parity level (server, in-scope):** High for email/password, session, cookies,
crypto, DB adapter traits, rate limiting, and plugin pipeline. Medium for some
top-level options and OpenAPI exposure. Low/N/A for OpenTelemetry spans in core
and browser client SDKs.

**Test coverage:** ~501 Rust tests total (~453 in-scope excluding oauth/social);
76 files under `tests/` plus 2 unit tests in `src/`. Upstream in-scope baseline is
~50 `.test.ts` files with ~184 `it()` in `@better-auth/core` and ~770+ in
better-auth server tests. Every in-scope HTTP route has at least one test, but
many routes have shallow coverage compared with upstream suites such as
`session-api.test.ts`.

**Open gaps:** Deeper test matrices for session revocation and account routes;
route tests run with CSRF/origin checks disabled; social/OAuth token routes live
in other crates; `trustedProviders` dynamic callbacks not yet public; user
lifecycle hooks (`sendDeleteAccountVerification`, fresh-session delete semantics)
partially diverge. See `SERVER_PARITY.md` and `SQL_ADAPTER_PARITY.md` for detailed
notes.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
