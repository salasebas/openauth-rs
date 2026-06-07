# openauth-core — Better Auth upstream parity

| Field | Value |
| --- | --- |
| **Parity pin** | Better Auth **1.6.9** ([`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md)) |
| **Upstream packages** | `@better-auth/core` + `better-auth` server runtime (see scope boundary) |
| **Rust crate** | `openauth-core` |
| **Parity level** | **High** (core server contracts); tracked G1-G15 parity gaps closed |
| **Audit status** | **Complete** (server-only inventory, 2026-06-05) — all in-scope files classified and tracked implementation deltas closed |

## Summary

OpenAuth merges `@better-auth/core` and the `better-auth` server runtime into one crate.
Core HTTP paths, verification storage, cookies, crypto, adapter traits, plugin DB hooks,
rate limiting, dynamic options, session-depth behavior, OAuth state hardening, and
server-only middleware align closely with 1.6.9. The tracked implementation deltas
below are closed; remaining differences are intentional Rust integration choices.

## Scope boundary (server-only)

| Upstream path | Disposition |
| --- | --- |
| `packages/core/src/{db,api,context,error,env,utils}/` | **In scope** → `openauth-core` |
| `packages/better-auth/src/{api,cookies,crypto,context,auth,db,oauth2,state,utils}/` | **In scope** → `openauth-core` |
| `packages/core/src/oauth2/` | **Sibling** → `openauth-oauth` |
| `packages/core/src/social-providers/`, `better-auth/src/social-providers/` | **Sibling** → `openauth-social-providers` |
| `packages/core/src/instrumentation/` | **Sibling** → `openauth-telemetry` |
| `better-auth/src/plugins/` | **Sibling** → `openauth-plugins` |
| `better-auth/src/adapters/` | **Sibling** → `openauth-sqlx`, `tokio-postgres`, … |
| `better-auth/src/integrations/` | **Sibling** → `openauth-axum` |
| `better-auth/src/auth/{full,minimal}.ts`, `auth/base.ts` handler loop | **Sibling** → `openauth` facade |
| `better-auth/src/test-utils/`, `*.test.ts` | Test harness — not parity surface |
| `better-auth/src/types/*.ts`, `db/to-zod.ts`, `db/field.ts` | Schema/inference helpers — no runtime parity target |

**Inventory:** 143 Rust `src/*.rs`, 81 test files, 55 upstream `@better-auth/core` server
`.ts`, 98 upstream `better-auth` server `.ts` (excl. `client/`, `plugins/`) — all mapped.

## Feature parity

| Area | Status | Notes |
| --- | --- | --- |
| Core HTTP routes | ✅ | `core_auth_async_endpoints` — same paths as upstream core routes |
| Session routes | ✅ | Implemented; skip-refresh, deferred `needsRefresh`, cookie-cache and chunked-cache route coverage |
| Password / email verification / delete-user | ✅ | Route + service layers |
| Cookies & chunked store | ✅ | `ChunkedCookieStore`, cache, defer/disable refresh |
| Crypto (password, JWT/JWE, secrets) | ✅ | Implemented; secret rotation covers current, old, legacy, and tamper rejection |
| Verification token storage | ✅ | `verification.rs` + `VerificationStore` |
| Secondary session storage | ✅ | `session.rs` + optional storage index |
| DB adapter traits & `MemoryAdapter` | ✅ | Contract + reference impl |
| DB mutation hooks | ✅ | `db/hooks/`, `with-hooks` parity |
| Internal adapter (user/session/verification CRUD) | ✅ | `listUsers`, `countTotalUsers`, batch `findSessions`, and update-user cookie-cache refresh implemented |
| Schema output on responses | ✅ | User/session route output helpers apply schema/additional-field returnability, including plugin schema fields |
| Rate limiting | ✅ | Router-level; Redis via `openauth-redis` for multi-instance |
| CSRF / origin guards | ✅ | `api/security.rs` |
| Trusted origins (static + request-aware) | ✅ | `auth/trusted_origins.rs` |
| Request-scoped state | ✅ | `define_request_state`, session user/path |
| Skip session refresh (per-request) | ✅ | Tokio request-state flag mirrors upstream `api/state/should-session-refresh.ts` |
| `freshSessionMiddleware` | ✅ | Reusable `fresh_session_middleware()` export plus delete-user shared helper |
| `requireResourceOwnership` middleware | ✅ | Server-only `require_resource_ownership()` middleware export |
| `onAPIError` hook | ✅ | `throw`, redirect, custom handler, and typed default error page customization |
| OAuth link-account / state | ✅ | OAuth state carries `oauth_state` nonce and supports explicit `skip_state_cookie_check`; oauth-proxy packages nonce |
| OAuth / social HTTP routes | ✅ | Feature `oauth`; providers in sibling crate |
| Options / context init | ✅ | Static + dynamic trusted origins/providers; Axum request-derived base URL and trusted proxy header handling |
| Plugin schema merge | ✅ | `plugin/schema.rs`, `context/plugins.rs` |
| Plugin migration metadata | ✅ | Names plus SQL/body/plan metadata; SQL bodies/plans executable through adapter migration hooks |
| Router / plugin pipeline | ✅ | `router.rs`, `plugin_pipeline.rs` vs `api/index.ts` |
| OpenAPI metadata | ✅ | Core and plugin routes expose operation IDs, schemas, path params, and success/redirect responses |
| Programmatic `auth.api` / endpoint caller | Intentional difference | `openauth` facade — HTTP router is the Rust integration surface |

## Test coverage

| Surface | OpenAuth | Upstream | Notes |
| --- | ---: | ---: | --- |
| Crate total | 541 | — | ~491 in-scope excl. feature-gated OAuth/social suites |
| `@better-auth/core` server | — | 148 `it()` | Excl. `oauth2/`, `instrumentation/`, `social-providers/` |
| `better-auth` server runtime | — | ~798 `it()` | Excl. `plugins/` |
| HTTP routes | 116 | 177 | `session-api.test.ts`: 56 |
| Context / init | 25 | 115 | `create-context.test.ts` |
| DB layer | 140 | 56+ | `internal-adapter.test.ts`: 33 |
| Cookies | 31 | 54 | |
| Crypto / secrets | 39 | 50+ | `secret-rotation.test.ts`: 38 |
| Auth / OAuth | 35 | 55+ | `social.test.ts`: 40; `link-account.test.ts`: 15 |
| Middleware / rate limit | 34 | 52 | |
| Router / pipeline | 44+ | 51+ | `to-auth-endpoints.test.ts` |
| Options | 11 | (in context) | |
| `onAPIError` | 4+ | covered | `tests/api/router.rs`, `tests/api/routes/error_page.rs` |

```bash
cargo nextest run -p openauth-core
```

Match HTTP status, error codes, cookie names, and DB mutations.

## Intentional differences

| Topic | Better Auth | OpenAuth | Why |
| --- | --- | --- | --- |
| Trusted providers | array or function | static `trusted_providers` plus dynamic provider callback | idiomatic Rust callback API |
| Error payloads | upstream string shapes | typed JSON / redirect | equivalent security behavior |
| SQL identifiers | flexible Kysely style | strict ASCII; reject invalid | fail-closed SQL boundary |
| `LIKE` filters | wildcards from input | escape `%`, `_`, `\` | untrusted input cannot broaden queries |
| `GET /ok` | JSON `{"ok": true}` | plain-text `OK` | minimal liveness probe |
| Request state | `AsyncLocalStorage` | Tokio `task_local` | idiomatic async Rust |
| Schema validation | Zod (`to-zod.ts`) | JSON Schema / OpenAPI | different stack, same routes |

## Closed parity gaps

| ID | Gap | Severity | Notes |
| --- | --- | --- | --- |
| G1 | Session route test depth | Closed | Added skip-refresh, deferred `needsRefresh`, cookie-cache and chunked-cache route coverage |
| G2 | Reusable fresh-session middleware | Closed | public `fresh_session_middleware()` and shared freshness helper |
| G3 | `shouldSkipSessionRefresh` | Closed | request-scoped flag wired through session resolution |
| G4 | Context/options init coverage | Closed | Dynamic trusted providers, app-name/fresh-age/env-origin, and request-aware base URL/origin coverage |
| G5 | CSRF/origin in route tests | Closed | Route-level null-origin, fetch metadata, callback URL, and origin coverage |
| G6 | Distributed rate limits | Closed | Custom/global store and hybrid denial coverage; Redis/fred crates own backend-specific smoke tests |
| G7 | `requireResourceOwnership` | Closed | `require_resource_ownership()` resolves the request session from cookies before handlers and blocks non-owners |
| G8 | Secret rotation test depth | Closed | Added tamper rejection; existing coverage includes current, old, and legacy secrets |
| G9 | Dynamic `trustedProviders` | Closed | static, global dynamic, and request-aware trusted-provider callbacks |
| G10 | Error page theming | Closed | Typed `DefaultErrorPage` customization |
| G11 | OAuth state hardening | Closed | `oauth_state` nonce/cookie validation across core social OAuth, generic OAuth, SSO OIDC, and oauth-proxy |
| G12 | Dynamic `baseURL` / async origins | Closed | Axum request-derived base URL and trusted proxy header tests; origins include static/request/env sources |
| G13 | Plugin migration bodies | Closed | `PluginMigrationBody`/plan metadata collected in context and SQL bodies executable through adapter migration hooks |
| G14 | Output field filtering on routes | Closed | user/session route JSON applies schema returnability, including plugin schema fields |
| G15 | Internal adapter admin queries | Closed | Admin queries, `refresh_user_sessions`, and update-user DB/cache refresh implemented |

## Hardening

- OAuth implicit linking in `handle_oauth_user_info` (verified-email, trusted-provider gate,
  `disable_implicit_linking` fail-closed).
- SQL `LIKE`/`ILIKE` escaping and per-segment identifier quoting.
- Production fail-closed on ambiguous deployment + default secrets.
- Chunked cookie split/join (`cookies/chunked.rs`).

## Upstream lookup

1. Pin: [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md).
2. Fetch: `./scripts/fetch-upstream-better-auth.sh` →
   `reference/upstream-src/1.6.9/repository/`.
3. Compare in-scope paths only (scope boundary table above).
4. Map by HTTP path and `*.test.ts` → `src/` and `tests/`.

### Upstream → Rust mapping

| Upstream | Rust |
| --- | --- |
| `packages/core/src/db/` | `src/db/` |
| `packages/core/src/api/` | `src/api/endpoint.rs`, `src/plugin/endpoint.rs` |
| `packages/core/src/context/` | `src/context/` |
| `packages/core/src/error/`, `env/`, `utils/` | `src/error*.rs`, `env/`, `utils/` |
| `packages/better-auth/src/api/routes/` | `src/api/routes/` |
| `packages/better-auth/src/api/middlewares/origin-check.ts` | `src/api/security.rs` |
| `packages/better-auth/src/api/middlewares/authorization.ts` | `src/api/middleware.rs` |
| `packages/better-auth/src/api/rate-limiter/` | `src/rate_limit.rs` |
| `packages/better-auth/src/api/index.ts`, `to-auth-endpoints.ts` | `src/api/router.rs`, `plugin_pipeline.rs` |
| `packages/better-auth/src/api/state/` | `src/context/request_state.rs`, `auth/oauth/state.rs` |
| `packages/better-auth/src/cookies/`, `crypto/` | `src/cookies/`, `crypto/` |
| `packages/better-auth/src/context/` | `src/context/builder.rs`, `secrets.rs` |
| `packages/better-auth/src/auth/trusted-origins.ts` | `src/auth/trusted_origins.rs` |
| `packages/better-auth/src/db/` | `src/db/`, `session.rs`, `verification.rs` |
| `packages/better-auth/src/oauth2/link-account.ts` | `src/auth/oauth/account_linking.rs` |
| `packages/better-auth/src/state.ts` | `src/auth/oauth/state.rs` |
| `packages/better-auth/src/utils/url.ts`, `get-request-ip.ts` | `src/utils/url.rs`, `ip.rs` |
| `packages/core/src/db/plugin.ts` | `src/plugin/schema.rs` |
| `packages/better-auth/src/options` (via `init-options`) | `src/options/*` |
| `packages/better-auth/src/api/routes/error.ts` + `onAPIError` | `api/on_api_error.rs`, `routes/error.rs` |

## Links

- [Crate README](./README.md)
- [Parity index](../../docs/parity/README.md)
