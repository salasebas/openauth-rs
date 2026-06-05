# openauth-plugins

Official server-side plugin modules for OpenAuth-RS.

## What It Is

`openauth-plugins` groups Better Auth-inspired server features translated into
OpenAuth's Rust plugin contracts. Use it when you want optional auth behavior
without pulling each feature into `openauth-core`.

The deprecated upstream `oidc-provider` plugin is not implemented here. Use
`openauth-oauth-provider` for OAuth 2.1 and OpenID Connect provider behavior.

## What It Provides

Current modules include access control, additional fields, admin, anonymous
users, API keys, bearer sessions, CAPTCHA hooks, custom sessions, device
authorization, email OTP, generic OAuth, Have I Been Pwned checks, JWT, last
login method, magic links, MCP, multi-session, OAuth proxy, one-tap, one-time
tokens, OpenAPI, organizations, phone number, SIWE, two-factor, and username.

Some plugins are pure helpers. Many require an OpenAuth adapter because they
store users, sessions, keys, organizations, tokens, or verification state.

## Quick Start

```rust
use openauth::OpenAuth;
use openauth_plugins::admin::{admin, AdminOptions};
use openauth_plugins::jwt;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .plugin(admin(AdminOptions::default()))
    .plugin(jwt::jwt()?)
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use module-specific options when a plugin needs application callbacks such as
email sending, OTP delivery, CAPTCHA verification, SIWE verification, or custom
authorization policy.

## Operational Notes

- Run adapter migrations after adding plugins that contribute schema.
- Prefer server-side plugins here for server behavior; browser-only upstream
  helpers should live in thin client SDKs instead.
- API key storage can use the database and selected secondary-storage paths.
- In pure `SecondaryStorage` mode (no database fallback) the `api-key:by-ref:*`
  listing index is mutated through an in-process lock, so concurrent
  create/delete on one process stay consistent. Multi-process deployments still
  need a secondary-storage backend with atomic collection semantics, or the
  database fallback, to keep `/api-key/list` from dropping concurrently written
  keys.
- OpenAPI support serves generated auth schemas and optional Scalar reference
  UI.

## Status

Experimental beta. Individual plugin APIs, schemas, endpoints, hooks, and
error codes may change before stable release.

## Upstream parity (Better Auth 1.6.9)

Parity pin: [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md)
(commit `f484269`). Upstream server plugins live under
`packages/better-auth/src/plugins/` (26 modules) plus `@better-auth/api-key` as a
separate npm package. OpenAuth consolidates **27 server plugins** in this crate.
The deprecated upstream `oidc-provider` plugin is replaced by `openauth-oauth-provider`.
SSO, SCIM, Stripe, and Electron/Expo surfaces are out of scope here.

**Parity level:** High for HTTP routes (~130) and schema/hook wiring; June 2026
work closed server gaps for `generateTOTP`, organization access-control options,
api-key `defaultPermissions` and schema merge, two-factor custom OTP storage,
jwt/phone-number/username schema options, and `verification.storeIdentifier: hashed`
(in `openauth-core`). Remaining gaps are mostly test depth and a few organization
options (`allowUserToCreateOrganization` callback, `organizationHooks` async,
session field renames).

**Test coverage:** **610** integration tests under `tests/<plugin>/` vs **986**
upstream `it()` declarations (excluding `test-utils` and `oidc-provider`). Largest
gaps: organization (−150), api-key (−124), email-otp (−42), two-factor (−34).
Several plugins exceed upstream counts (access, bearer, multi_session, one_tap).

**Open gaps:** Partial test parity vs upstream Vitest suites; some organization
permission merge semantics; plugin rate limits not always exposed as options;
client-only `client.ts` exports and TypeScript inference helpers are N/A.
Inventory guard: `tests/plugins.rs`
(`upstream_server_plugin_parity_is_explicit_about_replaced_oidc_provider`).
See `SERVER_PARITY.md` for per-plugin design notes.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
