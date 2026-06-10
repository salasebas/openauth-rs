# Changelog

All notable changes to `openauth-social-providers` are documented in this file.

## Unreleased

### Changed

- **Breaking:** Provider factories (`GitHubProvider::new`, `DropboxProvider::new`,
  and the rest of the catalog) now return `Result<Self, OAuthError>` and hold an
  internal [`OAuth2Client`](https://docs.rs/openauth-oauth/latest/openauth_oauth/oauth2/struct.OAuth2Client.html)
  instead of implementing the removed `OAuthProviderContract`.
- **Breaking:** Re-exported [`ProviderIdentity`](src/runtime/mod.rs) for
  provider id/name metadata used by the social runtime macro.
- **Breaking:** Removed `Default` on providers whose default options lack
  `client_id` (Notion, Naver, Kakao, VK, Hugging Face, Roblox).
- Clarified Better Auth 1.6.9 parity status: remaining provider differences are
  intentional Rust API or token-verification hardening choices, not open
  in-scope `openauth-social-providers` implementation gaps.

### Fixed

- Fixed TikTok and WeChat authorization URL construction to reject empty OAuth
  `state` and malformed `redirect_uri` values before emitting provider redirects.
- Fixed social provider profile and userinfo HTTP calls to use the SSRF-safe
  provider HTTP client so requests cannot target literal private IPs by default.
- Fixed Facebook limited-login `verify_id_token` to reject opaque (non-JWT)
  tokens instead of treating them as locally verifiable ID tokens.
- Fixed Apple, Cognito, Facebook, Microsoft Entra ID, and Twitch ID token
  verification to require standard JWT claims (`sub`, `aud`, `iss`, `exp`)
  before accepting a token.
- Fixed PayPal `verify_id_token` to validate ID tokens against PayPal JWKS
  with issuer, audience, expiration, and nonce checks instead of accepting
  unsigned JWT payloads with only a `sub` claim.

## [0.0.6] - 2026-05-24

### Fixed

- Set token authentication methods for affected social providers.
- Updated Apple provider coverage.

## [0.0.5] - 2026-05-19

### Added

- Published the beta social providers release line.

