# 02 — Package and code mapping

## npm ↔ crate equivalence

| Capability | Upstream | OpenAuth | Notes |
| --- | --- | --- | --- |
| Authorization Server | `@better-auth/oauth-provider` | `openauth-oauth-provider` | **This document** |
| OAuth client / PKCE / JWT verify helpers | `better-auth/oauth2`, core crypto | `openauth-oauth` | RP and social, not AS |
| JWT plugin (JWKS, sign access/id) | `better-auth/plugins/jwt` | `openauth-plugins::jwt` | Optional merge from `oauth_provider()` |
| Router / session / adapter | `better-auth`, `@better-auth/core` | `openauth-core` | Parity in another crate |
| MCP HTTP server | Not in oauth-provider (handler only) | `openauth-plugins::mcp` | Separate `/mcp/*` routes |
| Browser OAuth query | `./client` export | — | **N/A** server-only |
| Resource server SDK | `./resource-client` | `mcp::*` + `validate_access_token` | No TS client plugin |

Upstream does not split the AS across crates: **1 npm package → 1 Rust crate**. The “split” appears because Better Auth **couples** JWT and login in other monorepo packages.

## File tree

### Upstream (`packages/oauth-provider/src/`)

| File | Role |
| --- | --- |
| `index.ts` | Server re-exports + metadata + `mcpHandler` |
| `oauth.ts` | Plugin factory, defaults, before/after hooks, rate limits, schema merge |
| `schema.ts` | Models `oauthClient`, `oauthRefreshToken`, `oauthAccessToken`, `oauthConsent` |
| `authorize.ts` | `GET /oauth2/authorize` |
| `consent.ts` | `POST /oauth2/consent` |
| `continue.ts` | `POST /oauth2/continue` |
| `token.ts` | `POST /oauth2/token` |
| `introspect.ts` / `revoke.ts` | Introspection and revocation |
| `userinfo.ts` / `logout.ts` | UserInfo and end-session |
| `metadata.ts` | Discovery + standalone wrappers |
| `register.ts` | DCR `POST /oauth2/register` |
| `oauthClient/endpoints.ts` | Client CRUD + rotate secret |
| `oauthConsent/endpoints.ts` | Consent CRUD |
| `middleware/index.ts` | `publicSessionMiddleware` + `oauth_query` |
| `utils/index.ts` | PKCE, secrets, pairwise, prompts, Basic auth |
| `mcp.ts` | `mcpHandler` middleware |
| `client.ts` | **Browser** `oauthProviderClient` |
| `client-resource.ts` | **Client** `verifyAccessToken`, protected resource metadata |
| `types/*` | OAuth/OIDC types, Zod |
| `client.ts` | Excluded from server parity |

### OpenAuth (`crates/openauth-oauth-provider/src/`)

| Module | Role | Primary upstream |
| --- | --- | --- |
| `lib.rs` | `oauth_provider()`, options resolution, rate limits, JWT merge | `oauth.ts` |
| `options.rs` | `OAuthProviderOptions`, resolvers, types | `types/index.ts` + part of `oauth.ts` |
| `schema.rs` | DB contributions | `schema.ts` |
| `models.rs` | Row structs | types in `types` + schema |
| `client.rs` | DCR, validation, domain CRUD | `register.ts`, `oauthClient/endpoints.ts`, `utils` |
| `authorize.rs` | `decide_authorize` | logic in `authorize.ts` |
| `consent.rs` | Consent helpers | `oauthConsent` + `consent.ts` |
| `endpoints/authorization.rs` | HTTP authorize | `authorize.ts` |
| `endpoints/consent.rs` | consent + continue + consent API | `consent.ts`, `continue.ts`, `oauthConsent` |
| `endpoints/token.rs` | HTTP token | `token.ts` |
| `endpoints/introspection.rs` | introspect + revoke HTTP | `introspect.ts`, `revoke.ts` |
| `endpoints/metadata.rs` | well-known | `metadata.ts` |
| `endpoints/userinfo.rs` | userinfo | `userinfo.ts` |
| `endpoints/logout.rs` | end-session | `logout.ts` |
| `endpoints/clients.rs` | DCR + client management | `register.ts`, `oauthClient` |
| `token/mod.rs` | Issuance, storage, grants | `token.ts`, `utils` |
| `token/claims.rs` | ID token, pairwise | `token.ts`, `utils` |
| `token/introspection.rs` | Validate / introspect / revoke | `introspect.ts`, `revoke.ts`, `utils` |
| `metadata.rs` | Metadata builders | `metadata.ts` |
| `utils.rs` | HTTP, crypto, session | `utils/index.ts` |
| `error.rs` | `OAuthProviderError` | OAuth `APIError` |
| `mcp.rs` | MCP helpers (public) | `mcp.ts` + part of `client-resource.ts` |

