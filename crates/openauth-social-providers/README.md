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
use openauth_oauth::oauth2::ProviderOptions;
use openauth_social_providers::github::github;

let github = github(ProviderOptions {
    client_id: Some("github-client-id".into()),
    client_secret: Some("github-client-secret".into()),
    ..ProviderOptions::default()
});

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .base_url("https://app.example.com/api/auth")
    .social_provider(github)
    .build()?;
# let _ = auth;
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
