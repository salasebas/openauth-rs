# openauth-passkey

Server-side passkey plugin for OpenAuth-RS.

## Status

This package is in beta. It is usable for controlled server-side passkey
integrations, but should be validated against the browsers, authenticators, RP
ID, and origins used by your deployment before production rollout.

## What It Provides

`openauth-passkey` adds Better Auth-inspired passkey endpoints to the OpenAuth
server. It contributes a `passkeys` table with snake_case columns, stores
WebAuthn ceremony state server-side in OpenAuth verification storage, and uses
`webauthn-rs` for cryptographic verification.

## Example

```rust
use openauth::OpenAuth;
use openauth_passkey::{passkey, PasskeyOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com")
    .plugin(passkey(PasskeyOptions::default()))
    .build()?;
```

## Endpoints

The plugin contributes a `passkeys` table with snake_case columns and exposes:

- `GET /passkey/generate-register-options`
- `POST /passkey/verify-registration`
- `GET /passkey/generate-authenticate-options`
- `POST /passkey/verify-authentication`
- `GET /passkey/list-user-passkeys`
- `POST /passkey/update-passkey`
- `POST /passkey/delete-passkey`

WebAuthn ceremony state is stored server-side in OpenAuth's `verification`
storage and is referenced by a signed, short-lived cookie. It is not stored in
the cookie itself.

Generated registration and authentication option JSON follows the Better Auth
server behavior for passkey names, authenticator selection hints, attachments,
and extensions. Cryptographic verification is still delegated to `webauthn-rs`;
OpenAuth does not hand-roll WebAuthn verification or trust state supplied by the
client.

Registration with an existing session requires a fresh session according to the
core session `fresh_age` setting. This mirrors Better Auth's fresh-session
boundary for sensitive account changes.

## WebAuthn Configuration

Set `base_url` to the public origin of your auth server. By default the plugin
derives:

- `origin` from the request `Origin` header, then `base_url`, then
  `http://localhost` as a development fallback.
- `rp_id` from `base_url`, then from the first configured origin, then
  `localhost` as a development fallback.
- `rp_name` from the OpenAuth app name.

For production, configure explicit `origin(...)` and `rp_id(...)` values when
your auth server is behind a proxy, custom domain, or multi-origin setup.

## Storage Notes

The plugin creates a `passkeys` table by default. `credential_id` is unique to
prevent duplicate credential registration and race-created duplicates. Existing
installations with duplicate credential IDs must clean those rows before
applying the migration that adds the unique index.

`webauthn_credential` is internal storage for `webauthn-rs` and is intentionally
not serialized in API responses. Public credential metadata uses camelCase JSON;
`publicKey` is base64 encoded and `transports` use WebAuthn-compatible
lower-case values when exposed by `webauthn-rs`.

OpenAuth currently uses a stable WebAuthn user handle derived from the OpenAuth
user ID. A future passkey-first anonymous enrollment flow should store a random
per-registration user handle for stronger cross-credential privacy.

## Testing

Run the default passkey suite:

```sh
cargo nextest run -p openauth-passkey --test passkey
```

SQLite schema coverage runs by default. Postgres and MySQL schema tests are
ignored unless Docker Compose services are running:

```sh
./scripts/ensure-test-services.sh postgres mysql
OPENAUTH_TEST_POSTGRES_URL=postgres://user:password@localhost:5432/openauth \
OPENAUTH_TEST_MYSQL_URL=mysql://user:password@localhost:3306/openauth \
cargo nextest run -p openauth-passkey --test passkey --run-ignored ignored-only
```

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