## Database schema

| Upstream model (logical camelCase) | OpenAuth physical table | Notable fields |
| --- | --- | --- |
| `oauthClient` | `oauth_clients` | `client_id`, `redirect_uris`, `subject_type`, `reference_id`, flags `skip_consent`, `enable_end_session`, `require_pkce` |
| `oauthRefreshToken` | `oauth_refresh_tokens` | hashed `token`, `revoked`, `auth_time`, immutable `scopes` |
| `oauthAccessToken` | `oauth_access_tokens` | opaque; `refresh_id` link |
| `oauthConsent` | `oauth_consents` | per `user_id` + `client_id` (+ `reference_id`) |
| `verification` (core) | core `verification` table | authorization codes (JSON `AuthorizationCodeValue`) |

**Decision:** physical names use **plural snake_case** in Rust; OAuth/DCR JSON remains **snake_case** in bodies where RFC 7591 applies.

## Dependencies

| Upstream | OpenAuth |
| --- | --- |
| `jose` | `josekit` (+ `openauth-plugins::jwt`) |
| `zod` | `serde` + explicit validation in `client.rs` / endpoints |
| `better-auth/crypto`, `@better-auth/utils` | `openauth-core::crypto`, `sha2`, `hmac`, `subtle`, `data-encoding` |
| `better-auth/oauth2` | Logic in `token/`, `utils.rs` |
| `better-auth/plugins/jwt` | `openauth-plugins::jwt` (workspace feature) |
| `@modelcontextprotocol/sdk` (dev) | No dependency; MCP tests use local helpers |

## Public exports

| Upstream `index.ts` | OpenAuth `lib.rs` |
| --- | --- |
| `oauthProvider` | `oauth_provider`, `oauth_provider_with_jwt` |
| `getOAuthProviderState` | — (no global request state; pending in verification) |
| `authServerMetadata`, `oidcServerMetadata` | `auth_server_metadata`, `oidc_server_metadata` |
| `oauthProviderAuthServerMetadata` (handler) | Plugin endpoints + `oauth_authorization_server_metadata`, `well_known_metadata_response` |
| `mcpHandler` | `mcp::*` functions, not HTTP middleware |
| `export type *` | Types in `options`, `models`, `client`, etc. |

| Upstream subpath | OpenAuth |
| --- | --- |
| `./client` → `oauthProviderClient` | **Does not exist** |
| `./resource-client` | Partial: `mcp::protected_resource_metadata`, `validate_bearer_token` |

## Modules upstream does not have

| Rust module | Reason |
| --- | --- |
| `endpoints/mod.rs` | Thin HTTP adapter over domain |
| Tests under `tests/oauth_provider/` | Integration with `AuthRouter` + `MemoryAdapter` |

## Upstream modules with no Rust equivalent in this crate

| Upstream | Reason |
| --- | --- |
| `client.ts` | Browser-only |
| `client-resource.ts` (as client plugin) | Server-only; part in `mcp.rs` |
| `middleware/index.ts` | Prelogin validates `oauth_query` inline in `endpoints/clients.rs`; no global plugin middleware |
| `version.ts` | `VERSION` in `lib.rs` from Cargo |
