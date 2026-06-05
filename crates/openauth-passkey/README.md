# openauth-passkey

Server-side passkey plugin for OpenAuth-RS.

## What It Is

`openauth-passkey` adds WebAuthn/passkey registration, authentication, and
credential management endpoints to OpenAuth. It is server-side only and uses
`webauthn-rs` for ceremony generation and cryptographic verification.

## What It Provides

- `/passkey/*` registration, authentication, list, update, and delete endpoints.
- A `passkeys` table schema contribution.
- Server-side WebAuthn ceremony state stored through OpenAuth verification
  storage and referenced by a signed short-lived cookie.
- Configurable relying-party ID, origin, relying-party name, user verification,
  authenticator selection, and registration user resolution.
- Ceremony and per-challenge rate limits for verify endpoints (see
  `PasskeyOptions::rate_limit` and `PasskeyOptions::challenge_rate_limit`).

## Quick Start

```rust
use openauth::OpenAuth;
use openauth_passkey::{passkey, PasskeyOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com")
    .plugin(passkey(PasskeyOptions::default()))
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For production deployments, set an explicit public `base_url`, and configure
`rp_id`/`origin` in `PasskeyOptions` when your auth server runs behind a proxy,
custom domain, or multi-origin setup.

## Endpoint Summary

- `GET /passkey/generate-register-options`
- `POST /passkey/verify-registration`
- `GET /passkey/generate-authenticate-options`
- `POST /passkey/verify-authentication`
- `GET /passkey/list-user-passkeys`
- `POST /passkey/update-passkey`
- `POST /passkey/delete-passkey`

Registration with an existing session requires a fresh session according to
OpenAuth core's `fresh_age` setting.

## Status

Beta. The plugin is usable for controlled integrations, but validate it against
the browsers, authenticators, RP ID, and origins used by your deployment before
production rollout.

## Upstream parity (Better Auth 1.6.9)

Parity pin: [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md).
Upstream: `@better-auth/passkey` (server routes only; no TS client in this crate).

| Area | Status | Notes |
| --- | --- | --- |
| HTTP endpoints | **High (~99%)** | Same **7** routes (method + path) |
| Challenge state | **High** | Verification store + `better-auth-passkey` cookie, 5 min TTL |
| WebAuthn | **High** | `webauthn-rs` vs `@simplewebauthn/server`; observable contract aligned |
| Error codes | **High** | 14 `PASSKEY_ERROR_CODES` matched |
| Tests | **Beyond upstream** | **60+** Rust tests vs 19 upstream server Vitest cases |
| Open gaps | **Minor** | No `mergeSchema` field rename; legacy `publicKey`-only verify not ported |

Intentional extras: discoverable auth without session, per-challenge rate limits,
hidden `webauthn_credential` field, stricter session-scoped auth challenge checks.
See [UPSTREAM_PARITY.md](./UPSTREAM_PARITY.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
