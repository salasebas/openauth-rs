# 05 — Design decisions and gaps

Classification: **intentional** (server-only / Rust / security), **partial** (same intent, different shape), **gap** (upstream behavior not yet replicated).

> **Jun 2026:** Server gaps listed in [07-deep-audit.md](./07-deep-audit.md) § second pass were **closed** in code. See [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md). Tables below keep historical notes where useful.

## Server-only: not ported by scope

| Upstream | Why not in `openauth-oauth-provider` | OpenAuth alternative |
| --- | --- | --- |
| `@better-auth/oauth-provider/client` (`oauthProviderClient`) | Injects `window.location.search` into **browser** fetch | Host app builds redirects/login; prelogin validates signed `oauth_query` |
| `@better-auth/oauth-provider/resource-client` | **Client** SDK for API consumers | `validate_access_token`, `mcp::*`, or call `/oauth2/introspect` |
| TypeScript `BetterAuthPluginRegistry` inference | TS-only | Rust traits and types in `options` |
| `createAuthEndpoint` + OpenAPI | BA framework | `openauth_core::api::create_auth_endpoint` |
| Tests with `listhen`, Generic OAuth, organization mounted | Monorepo E2E | Tests with `MemoryAdapter` + minimal router |

**Conclusion:** missing `/client` subpath is **not a bug**; it matches OpenAuth as a **server** stack.

## Intentional Rust / architecture decisions

| Topic | Upstream | OpenAuth | Reason |
| --- | --- | --- | --- |
| DB table names | BA logical camelCase | `oauth_clients`, etc. | SQL/Rust convention |
| Config errors | `BetterAuthError` runtime | `OAuthProviderConfigError` enum | Idiomatic Rust |
| Admin update HTTP | `PATCH /admin/oauth2/update-client` | `POST` same relative path | OpenAuth router; equivalent body |
| Continue | `POST` only | `GET` + `POST` | Easier browser redirects |
| OAuth state | `defineRequestState` | JSON in `verification` + pending token | No global per-request state in core |
| Prompt UX config | `signup`, `selectAccount`, `postLogin` objects | URLs + optional `*_redirect` / `*_should_redirect` resolvers | Explicit flat API |
| Encrypted token storage | Supported for tokens | **Rejected** at runtime | Simpler crypto surface; hashed only |
| MCP | `mcpHandler` coupled to BA HTTP | `mcp` library module | Separate AS vs resource server crates |
| JWT plugin | Implicit peer | `openauth-plugins::jwt` explicit merge | Same split as BA monorepo |

## Partial parity (same behavior, different mechanism)

| Behavior | Upstream | OpenAuth | Notes |
| --- | --- | --- | --- |
| Resume OAuth after login | `after` hook parses cookie and re-calls authorize | App calls `/oauth2/continue` with flags | Requires login integration; not automatic |
| `oauth_query` on sign-in | `before` hook on `/sign-in/*` | Validated on `public-client-prelogin` only | Apps must propagate query to login if needed |
| JSON redirect vs 302 after login | `Sec-Fetch-Mode` / `Accept: text/html` | `redirect_or_json_response` for fetch / JSON Accept | SPA clients can avoid 302 |
| Remote resource verify | `remoteVerify` in resource-client | HTTP introspect from the app | No TS wrapper |
| Protected resource well-known URL | Generated in challenge | Path `/.well-known/oauth-protected-resource{suffix}` in challenge | Not always registered on AS |

## Known server gaps (Jun 2026 audit → closeout)

See [07-deep-audit.md](./07-deep-audit.md) for evidence. Status after [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md):

| Gap | Severity (audit) | Status |
| --- | --- | --- |
| `prompt=none` → `account_selection_required` | Medium | **Closed** (`select_account_should_redirect`) |
| `prompt=none` → `interaction_required` | Medium | **Closed** (`signup_should_redirect`, `post_login_should_redirect`) |
| UserInfo missing `given_name`/`family_name` | Medium | **Closed** |
| `postLogin.consentReferenceId` | Low | **Closed** (`consent_reference_id`) |
| `/.well-known/oauth-authorization-server` = OIDC when `openid` | High | **Closed** |
| Token/DCR `Cache-Control: no-store` | Medium | **Closed** |
| Fetch redirect → `{ redirect, url }` | Medium | **Closed** |
| `id_token_signing_alg` hardcoded | Medium | **Closed** (`advertised_*`, `oauth_provider_with_jwt`) |
| PAR endpoint | Low | Resolver only; no PAR storage |
| `getOAuthProviderState` | Low | Pending in `verification` |
| MCP SDK E2E | Low | `openauth-plugins::mcp` |
| `login_hint`, `display`, etc. on authorize | Low | Optional OIDC params; only `max_age` effective |
| Rate limit 429 E2E | Low | Rules registered in core |
| JWT introspect test suite | Medium | Code supports JWT; tests mostly JWT off |

## Rust improvements vs upstream (documented, not gaps)

| Improvement | Notes |
| --- | --- |
| Typed OAuth errors | `OAuthProviderError` → status + body |
| Explicit rejection of encrypted token storage | Fewer modes = fewer footguns |
| Ownership / consent scope tests | Security reinforcement |
| `refresh_token_replay_revokes_refresh_token_family` | Explicit upstream parity |

## When to use another OpenAuth crate

| Need | Crate |
| --- | --- |
| Consume Google/GitHub as IdP | `openauth-social-providers`, `openauth-oidc`, `openauth-sso` |
| Your app **issues** OAuth tokens | **`openauth-oauth-provider`** |
| MCP server with HTTP routes | `openauth-plugins::mcp` |
| Generic OAuth client (PKCE, token exchange helpers) | `openauth-oauth` |

## Quick reference: classify a difference

1. In `client.ts` or `resource-client` as a **client plugin**? → **N/A** server-only.
2. Global Better Auth hook (cookies, sign-in)? → **Partial** / integrate in app + `continue`.
3. DB naming or Rust type? → **Intentional**.
4. Same OAuth endpoint and body? → Should be **high parity**; if not, it is a real **gap**.
