# openauth-social-providers

Social OAuth provider definitions for OpenAuth-RS.

## Status

This package is in experimental beta. Provider coverage, profile mapping,
scopes, and OAuth edge-case behavior may change before stable release.

## What It Provides

`openauth-social-providers` contains provider modules for services such as
Apple, Atlassian, Cognito, Discord, Dropbox, Facebook, Figma, GitHub, GitLab,
Google, LinkedIn, Microsoft Entra ID, Notion, PayPal, Reddit, Slack, Spotify,
TikTok, Twitch, Twitter/X, Vercel, Zoom, and others.

## Example

```rust
use openauth::OpenAuth;
use openauth_oauth::oauth2::ProviderOptions;
use openauth_social_providers::github::github;

let github = github(ProviderOptions {
    client_id: Some("github-client-id".into()),
    client_secret: Some("github-client-secret".to_owned()),
    ..ProviderOptions::default()
});

let auth = OpenAuth::builder()
    .secret("secret-a-at-least-32-chars-long!!")
    .social_provider(github)
    .build()?;
```

Provider modules are server-side OAuth definitions. Keep browser redirects and
client SDK concerns outside this crate.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
