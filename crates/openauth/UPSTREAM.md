# Upstream parity — openauth

Better Auth **1.6.9** behavioral reference for contributors and parity audits.
OpenAuth is inspired by Better Auth; it is not a line-by-line port.

| Field | Value |
| --- | --- |
| **Parity pin** | [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md) |
| **Upstream package** | `better-auth` (public npm facade) |
| **Upstream path** | `reference/upstream-src/1.6.9/repository/packages/better-auth/src/` (`index.ts`, `auth/full.ts`, `auth/minimal.ts`, `auth/base.ts`, `package.json` `exports`) |
| **Rust crate** | `crates/openauth/` (`src/lib.rs`, `src/auth.rs`) |
| **Parity level** | **High** for core auth with default integrations; **Partial** for SAML and some product plugins |
| **Scope** | Server-side public entry crate. Out of scope: browser/React/Vue clients (`better-auth/client`, framework SDKs), CLI (`openauth-cli`), HTTP mount (`openauth-axum`), and runtime behavior owned by sibling crates listed below |

## Summary

The `openauth` crate is the application-facing facade: it re-exports
[`openauth-core`](../openauth-core/UPSTREAM.md) (builder, handler, sessions, routes)
and optional integration crates behind Cargo features. Upstream has no separate
facade package—`better-auth` is the union of core server runtime plus optional
plugins and adapters. Parity for this crate is therefore **aggregate**: route and
crypto behavior is validated in `openauth-core` and feature-specific crates;
`openauth` tests lock the public re-export surface, initializer wiring, and
feature-flag dependency boundaries.

