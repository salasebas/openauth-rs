# 08 — Server gap closeout (Jun 2026)

Second implementation round after [07-deep-audit.md](./07-deep-audit.md). Goal: close **authorization server behavior** gaps with clear value; document what remains N/A or low ROI.

## Closed in code (Jun 2026)

| Gap | Solution |
| --- | --- |
| UserInfo missing `given_name` / `family_name` | `userinfo.rs` uses `user_normal_claims` |
| `/.well-known/oauth-authorization-server` with `openid` | Returns OIDC metadata (same as upstream) |
| Hardcoded `id_token_signing_alg` / remote JWKS | `advertised_*` + `jwks_path`; `oauth_provider_with_jwt()` fills from `JwtOptions` |
| Token / DCR without `no-store` | `no_store_json_response` on `/oauth2/token` and `POST /oauth2/register` |
| SPA redirect (`fetch` / `Accept: application/json`) | `redirect_or_json_response` → `{ "redirect": true, "url": "..." }` |
| `prompt=none` + `shouldRedirect` | `signup_should_redirect`, `select_account_should_redirect`, `post_login_should_redirect` |
| `postLogin.consentReferenceId` | `consent_reference_id` resolver in authorize/consent |
| Revoke body | `empty_success_response()` (empty body, not JSON `null`) |
| Exportable metadata | `oauth_authorization_server_metadata`, `well_known_metadata_response`, `WELL_KNOWN_METADATA_CACHE_CONTROL` |

## Tests added (96 total)

- `oauth_authorization_server_returns_oidc_metadata_when_openid_enabled`
- `oauth_provider_jwt_plugin_options_fill_advertised_metadata_defaults`
- `authorize_prompt_none_returns_account_selection_required_when_needed`
- `authorize_prompt_none_returns_interaction_required_for_post_login`
- `authorize_json_accept_returns_redirect_payload_instead_of_302`
- `authorize_success_redirect_includes_iss_parameter`
- `token_endpoint_sets_no_store_cache_headers`
- `refresh_token_grant_rejects_scope_not_in_original_grant`
- `revoke_endpoint_returns_empty_body_on_success`
- UserInfo: asserts `given_name` / `family_name` with `profile` scope

## Not implemented (stop here — low value / out of crate)

| Topic | Reason |
| --- | --- |
| `@better-auth/oauth-provider/client` | Browser SDK; server N/A |
| Full `resource-client` (`externalScopes`, `remoteVerify`) | TS SDK; partial helpers in `mcp.rs` |
| `getOAuthProviderState` / global `oauth_query` hooks | Different model: `request_id` + verification store |
| `schema` merge in plugin | Not required for OAuth protocol |
| `silenceWarnings` | Upstream DX only |
| Custom `storeClientSecret` `{encrypt, decrypt}` | Only `symmetric_*`; use `hash`/`verify` callbacks |
| `scopeExpirations` with strings like `"5 minutes"` | `u64` seconds only; sufficient in Rust |
| Dynamic base URL in metadata wrappers | Depends on `AuthContext` / per-request host in integration |
| Rate limit 429 E2E | Rules registered; enforcement in `openauth-core` |
| 261 upstream `it` 1:1 | ~50% scenarios with explicit test; rest covered indirectly or N/A |
| MCP SDK E2E | Crate `openauth-plugins::mcp` |
| OIDC query passthrough (`display`, `login_hint`, …) | Optional; only `max_age` has effect today |
| Separate PKCE “authorize only” / “token only” tests | Policy already covered by existing PKCE tests |
| Admin `require_pkce` persist test | Low ROI |

## New public API

- `oauth_provider_with_jwt(options, jwt_options)`
- `PromptShouldRedirectResolver`, `consent_reference_id`
- `advertised_jwks_uri`, `advertised_id_token_signing_algorithms`, `jwks_path`
- `oauth_authorization_server_metadata`, `well_known_metadata_response`

## Verification

```bash
cargo fmt --all --check
cargo clippy -p openauth-oauth-provider --all-targets -- -D warnings
cargo nextest run -p openauth-oauth-provider
```

Status: **96 tests**, all passing after this closeout.
