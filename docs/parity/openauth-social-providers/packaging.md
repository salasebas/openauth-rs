# Packaging and module mapping

Better Auth ships social providers inside the **`@better-auth/core`** package. OpenAuth splits responsibilities across multiple crates so OAuth primitives, provider catalogs, and HTTP auth stay testable and optional.

## Package map

| Concern | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| Built-in provider registry + factories | `packages/core/src/social-providers/` | `crates/openauth-social-providers/` |
| Shared OAuth2 helpers (`createAuthorizationURL`, `validateAuthorizationCode`, `refreshAccessToken`, JWKS) | `packages/core/src/oauth2/` | `crates/openauth-oauth` (`feature = "jose"`) |
| Auth config `socialProviders` + runtime list | `packages/better-auth/src/context/create-context.ts` | `openauth-core` `OpenAuthOptions::social_providers`, `AuthContext::social_providers` |
| Public re-export | `better-auth/social-providers` → core | `openauth_core::social_providers` (feature `social-providers`) |
| Sign-in / callback / link HTTP | `packages/better-auth/src/api/routes/` | `openauth-core` `api/routes/social/`, `account` |
| Extra IdPs (not in registry) | `generic-oauth` plugin + `providers/*` | Separate plugin work (not this crate) |
| CLI scaffolding | `packages/cli/.../social-providers.config.ts` | N/A (server-only) |

## File layout comparison

| Upstream | OpenAuth |
| --- | --- |
| One `*.ts` per provider (~provider logic + types) | `src/<provider>.rs` (public API, mapping, HTTP) + `src/runtime/<provider>.rs` (`SocialOAuthProvider` wiring) |
| `index.ts` exports `socialProviders` map | `lib.rs` modules + `PROVIDER_IDS` |
| No dedicated HTTP SSRF layer in providers | `src/http.rs` (`ProviderHttpClient`, `ValidationHttpClient`) |

The **`runtime/`** split is an OpenAuth design choice: keep provider behavior testable without exposing trait impl details on every public type.

## Naming differences

| Upstream | OpenAuth | Notes |
| --- | --- | --- |
| Config key `microsoft` | Module `microsoft_entra_id`, id `"microsoft"` | Same public provider id |
| File `microsoft-entra-id.ts` | `microsoft_entra_id.rs` | Rust naming |
| Factory `github(options)` | `github(options)` + `GitHubProvider::new` | Same pattern (Dropbox: `new` only) |
| `socialProviderList` / Zod enum | `PROVIDER_IDS: &[&str]` | Rust has no Zod; core validates at registration |
| `enabled?: false` on config entry | Not on provider struct | Disabled providers omitted from `OpenAuth` builder |
| Async config factory `socialProviders.github(async () => …)` | Sync Rust constructors | Use `Arc<dyn SocialOAuthProvider>` at app layer if needed |

## Features and dependencies

| | Upstream core | `openauth-social-providers` |
| --- | --- | --- |
| Cargo features | N/A (bundled in core) | None on this crate |
| Enable in app | `betterAuth({ socialProviders: { … } })` | `openauth-core` feature `social-providers` (default) or direct dependency |
| Crypto / JWT | `jose` (TS) | `josekit` + `openauth-oauth/jose` |
| HTTP | `@better-fetch/fetch` | `reqwest` via `openauth-oauth` + SSRF wrappers in `http.rs` |

## What we deliberately do not mirror

| Upstream surface | Reason |
| --- | --- |
| `better-auth` client `signIn.social` / `linkSocial` | Client-only; OpenAuth documents server routes in core |
| `SocialProviderListEnum` Zod schema | Rust uses typed registration + `PROVIDER_IDS` for tests |
| `socialProviders` async factories | App composes `Arc<dyn SocialOAuthProvider>` instead |
| Monolithic `better-auth` npm package | Rust workspace crates for oauth, providers, core, plugins |

## Related crates for full social sign-in parity

To implement or audit **end-to-end** social login (not just provider definitions), also read:

- `openauth-core` — state, callbacks, account linking, id-token sign-in routes
- `openauth-oauth` — `ProviderOptions`, PKCE, token helpers
- Upstream `packages/better-auth/src/social.test.ts`, `oauth2/link-account.test.ts`
