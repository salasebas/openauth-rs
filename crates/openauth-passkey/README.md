# openauth-passkey

Server-side passkey plugin for OpenAuth.

```rust
use openauth_core::options::OpenAuthOptions;
use openauth_passkey::{passkey, PasskeyOptions};

let options = OpenAuthOptions::new()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com")
    .plugin(passkey(PasskeyOptions::default()));
```

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
