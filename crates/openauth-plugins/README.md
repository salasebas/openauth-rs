# openauth-plugins

Official server-side plugin modules for OpenAuth-RS.

## Status

This package is in experimental beta. Individual plugin APIs, schemas,
endpoints, hooks, and error codes may change before stable release.

## What It Provides

`openauth-plugins` groups server-side features inspired by Better Auth,
translated into Rust plugin contracts. Current modules include admin,
anonymous, API keys, bearer auth, captcha, custom sessions, device
authorization, email OTP, generic OAuth, haveibeenpwned, JWT, magic links, MCP,
multi-session, OAuth proxy, OpenAPI, organization, phone number, SIWE,
two-factor, username, and related helpers.

## Example

```rust
use openauth::OpenAuth;
use openauth_plugins::admin::{admin, AdminOptions};
use openauth_plugins::jwt;

let jwt_plugin = jwt::jwt()?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .plugin(admin(AdminOptions::default()))
    .plugin(jwt_plugin)
    .build()?;
```

Prefer plugin modules here for server behavior. Browser-only upstream behavior
should live in future thin client SDKs instead of this crate.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
