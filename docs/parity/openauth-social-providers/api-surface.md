# API surface parity

## Upstream `OAuthProvider` vs Rust `SocialOAuthProvider`

| Upstream (`OAuthProvider`) | Rust (`SocialOAuthProvider`) | Notes |
| --- | --- | --- |
| `id`, `name` | `id()`, `name()` | Same |
| `options?: O` | `provider_options() -> ProviderOptions` | Rust flattens provider-specific fields into `*Options` structs |
| `createAuthorizationURL({ state, codeVerifier, scopes?, redirectURI, display?, loginHint? })` | `create_authorization_url(SocialAuthorizationUrlRequest)` | `code_verifier` optional at type level; some providers error if missing |
| `validateAuthorizationCode({ code, redirectURI, codeVerifier?, deviceId? })` | `validate_authorization_code(SocialAuthorizationCodeRequest)` | `device_id` for VK etc. |
| `getUserInfo(token & { user? })` | `get_user_info(tokens, provider_user: Option<Value>)` | Apple `user` JSON via `provider_user` |
| `refreshAccessToken?` | `refresh_access_token` (default: unsupported `Err`) | Implemented per provider in `runtime/` |
| `revokeToken?` | `revoke_token` (default: unsupported `Err`) | Rare in upstream built-ins |
| `verifyIdToken?` | `verify_id_token(SocialIdTokenRequest)` (default: `Ok(false)`) | See provider catalog |
| `disableImplicitSignUp?` | `ProviderOptions.disable_implicit_sign_up` | Enforced in **core** routes, not this crate |
| `disableSignUp?` | `ProviderOptions.disable_sign_up` | Same |

Return type differences:

| Operation | Upstream | Rust |
| --- | --- | --- |
| Authorization URL | `URL` | `url::Url` |
| Token exchange | `Promise<OAuth2Tokens \| null>` | `Result<OAuth2Tokens, OAuthError>` |
| User info | `Promise<{ user, data } \| null>` | `Result<Option<OAuth2UserInfo>, OAuthError>` (profile `data` often in provider types / tests) |
| Verify ID token | `Promise<boolean>` | `Result<bool, OAuthError>` |

## Shared `ProviderOptions`

Fields aligned with `packages/core/src/oauth2/oauth-provider.ts` → `openauth-oauth::ProviderOptions`:

| Field | Upstream | Rust |
| --- | --- | --- |
| `clientId` | `unknown` | `ClientId` (single / multiple) |
| `clientSecret` | `string?` | `Option<String>` |
| `scope` | `string[]?` | `Vec<String>` |
| `disableDefaultScope` | `bool?` | `disable_default_scope` |
| `redirectURI` | `string?` | `redirect_uri` |
| `authorizationEndpoint` | override | same |
| `clientKey` | TikTok etc. | `client_key` |
| `disableIdTokenSignIn` | bool | `disable_id_token_sign_in` |
| `prompt` | string | `prompt` |
| `responseMode` | `query` \| `form_post` | `response_mode` |
| `overrideUserInfoOnSignIn` | bool | `override_user_info_on_sign_in` |
| `disableImplicitSignUp` / `disableSignUp` | bool | same |

**Not on Rust `ProviderOptions`** (upstream-only callbacks): `mapProfileToUser`, `getUserInfo`, `verifyIdToken`, `refreshAccessToken` — see [design-decisions.md](./design-decisions.md).

## Registry and public exports

| Upstream | Rust |
| --- | --- |
| `socialProviders` object | `PROVIDER_IDS` + per-module `fn provider_name(…)` |
| `export *` from each provider file | `pub mod <provider>` |
| `SocialProviders` type (config map) | App: `Vec` / `BTreeMap` of `Arc<dyn SocialOAuthProvider>` via core |
| `SocialProviderListEnum` (Zod) | No runtime enum; string ids at registration |
| `socialProviderList` | `PROVIDER_IDS` |

## Factory constructors

| Pattern | Providers |
| --- | --- |
| `fn foo(options) -> FooProvider` | Most |
| `FooProvider::new(options)` only | Dropbox |
| `fn cognito(options) -> Result<CognitoProvider, OAuthError>` | Cognito (validates domain/region/pool) |

## HTTP and security (Rust-only surface)

| Module | Purpose |
| --- | --- |
| `http::ProviderHttpClient` | SSRF-guarded userinfo GET |
| `http::ValidationHttpClient` | JWKS / ID-token validation fetches |
| `with_http_client` / `with_validation_http_client` | Test and custom client injection (several providers) |

Upstream relies on shared fetch without a separate SSRF module in `social-providers/`; OpenAuth centralizes outbound policy in `openauth-oauth` + `http.rs`.

## Integration in OpenAuth (outside this crate)

| Upstream | OpenAuth (`openauth-core`) |
| --- | --- |
| `betterAuth({ socialProviders: { github: { … } } })` | `.social_provider(github(opts))` / `.social_providers([…])` |
| Resolved list on context | `AuthContext::social_providers: BTreeMap<String, Arc<dyn SocialOAuthProvider>>` |
| `POST /sign-in/social` | `api/routes/social/flow.rs` |
| `GET /callback/:id` | same |
| `POST /link-social` | `api/routes/account.rs` |
| Plugins registering providers | `AuthPlugin::with_social_provider` |

`openauth-plugins` uses this crate directly for **Google One Tap** (`verify_id_token` + `get_user_info`), not only via core registry.

## `revokeToken`

Upstream interface allows `revokeToken`; few built-in providers implement it. Rust exposes `revoke_token` on the trait with a default error; no built-in provider in this crate implements revocation today.
