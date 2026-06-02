# Parity: `openauth-oauth-provider` ↔ `@better-auth/oauth-provider`

**Server-only** parity documentation between OpenAuth and Better Auth **v1.6.9**.

| Field | Value |
| --- | --- |
| Upstream npm | `@better-auth/oauth-provider@1.6.9` |
| Upstream path | `reference/upstream-src/1.6.9/repository/packages/oauth-provider/` |
| Rust crate | `crates/openauth-oauth-provider` (`openauth-oauth-provider` on crates.io) |
| Parity pin | [`reference/upstream-better-auth/VERSION.md`](../../../reference/upstream-better-auth/VERSION.md) |
| Historical checklist | [`docs/superpowers/plans/2026-05-12-oauth-provider-upstream-checklist.md`](../../superpowers/plans/2026-05-12-oauth-provider-upstream-checklist.md) |
| Closeout plan | [`docs/superpowers/plans/2026-05-17-oauth-provider-parity-closeout.md`](../../superpowers/plans/2026-05-17-oauth-provider-parity-closeout.md) |
| Test matrix (legacy in crate) | [`crates/openauth-oauth-provider/tests/upstream_mapping.md`](../../../crates/openauth-oauth-provider/tests/upstream_mapping.md) |

## Package relationship (not 1:1 with the whole OAuth ecosystem)

| Role | Upstream Better Auth | OpenAuth |
| --- | --- | --- |
| Authorization server (OAuth 2.1 / OIDC) | `@better-auth/oauth-provider` | **`openauth-oauth-provider`** (this crate) |
| OAuth **client** / social / PKCE helpers | Inside `better-auth` (`better-auth/oauth2`, plugins) | **`openauth-oauth`** + `openauth-social-providers` |
| JWT / JWKS for the AS | `better-auth/plugins/jwt` (runtime merge) | **`openauth-plugins::jwt`** (merged from `oauth_provider()` when `disable_jwt_plugin = false`) |
| Browser: inject `oauth_query` into fetch | `@better-auth/oauth-provider/client` | **Not ported** (server-only) |
| Resource server: verify bearer | `@better-auth/oauth-provider/resource-client` | **`mcp` module** + validation in `token`; no TS client SDK |
| Full MCP server | Tests with `@modelcontextprotocol/sdk` | **`openauth-plugins::mcp`** (separate crate; `/mcp/*` routes) |

**Upstream split vs OpenAuth:** Better Auth ships the AS in one npm package but spreads **OAuth client**, **JWT**, and **login hooks** across the `better-auth` monorepo. OpenAuth mirrors that with separate crates; *authorization server behavior* is documented here, not in `openauth-oauth`.

## Index

| Document | Contents |
| --- | --- |
| [01-overview.md](./01-overview.md) | Executive summary, scope, parity status |
| [02-package-mapping.md](./02-package-mapping.md) | Module ↔ upstream file map, dependencies, schema |
| [03-endpoints.md](./03-endpoints.md) | HTTP inventory (25 upstream / 26 Rust), auth, admin |
| [04-features-and-options.md](./04-features-and-options.md) | Grants, storage, prompts, metadata, hooks |
| [05-design-decisions.md](./05-design-decisions.md) | Intentional divergences and known gaps |
| [06-tests.md](./06-tests.md) | Vitest ↔ Rust counts, matrix by upstream test file |
| [07-deep-audit.md](./07-deep-audit.md) | **Code + source audit** (Jun 2026): confirmed gaps, exports, options |
| [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md) | **Server gap closeout** (Jun 2026): what was implemented and where to stop |

## Quick verification

```bash
cargo fmt --all --check
cargo clippy -p openauth-oauth-provider --all-targets -- -D warnings
cargo nextest run -p openauth-oauth-provider
```

| Metric | Upstream (Vitest) | OpenAuth (`openauth-oauth-provider`) |
| --- | --- | --- |
| `*.test.ts` files | 18 (co-located under `src/`) | 6 modules under `tests/oauth_provider/` + harness |
| `describe(` | 58 | — |
| `it(` | **261** | — |
| `#[test]` | — | 9 |
| `#[tokio::test]` | — | 87 |
| **Total Rust tests** | — | **96** |

## Server status summary

| Area | Parity with BA 1.6.9 | Notes |
| --- | --- | --- |
| OAuth/OIDC endpoints (authorize → userinfo) | **High** | 25 upstream routes; Rust exposes the same + extra `GET /oauth2/continue` |
| DCR + client/consent management | **High** | Admin + session; ownership and `client_privileges` |
| Token grants (code, refresh, M2M) | **High** | PKCE, rotation, replay, `resource` → JWT |
| Introspection / revocation | **High** | RFC 7662/7009; client auth required |
| Metadata / discovery | **High** | OIDC on oauth-authorization-server when `openid`; JWKS/alg via `advertised_*` or `oauth_provider_with_jwt` |
| Pairwise `sub` | **High** | Sector from host:port in DCR |
| Prompts `account_selection_required` / `interaction_required` | **High** | `*_should_redirect` + `prompt=none` |
| UserInfo `given_name` / `family_name` | **High** | Same logic as ID token |
| MCP helpers | **Partial** | Rust: `src/mcp.rs` without routes; upstream: `mcpHandler` + SDK tests |
| Post-login hooks (`oauth_query`, cookies) | **Partial / N/A** | Logic split across core + explicit continue flows |
| `client.ts` / `resource-client` | **N/A** | TS/browser; see [05-design-decisions.md](./05-design-decisions.md) |
| Tests (count) | 261 `it` | 96 tests; server gaps closed — see [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md) |

Last audit: [07-deep-audit.md](./07-deep-audit.md). Server closeout: **2026-06-02** — [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md).
