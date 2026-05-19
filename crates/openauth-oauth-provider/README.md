# openauth-oauth-provider

OAuth 2.1 and OpenID Connect provider support for OpenAuth-RS.

## Status

This package is in experimental beta. Provider metadata, endpoint behavior,
token storage, grant support, and option validation may change before stable
release.

## What It Provides

`openauth-oauth-provider` lets an OpenAuth-RS server act as an OAuth/OIDC
provider. It contributes client, consent, access token, and refresh token
schema, exposes provider endpoints, and can include JWT/JWKS support through
`openauth-plugins`.

## Example

```rust
use openauth::OpenAuth;
use openauth_oauth_provider::{oauth_provider, OAuthProviderOptions};

let provider = oauth_provider(OAuthProviderOptions {
    login_page: "/login".to_owned(),
    consent_page: "/oauth/consent".to_owned(),
    scopes: vec!["openid".to_owned(), "profile".to_owned(), "email".to_owned()],
    ..OAuthProviderOptions::default()
})?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .plugin(provider.into_auth_plugin())
    .build()?;
```

Keep client registration and token storage settings explicit for production
deployments.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
