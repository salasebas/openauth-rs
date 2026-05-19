# openauth-sso

Enterprise single sign-on support for OpenAuth-RS.

## Status

This package is in experimental beta. SSO provider management, OIDC, SAML,
domain verification, audit hooks, and rate-limit behavior may change before
stable release.

## What It Provides

`openauth-sso` exposes a server-side plugin for enterprise SSO. It adds SSO
provider storage, OIDC sign-in, SAML ACS and metadata endpoints, domain
verification, account linking helpers, organization provisioning, audit hooks,
and SAML single logout support.

## Example

```rust
use openauth::OpenAuth;
use openauth_sso::{sso, SsoOptions};

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .plugin(sso(SsoOptions::default()))
    .build()?;
```

Enable the `saml-signed` feature only when the deployment provides the required
native XML security tooling and dependencies.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
