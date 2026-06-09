# openauth-social-providers

Social OAuth provider definitions for OpenAuth-RS.

## What It Is

`openauth-social-providers` contains server-side provider definitions used by
OpenAuth social sign-in. It builds on `openauth-oauth` and keeps provider
metadata, scopes, profile mapping, and token-auth behavior out of application
code.

## What It Provides

Provider modules include Apple, Atlassian, Cognito, Discord, Dropbox, Facebook,
Figma, GitHub, GitLab, Google, Hugging Face, Kakao, Kick, Line, Linear,
LinkedIn, Microsoft Entra ID, Naver, Notion, PayPal, Reddit, Salesforce, Slack,
Spotify, TikTok, Twitch, Twitter/X, Vercel, VK, WeChat, Zoom, and others.

## Quick Start

```rust
use openauth::OpenAuth;
use openauth_oauth::oauth2::{ClientSecret, ProviderOptions};
use openauth_social_providers::github::GitHubProvider;

let github = GitHubProvider::new(ProviderOptions {
    client_id: Some("github-client-id".into()),
    client_secret: Some(ClientSecret::new("github-client-secret")?),
    ..ProviderOptions::default()
})?;

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .social_provider(github)
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each provider exposes a typed `*Provider::new(...)` that returns
`Result<_, OAuthError>`. Providers build on [`OAuth2Client`] from
`openauth-oauth` internally; use [`ProviderIdentity`] when you only need
provider id/name metadata.

[`OAuth2Client`]: https://docs.rs/openauth-oauth/latest/openauth_oauth/oauth2/struct.OAuth2Client.html
[`ProviderIdentity`]: https://docs.rs/openauth-social-providers/latest/openauth_social_providers/trait.ProviderIdentity.html

Browser redirects and UI remain application/client concerns. This crate only
defines server-side OAuth provider behavior.

## Status

Experimental beta. Provider coverage, scopes, profile mapping, and provider
edge-case behavior may change before stable release.

## Better Auth compatibility

Server-side social OAuth provider definitions (metadata, scopes, profile mapping,
token auth). Aligned with Better Auth **1.6.9** where it matters for this crate;
OpenAuth is not a line-by-line port.

For route-level parity, test counts, intentional differences, and known gaps, see
[UPSTREAM.md](./UPSTREAM.md).

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
