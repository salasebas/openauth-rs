# openauth-passkey

Server-side passkey plugin for OpenAuth-RS.

## Status

This package is in experimental beta. Endpoint details, option names, schema
contributions, and WebAuthn ceremony behavior may change before stable release.

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

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