Status symbols are defined in the [parity index](../../docs/parity/README.md#status-symbols).

## Feature parity

| Area | Status | Notes |
| --- | --- | --- |
| `betterAuth()` / options builder | ✅ | `OpenAuth::builder()`, `OpenAuthBuilder` (`src/auth.rs`); async `build()` |
| `auth.handler(Request)` | ✅ | `OpenAuth::handler` / `handler_async` delegate to `AuthRouter` |
| App-dev import surface | ✅ | `openauth::prelude`; module paths for library-author APIs (`api`, `db`, `plugin`, …) |
| Feature-gated plugins | ✅ | `plugins`, `passkey`, `sso`, `scim`, `stripe`, `i18n`, `telemetry` features |
| Feature-gated enterprise | ⚠️ | `oidc`, `saml`, `saml-signed` — SAML remains experimental |
| SQL / Postgres adapters | ✅ | `sqlx-*`, `tokio-postgres`, `deadpool-postgres` re-exports |
| Schema / migrations API | ✅ | `create_schema`, `run_migrations` on `OpenAuth` |
| OpenAPI / endpoint registry | ✅ | Re-exported from core router |
| `auth.api` programmatic caller | 🎯 | HTTP router is the Rust integration surface; no TS-style in-process API object |
| Browser / React / Vue clients | ➖ | Client-only upstream; not ported |
| Framework handlers (Next, Svelte, Node) | ➖ | [`openauth-axum`](../openauth-axum/UPSTREAM.md) and other adapter crates |

### Parity by concern (sibling crates)

| Concern | Parity crate |
| --- | --- |
| Builder, handler, sessions, accounts, routes | [`openauth-core`](../openauth-core/UPSTREAM.md) |
| Enterprise SSO (OIDC/SAML routes) | [`openauth-sso`](../openauth-sso/UPSTREAM.md) |
| OAuth/OIDC authorization server | [`openauth-oauth-provider`](../openauth-oauth-provider/UPSTREAM.md) |
| SQL / Redis persistence | [`openauth-sqlx`](../openauth-sqlx/UPSTREAM.md), [`openauth-redis`](../openauth-redis/UPSTREAM.md), … |
| Framework mount (Axum) | [`openauth-axum`](../openauth-axum/UPSTREAM.md) |
| Official plugins | [`openauth-plugins`](../openauth-plugins/UPSTREAM.md) |

## Test coverage

| Surface | OpenAuth (Rust) | Upstream | Notes |
| --- | ---: | ---: | --- |
| **Total (default features)** | **45** | — | `cargo nextest list -p openauth` |
| Public API / initializer contract | 48 | — | `tests/public_api.rs` (some `#[cfg(feature)]` gated) |
| Feature-flag dependency graph | 5 | — | `tests/feature_flags.rs` — SQLx dialect isolation, telemetry opt-in |
| Adapter DB hooks through umbrella | 3 | — | `tests/adapter_database_hooks.rs` |
| README doc example | 1 | — | `tests/docs.rs` |
| Facade `index.ts` / `auth/*.ts` Vitest | — | **0** dedicated | Upstream facade is thin; behavior tested in core + plugin packages |

```bash
cargo nextest run -p openauth
```

Route-level and adapter parity suites live in sibling crates (start with
[`openauth-core`](../openauth-core/UPSTREAM.md)).

## Intentional differences

| Topic | Better Auth 1.6.9 | OpenAuth | Why |
| --- | --- | --- | --- |
| Package layout | Single `better-auth` npm import | `openauth` facade + focused workspace crates | Smaller compile units, explicit feature flags |
| `auth.api` in-process calls | `auth.api.getSession()` etc. | HTTP `handler_async` only | Idiomatic Rust server integration |
| Optional plugins | npm subpath / plugin imports | Cargo features (`sso`, `stripe`, …) | Compile-time dependency control |
| Telemetry | On when configured in JS | `telemetry` feature; off in default build | Opt-in binary size and network |
| OIDC vs SAML deps | Bundled in SSO plugin import graph | `oidc` feature excludes SAML/XML crates | Fail-closed dependency boundaries |

## Open gaps and risks

| ID | Gap / risk | Severity | Notes |
| --- | --- | --- | --- |
| G1 | Aggregate parity only at this layer | Med | Facade tests do not replace `openauth-core` route suites |
| G2 | SAML / `saml-signed` experimental | Med | Enable only with explicit risk acceptance |
| G3 | Feature ↔ upstream import drift | Low | New upstream plugins need matching Cargo feature + re-export in `lib.rs` |
| G4 | No browser/client SDK | Low | By design; server-only workspace |
| G5 | Re-export surface vs `package.json` exports | Low | `public_api.rs` guards key symbols; audit on major bumps |

## Hardening notes

- Default build excludes telemetry and all optional plugins—enable features explicitly.
- `oidc` feature must not pull SAML/XML stacks (`feature_flags` test).
- SQLx dialect features (`sqlx-postgres`, etc.) must not enable unrelated drivers.
- Async initializers (`build_async`, `open_auth_*_async`) work without `telemetry`.
- Use durable adapters and distributed rate-limit storage for multi-instance production
  (configured through re-exported core options).

## Upstream lookup

1. Read the pin in [`reference/upstream-better-auth/VERSION.md`](../../reference/upstream-better-auth/VERSION.md).
2. Run `./scripts/fetch-upstream-better-auth.sh` if `reference/upstream-src/` is missing.
3. Open `packages/better-auth/src/` and `packages/better-auth/package.json` (`exports`).
4. Map upstream → Rust:

| Upstream | Rust |
| --- | --- |
| `packages/better-auth/src/index.ts` | `crates/openauth/src/lib.rs` |
| `packages/better-auth/src/auth/full.ts`, `auth/minimal.ts` | `crates/openauth/src/auth.rs` (`OpenAuth`, `OpenAuthBuilder`, `open_auth*`) |
| `packages/better-auth/src/auth/base.ts` (`handler`) | `OpenAuth::handler` / `handler_async` → `openauth-core` router |
| `packages/better-auth/src/plugins/*` | Feature-gated `openauth_*` re-exports |
| `packages/better-auth/src/adapters/*` | `sqlx`, `tokio-postgres`, `deadpool-postgres`, `openauth-redis`, … features |
| `packages/better-auth/src/integrations/*` | [`openauth-axum`](../openauth-axum/UPSTREAM.md) |
| Server `*.test.ts` under `better-auth/src/` | [`openauth-core`](../openauth-core/UPSTREAM.md) `tests/` (not duplicated here) |

5. Add a failing Rust test in the owning crate before behavior changes; match HTTP
   status, error codes, and DB side effects—not TypeScript types.

## Related docs

- [Crate README](./README.md) — usage and quick start
- [openauth-core UPSTREAM](../openauth-core/UPSTREAM.md) — server runtime parity
- [Parity index](../../docs/parity/README.md)
