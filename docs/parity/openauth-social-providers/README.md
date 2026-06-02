# `openauth-social-providers` parity with Better Auth

Parity reference for the Rust crate [`openauth-social-providers`](../../../crates/openauth-social-providers) against Better Auth **v1.6.9** built-in social providers.

| Field | Value |
| --- | --- |
| Upstream package | `@better-auth/core` → `packages/core/src/social-providers/` |
| Upstream re-export | `better-auth/social-providers` |
| Parity pin | [`reference/upstream-better-auth/VERSION.md`](../../../reference/upstream-better-auth/VERSION.md) |
| Local upstream tree | `reference/upstream-src/1.6.9/repository/` (gitignored; `./scripts/fetch-upstream-better-auth.sh`) |

## Scope of this document

| In scope | Out of scope (document elsewhere) |
| --- | --- |
| Built-in `socialProviders` registry (35 providers) | `better-auth` HTTP routes (`/sign-in/social`, `/callback/:id`, `/link-social`) → `openauth-core` |
| Provider factories, OAuth URLs, token exchange, profile mapping | `generic-oauth` plugin (Auth0, Okta, Keycloak, …) |
| `SocialOAuthProvider` runtime in this crate | Browser/client SDK (`signIn.social`, redirects, cookies) |
| Per-provider integration tests in this crate | `social.test.ts` end-to-end flows in upstream `better-auth` |
| Shared OAuth primitives | Mostly [`openauth-oauth`](../../../crates/openauth-oauth) (see oauth2 parity separately) |

OpenAuth is **server-only**. Client-only surfaces (React helpers, client plugins, implicit browser flows) are noted only when they affect what the server must implement.

## Documents in this folder

| File | Contents |
| --- | --- |
| [packaging.md](./packaging.md) | How upstream packages map to Rust crates; what was split or merged |
| [provider-catalog.md](./provider-catalog.md) | Wire + hooks ratings, PKCE, ID token |
| [hooks-coverage.md](./hooks-coverage.md) | `mapProfileToUser` / `getUserInfo` / refresh / verify matrix |
| [confirmed-gaps.md](./confirmed-gaps.md) | Wire/hook gaps (incl. resolved wire fixes) |
| [integration-openauth-core.md](./integration-openauth-core.md) | HTTP routes, `social.test.ts` ownership |
| [AUDIT-CHECKLIST.md](./AUDIT-CHECKLIST.md) | Per-provider checklist for future audits |
| [api-surface.md](./api-surface.md) | Trait vs `OAuthProvider`, `ProviderOptions`, registry |
| [testing.md](./testing.md) | Test counts, `social.test.ts` map, per-file inventory |
| [design-decisions.md](./design-decisions.md) | Intentional divergences (errors, SSRF, Facebook verify, …) |
| [providers/complex.md](./providers/complex.md) | Deep notes: Google, Apple, GitHub, Microsoft, WeChat, Cognito, Linear, Atlassian, Twitter, Facebook |
| [providers/standard-oauth.md](./providers/standard-oauth.md) | Remaining 25 providers grouped by OAuth pattern |

## Executive summary (2026-06-01, deep pass)

Audit: all **35** upstream `*.ts` + Rust `src/*.rs` + `tests/*.rs`; cross-check `social.test.ts`.

| Dimension | Upstream (1.6.9) | OpenAuth |
| --- | --- | --- |
| Built-in providers | **35** | **35** — `PROVIDER_IDS` matches `index.ts` order |
| Provider unit tests | **0** in `social-providers/` | **310** in crate (+ **7** atlassian in `src/`) |
| Social E2E | `social.test.ts` (**40** `it`) | None here → `openauth-core` |
| **Wire parity** (URLs, scopes, default path) | — | **33/35** full; **1** div (facebook opaque verify); **1** extra (twitch JWKS) |
| **Hook parity** (`ProviderOptions` callbacks) | Hook check sites on **34/35** providers | Typed overrides on **10/35** (architectural; see hooks doc) |

Wire fixes applied: discord/roblox `+` scopes, railway optional PKCE, paypal default `sub` verify. See [confirmed-gaps.md](./confirmed-gaps.md).

**Stopping point:** Further work belongs in `openauth-core` (E2E from `social.test.ts`) or a deliberate hooks API on `ProviderOptions` — not more provider-file audit.

Registry keys and order match upstream `packages/core/src/social-providers/index.ts` and `PROVIDER_IDS` in `src/lib.rs`.

## How to refresh this audit

```bash
./scripts/fetch-upstream-better-auth.sh
# Compare:
#   reference/upstream-src/1.6.9/repository/packages/core/src/social-providers/
#   crates/openauth-social-providers/src/
```

When bumping the parity pin, re-run provider-by-provider diff and update `provider-catalog.md` ratings.
