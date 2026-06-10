# Changelog

All notable changes to `openauth-oauth` are documented in this file.

## Unreleased

### Added

- Added [`OAuth2Client`](src/oauth2/client.rs), `OAuth2ClientBuilder`, and flow
  builders (`authorization_url`, `exchange_code`, `refresh_token`,
  `client_credentials`) as the primary OAuth client API.
- Added `exchange_authorization_code`, `refresh_access_token_at`, and
  `submit_token_form` for discovery-based token endpoints.
- Added `ClientSecret` newtype with redacted `Debug`, custom serde, and
  `ProviderOptions::client_secret_str` / `with_client_secret`.
- Added `RefreshAccessTokenRequest::header` and `RefreshTokenBuilder::header`
  for provider-specific token request headers.
- Added `validate_authorization_url_invariants` for manual provider URL builders
  that need the same non-empty `state` and parseable `redirect_uri` checks as
  `create_authorization_url`.
- Added options-based verification: `JwksVerifyOptions`, `ValidateTokenOptions`,
  and `VerifyAccessTokenOptions` with optional injected `OAuthHttpClient`.

### Changed

- **Breaking:** Removed top-level `validate_authorization_code`,
  `refresh_access_token`, and `client_credentials_token` helpers (including
  `*_with_client` and `*_with_cache_config` variants). Use `OAuth2Client` flow
  builders or the advanced `exchange_authorization_code` /
  `refresh_access_token_at` helpers instead.
- **Breaking:** Removed `OAuthProviderContract`, `ClientTokenRequest`, and
  `ClientCredentialsGrant`.
- **Breaking:** Removed alias functions `authorization_code_request`,
  `refresh_access_token_request`, and `client_credentials_token_request`.
- **Breaking:** `ProviderOptions.client_secret` is now `Option<ClientSecret>`
  instead of `Option<String>`.
- **Breaking:** Consolidated JWT/JWKS verification entry points behind
  `verify_jws_access_token`, `validate_token`, and `verify_access_token` with
  their options structs.
- Made the `request` module and `OAuthFormRequest` mutators `pub(crate)`; the
  form request type remains public for inspection.

### Fixed

- Token exchange requires `code_verifier` when the authorization request used PKCE.
- Local ID token verification rejects non-integer `exp` / `iat` / `nbf` claims.
- Fixed the default OAuth HTTP client to block GET/POST requests whose URLs
  use literal private, loopback, or link-local IP addresses (SSRF hardening).
- Fixed HTTP Basic client authentication to form-encode `client_id` and
  `client_secret` per RFC 6749 §2.3.1 before Base64 encoding (reserved and
  non-ASCII credentials no longer break token exchange).
- Fixed authorization URL and authorization-code/refresh token request builders
  so `additional_params` cannot override `state`, PKCE (`code_challenge`,
  `code_verifier`, `code_challenge_method`), or other standard OAuth fields.

## [0.0.6] - 2026-05-24

### Added

- Added OAuth claims, JWKS, introspection, HTTP, request, and token validation
  helpers.
- Added authorization URL and refresh/access-token support helpers.
- Added expanded OAuth helper coverage.

### Changed

- Updated authorization-code validation and token verification behavior.
- Made JOSE support feature-gated through the crate feature surface.

## [0.0.5] - 2026-05-19

### Added

- Published the beta OAuth helper release line.

