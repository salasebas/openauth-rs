# 06 — Tests and coverage

## Global counts

| Métrica | Upstream `@better-auth/oauth-provider` | OpenAuth `openauth-oauth-provider` |
| --- | --- | --- |
| Test files | 18 (`src/**/*.test.ts`) | 6 módulos + `tests/oauth_provider.rs` |
| `describe(` | 58 | — |
| `it(` | **261** | — |
| `#[test]` | — | 9 |
| `#[tokio::test]` | — | 78 |
| **Total Rust tests** | — | **96** |
| Ratio `it` → Rust | — | ~**37%** por conteo; muchos `it` upstream son variantes del mismo escenario |

Upstream framework: **Vitest**. OpenAuth: **integration** con `tokio`, `AuthRouter`, `MemoryAdapter`.

## Distribution by file

### Upstream

| Archivo test | `it(` aprox. | Main topic |
| --- | --- | --- |
| `oauth.test.ts` | 37 | Init, hooks login, prompts, rate limits, config |
| `token.test.ts` | 38 | Grants, PKCE, JWT/opaco, custom claims, prefixes |
| `authorize.test.ts` | 18 | Issuer, PAR, `prompt=none`, redirects |
| `register.test.ts` | 19 | DCR público/confidencial, metadata |
| `pairwise.test.ts` | 18 | Sector, ID token vs userinfo, DCR |
| `pkce-optional.test.ts` | 11 | PKCE opcional confidential |
| `oauthClient/endpoints.test.ts` | 10 | CRUD cliente |
| `oauthClient/endpoints-privileges.test.ts` | 16 | `clientPrivileges` |
| `metadata.test.ts` | 15 | Discovery, JWT off, protected resource |
| `introspect.test.ts` | 14 | JWT/opaco/refresh introspect |
| `types/zod.test.ts` | 14 | Safe redirect URL |
| `revoke.test.ts` | 11 | Revocación |
| `userinfo.test.ts` | 9 | Scopes, bearer |
| `logout.test.ts` | 7 | RP logout |
| `oauthConsent/endpoints.test.ts` | 6 | Consent API |
| `utils/query-serialization.test.ts` | 8 | Query arrays, delete prompt |
| `mcp.test.ts` | 4 | MCP challenge + flow SDK |
| `utils/timestamps.test.ts` | 6 | Normalización tiempo |

### OpenAuth

| Archivo test | Tests | Topic |
| --- | --- | --- |
| `authorization.rs` | 26 | Authorize, PKCE, prompts, PAR, loopback, openid+JWKS |
| `tokens.rs` | 27 | Token, refresh, introspect, revoke, resource JWT, custom claims |
| `clients.rs` | 17 | DCR, ownership, privileges, prelogin, trusted clients |
| `config_metadata.rs` | 13 | Defaults, schema, rate limits, config errors, MCP metadata URL |
| `consent.rs` | 7 | Consent helpers + endpoints + management |
| `oidc_misc.rs` | 6 | Pairwise, MCP helpers, RP logout |

## Upstream → Rust matrix (por archivo de test upstream)

Legend: **Yes** = scenario covered by at least one named Rust test; **Partial**; **No** = not applicable or no test; **Indirect** = no dedicated unit test.

### `authorize.test.ts`

| Upstream scenario | Rust | OpenAuth test(s) |
| --- | --- | --- |
| Issuer URL validation | Partial | Implicit in redirects |
| Login redirect without session | Yes | `authorize_prompt_none_returns_login_required_without_session` |
| `prompt=none` → `login_required` | Yes | same |
| PAR `request_uri` | Yes | `authorize_resolves_request_uri_parameters`, `authorize_request_uri_resolver_handles_origin_form_requests` |
| PAR ignores front-channel params | Partial | `authorize_rejects_unallowed_scope_and_request_uri_client_mismatch` |
| Redirect with `iss` | Partial | Flujos authorize (no assert dedicado `iss`) |
| Metadata issuer = authorize `iss` | No | — |
| `prompt=none` + consent → `consent_required` | Yes | `authorize_prompt_none_returns_consent_required_without_grant` |

### `oauth.test.ts`

| Upstream scenario | Rust | Notes |
| --- | --- | --- |
| Init requires JWT | Partial | `disable_jwt_plugin` en tests; merge JWT en `oauth_provider()` |
| Secondary storage + DB sessions | No | Core / otro doc |
| Dynamic base URL init | No | — |
| Generic OAuth sign-in integration | No | Out of crate |
| Fetch login → JSON redirect | No | N/A browser |
| Navigate → HTTP redirect | Indirect | Redirects en authorize |
| Client disabled mid-flow | Partial | No test dedicado |
| Prompts login/create/consent/select/none/post-login | Yes | Varios `authorize_prompt_*`, `authorize_post_login_*` |
| Config validation scopes/grants/storage | Yes | `oauth_provider_rejects_*` en config_metadata |
| Rate limits default/custom/disabled | Yes | `oauth_provider_contributes_default_rate_limit_rules`, `oauth_provider_rate_limit_options_*` |

### `token.test.ts`

| Upstream scenario | Rust | Test(s) |
| --- | --- | --- |
| Auth code + scopes openid/profile/email/offline | Yes | `authorization_code_flow_issues_access_and_refresh_tokens` |
| Without `state` | Indirect | — |
| JWT vs opaco por `resource` | Yes | `resource_parameter_*` |
| Refresh same/narrower scope | Partial | Rotación sí; narrow scope menos explícito |
| Refresh replay | Yes | `refresh_token_replay_revokes_refresh_token_family` |
| Client credentials JWT/opaco | Partial | Opaco sí; JWT con resource tests |
| Custom ID token claims precedence | Yes | `custom_id_token_claims_and_token_response_fields_are_added` |
| Prefixes | Yes | `prefixes_and_custom_generators_*` |
| Encrypted secret mismatch | Partial | Encrypted secret mode; no suite error decrypt |
| Loopback redirect en token exchange | Yes | `authorize_loopback_matching_*` |
| Custom token response fields | Yes | `custom_id_token_claims_*`, failure test |
| Verification value schema | Indirect | Código consume `AuthorizationCodeValue` |

### `pkce-optional.test.ts`

| Scenario | Rust |
| --- | --- |
| Public always requires PKCE | Yes — `authorization_code_flow_enforces_pkce_s256_for_public_clients` |
| Confidential default PKCE | Yes — `authorization_code_flow_enforces_upstream_pkce_policy_for_confidential_clients` |
| Confidential opt-out | Yes — mismo test |
| `offline_access` siempre PKCE | Yes — en policy test |
| Challenge mismatch | Yes — `authorization_code_flow_rejects_spurious_pkce_verifier` |
| Admin `require_pkce` persist | Partial | Implícito en client create |

### `introspect.test.ts` / `revoke.test.ts`

| Scenario | Rust |
| --- | --- |
| No client auth → failure | Yes — `introspect_and_revoke_require_valid_client_authentication` |
| JWT / opaco / refresh | Partial | Opaco+refresh sí; JWT con plugin en flujo openid |
| Wrong hint | Yes — `introspect_and_revoke_respect_token_type_hint` |
| No hint | Yes — mismo test |
| Prefixes | Partial | `prefixes_and_custom_generators_*` |
| User logged out, token introspectable | Partial | No test dedicado |

### `userinfo.test.ts`

| Scenario | Rust |
| --- | --- |
| No bearer | Partial | — |
| Requires `openid` | Yes — `userinfo_rejects_tokens_without_openid_scope` |
| Opaco / JWT | Partial | Pairwise test usa userinfo |
| Server API headers only | **No** | Documented gap |
| Scope-filtered claims | Yes — `userinfo_returns_claims_by_explicit_openid_profile_and_email_scopes` |

### `logout.test.ts`

| Scenario | Rust |
| --- | --- |
| Invalid `id_token_hint` | Yes — `rp_initiated_logout_rejects_invalid_id_token_hint` |
| DCR no `enable_end_session` | Yes — `dynamic_registration_cannot_enable_rp_initiated_logout` |
| Client sin enable | Yes — `rp_initiated_logout_rejects_clients_without_end_session_enabled` |
| Logout + redirect | Yes — `rp_initiated_logout_deletes_session_and_redirects_to_registered_uri` |
| JWT plugin disabled logout | Partial | Tests usan `disable_jwt_plugin: true` |

### `metadata.test.ts`

| Scenario | Rust |
| --- | --- |
| OIDC + OAuth metadata | Yes — `metadata_endpoint_returns_oidc_server_metadata` |
| Advertised scopes/claims | Yes — `metadata_endpoint_advertises_custom_claims_supported` |
| Remote JWKS | **No** | — |
| JWT disabled metadata | Partial | Código en `metadata.rs`; pocos asserts |
| Dynamic base URL wrappers | **No** | — |
| Protected resource validation | Partial | `oauth_provider_mcp_protected_resource_metadata_rejects_invalid_resource_urls` |

### `register.test.ts`

| Scenario | Rust |
| --- | --- |
| Body / auth failures | Partial | `dynamic_registration_rejects_invalid_client_metadata` |
| Type/grant/response validation | Yes | same + unsafe redirects |
| Public vs confidential | Yes — `dynamic_registration_creates_confidential_client_and_hashes_secret` |
| Metadata strip unknown | Partial | — |
| Unauthenticated DCR PKCE flow | Partial | — |
| Organization reference | Yes — `client_reference_owns_clients_and_flows_into_tokens` |
| skip_consent blocked | Partial | En DCR logout test |

### `oauthClient/endpoints.test.ts` + privileges

| Scenario | Rust |
| --- | --- |
| create/get/public/prelogin/list/update/rotate/delete | Yes — `clients.rs` |
| Cannot go public via update | Partial | `update_client_rejects_token_auth_method_changes` |
| Secret not updatable via update | Partial | — |
| Privileges all actions | Yes — `client_privileges_can_deny_client_crud_actions` |

### `oauthConsent/endpoints.test.ts`

| Scenario | Rust |
| --- | --- |
| CRUD consent | Yes — `consent_management_endpoints_enforce_owner_session` |
| Update rejects scopes > client | Yes — `update_consent_rejects_scopes_not_allowed_for_client` |

### `pairwise.test.ts`

| Scenario | Rust |
| --- | --- |
| Cross-RP unlinkability | Partial | Sector test cubre distintos hosts |
| Determinism same client | Yes — `pairwise_subject_is_stable_by_sector_*` |
| Public subject fallback | Partial | — |
| ID token + userinfo consistency | Partial | userinfo en pairwise test |
| Introspect pairwise opaco | Yes | mismo test |
| JWT sub real user id | **No** | — |
| DCR pairwise validation | Yes — `pairwise_registration_requires_single_redirect_sector` |

### `mcp.test.ts`

| Scenario | Rust |
| --- | --- |
| Challenge header | Yes — `mcp_helpers_return_metadata_challenge_and_validate_bearer_tokens` |
| Bad token → challenge | Partial | inactive bearer |
| Full MCP OAuth + DCR SDK flow | **No** | Server-only helpers |

### `types/zod.test.ts`

| Scenario | Rust |
| --- | --- |
| Safe redirect URL | Yes — `dynamic_registration_rejects_unsafe_redirect_urls`, `dynamic_registration_allows_https_loopback_and_custom_scheme_redirects` |

### `utils/query-serialization.test.ts` / `timestamps.test.ts`

| Scenario | Rust |
| --- | --- |
| Repeated query params | Indirect | State en redirects / verification |
| Prompt deletion | Indirect | Continue flow |
| Timestamp normalization | Indirect | Expiry en tokens/consent |

## Full Rust test list (96)

<details>
<summary>config_metadata.rs (13)</summary>

- `oauth_provider_uses_upstream_default_scopes_grants_and_expirations`
- `oauth_provider_contributes_default_rate_limit_rules`
- `oauth_provider_rate_limit_options_override_and_disable_endpoint_rules`
- `oauth_provider_contributes_plural_snake_case_schema`
- `oauth_provider_mcp_protected_resource_metadata_rejects_invalid_resource_urls`
- `metadata_endpoint_returns_oidc_server_metadata`
- `metadata_endpoint_advertises_custom_claims_supported`
- `oauth_provider_rejects_client_registration_scopes_not_in_server_scopes`
- `oauth_provider_rejects_refresh_token_without_authorization_code_grant`
- `oauth_provider_rejects_short_pairwise_secret`
- `oauth_provider_rejects_hashed_client_secrets_without_jwt_plugin`
- `oauth_provider_jwt_plugin_options_fill_advertised_metadata_defaults`

</details>

<details>
<summary>authorization.rs (26)</summary>

- `authorization_code_flow_issues_access_and_refresh_tokens`
- `authorization_code_flow_defaults_missing_scope_to_client_scopes`
- `authorization_code_flow_enforces_pkce_s256_for_public_clients`
- `authorization_code_flow_enforces_upstream_pkce_policy_for_confidential_clients`
- `authorization_code_flow_rejects_spurious_pkce_verifier`
- `authorization_code_flow_requires_active_session_and_user_before_tokens`
- `authorize_loopback_matching_uses_ip_literals_only`
- `authorize_prompt_none_returns_login_required_without_session`
- `authorize_prompt_none_returns_consent_required_without_grant`
- `authorize_prompt_none_rejects_supported_prompt_combinations`
- `authorize_ignores_unknown_prompt_values`
- `authorize_request_uri_resolver_handles_origin_form_requests`
- `openid_authorization_code_issues_signed_id_token_and_jwks`
- `authorize_resolves_request_uri_parameters`
- `authorize_rejects_unallowed_scope_and_request_uri_client_mismatch`
- `authorize_max_age_zero_forces_login_redirect`
- `authorize_prompt_create_redirects_to_signup_page`
- `authorize_prompt_create_continue_issues_code_when_session_exists`
- `authorize_prompt_select_account_redirects_to_select_account_page`
- `authorize_prompt_select_account_continue_issues_code`
- `authorize_post_login_continue_issues_code`
- `authorize_post_login_redirect_callback_can_choose_custom_page`
- `oauth_authorization_server_returns_oidc_metadata_when_openid_enabled`
- `authorize_prompt_none_returns_account_selection_required_when_needed`
- `authorize_prompt_none_returns_interaction_required_for_post_login`
- `authorize_json_accept_returns_redirect_payload_instead_of_302`
- `authorize_success_redirect_includes_iss_parameter`

</details>

<details>
<summary>tokens.rs (27)</summary>

- `token_endpoint_missing_grant_type_returns_unsupported_grant_type`
- `oauth_token_endpoints_return_oauth_json_for_malformed_basic_auth`
- `client_credentials_token_returns_bearer_token_and_persists_opaque_token`
- `client_credentials_rejects_oidc_scopes`
- `token_endpoint_prefers_basic_auth_over_body_credentials`
- `token_endpoint_rejects_expired_client_secret`
- `refresh_token_grant_rotates_and_revokes_previous_refresh_token`
- `refresh_token_replay_revokes_refresh_token_family`
- `introspect_and_revoke_require_valid_client_authentication`
- `introspect_and_revoke_respect_token_type_hint`
- `resource_parameter_issues_jwt_access_token_with_oauth_claims`
- `resource_array_issues_jwt_access_token_with_multiple_audiences`
- `resource_form_repeated_issues_multi_audience_and_invalid_json_resource_is_rejected`
- `custom_id_token_claims_and_token_response_fields_are_added`
- `custom_token_response_failure_does_not_persist_tokens`
- `custom_access_and_userinfo_claims_are_added`
- `userinfo_returns_claims_by_explicit_openid_profile_and_email_scopes`
- `userinfo_rejects_tokens_without_openid_scope`
- `scope_expirations_use_shortest_matching_scope`
- `client_credentials_uses_default_scopes_when_client_has_no_scopes`
- `prefixes_and_custom_generators_are_applied_without_storing_prefixes`
- `format_refresh_token_wraps_returned_token_and_decodes_refresh_grant`
- `custom_store_hash_callbacks_are_used_for_client_secrets_and_tokens`
- `resource_parameter_rejects_unconfigured_audience`
- `token_endpoint_sets_no_store_cache_headers`
- `refresh_token_grant_rejects_scope_not_in_original_grant`
- `revoke_endpoint_returns_empty_body_on_success`

</details>

<details>
<summary>clients.rs (17)</summary>

- `dynamic_registration_creates_confidential_client_and_hashes_secret`
- `dynamic_registration_uses_default_scopes_and_configured_secret_expiration`
- `dynamic_registration_confidential_client_secret_does_not_expire_by_default`
- `dynamic_registration_rejects_invalid_client_metadata`
- `dynamic_registration_rejects_unsafe_redirect_urls`
- `dynamic_registration_allows_https_loopback_and_custom_scheme_redirects`
- `client_reference_owns_clients_and_flows_into_tokens`
- `client_privileges_can_deny_client_crud_actions`
- `public_client_prelogin_requires_allow_flag_and_signed_oauth_query`
- `cached_trusted_clients_reject_manual_update_delete_and_rotate`
- `cached_trusted_clients_reuse_cached_db_client_on_later_reads`
- `dynamic_registration_cannot_enable_rp_initiated_logout`
- `client_management_endpoints_reject_cross_user_ownership`
- `rotate_secret_rejects_public_clients`
- `update_client_preserves_omitted_fields`
- `update_client_rejects_token_auth_method_changes`
- `update_client_rejects_invalid_scope`

</details>

<details>
<summary>consent.rs (7)</summary>

- `consent_helpers_persist_update_delete_and_match_scopes`
- `consent_endpoint_accepts_rejects_and_continue_without_flag_is_rejected`
- `consent_endpoint_accepts_subset_and_rejects_unrequested_scope`
- `continue_requires_matching_prompt_flag_and_rechecks_consent`
- `consent_management_endpoints_enforce_owner_session`
- `update_consent_rejects_scopes_not_allowed_for_client`
- `update_consent_without_scopes_preserves_existing_scopes`

</details>

<details>
<summary>oidc_misc.rs (6)</summary>

- `pairwise_subject_is_stable_by_sector_and_used_for_userinfo_and_introspection`
- `pairwise_registration_requires_single_redirect_sector`
- `mcp_helpers_return_metadata_challenge_and_validate_bearer_tokens`
- `rp_initiated_logout_rejects_invalid_id_token_hint`
- `rp_initiated_logout_deletes_session_and_redirects_to_registered_uri`
- `rp_initiated_logout_rejects_clients_without_end_session_enabled`

</details>

## Upstream `it(` inventory without dedicated Rust test (muestra representativa)

List extracted with `grep '\bit('` sobre los 18 archivos. **No** does not mean production behavior is missing — only that there is no Rust test with the same name/scenario.

### `oauth.test.ts` (37) — mostly no Rust test

| Upstream test | Rust |
| --- | --- |
| JWT plugin init / secondaryStorage / dynamic baseURL | No |
| Generic OAuth sign-in / discovery | No |
| JSON redirect vs 302 (fetch, Sec-Fetch, HTML accept) | No (N/A server) |
| Client deleted/disabled → JSON error redirect | No |
| Todos los prompts compuestos (login+consent, select+consent, post-login org) | Partial (subset en `authorization.rs`) |
| Form body pre-parsed en token | No |
| Rate limit enforcement 429 | No (solo config rules) |

### `token.test.ts` (38)

| Upstream test | Rust |
| --- | --- |
| Scope combinations openid/profile/email/offline (parametrizados) | Partial (one main flow) |
| Exchange without `state` | No |
| Refresh lesser scopes / JWT+opaque variants | No |
| Refresh removing `offline_access` | No |
| `auth_time` preserved after refresh | No |
| Custom claims pinned vs override (detailed) | Partial |
| Prefixes 3 separate tests | Un test combinado |
| Encrypted secret mismatch / custom decrypt error | No |
| Verification value zod (5 tests) | No |
| Loopback port/path (4 tests) | Partial (`authorize_loopback_*`) |

### `authorize.test.ts` (18)

| Upstream test | Rust |
| --- | --- |
| Issuer URL unit tests (10) | No (lógica en `validate_issuer_url`) |
| PAR sign resolved params | Partial |
| `iss` + metadata issuer match | No |
| consent_required prompt=none | Yes |

### `pairwise.test.ts` (18)

| Upstream test | Rust |
| --- | --- |
| Cross-RP unlinkability | Partial |
| Public sub fallback | No |
| JWT access sub = real user | No |
| subject_types_supported metadata | No |
| pairwiseSecret length (sync) | Yes (config error) |

### `pkce-optional.test.ts` (11)

| Upstream test | Rust |
| --- | --- |
| PKCE only auth / only token | No |
| Admin create `require_pkce` | No |

### `introspect.test.ts` + `revoke.test.ts`

| Upstream test | Rust |
| --- | --- |
| JWT access token paths | Partial |
| Logged-out user introspect (3) | No |
| Prefix tests | Partial |

### `userinfo.test.ts` (9)

| Upstream test | Rust |
| --- | --- |
| `auth.api` headers only | No |
| JWT userinfo path | Partial |
| Scoped sub/profile/email | Yes (verificar given/family) |

### `register.test.ts` (19)

| Upstream test | Rust |
| --- | --- |
| Unauthenticated DCR overrides (5) | Partial |
| Full PKCE after override | No |
| Metadata strip unknown fields | No |

### `oauthClient/*.test.ts` (26 total)

| Upstream test | Rust |
| --- | --- |
| Privileges 16 cases | 1 aggregated test |
| cannot become public / secret not updatable | No |

### `types/zod.test.ts` (14)

| Upstream test | Rust |
| --- | --- |
| Safe URL matrix | 2 tests (subset) |

### `mcp.test.ts` (4)

| Upstream test | Rust |
| --- | --- |
| Full SDK flow | No |
| Challenge | Yes |

## Upstream tests without Rust equivalent (suggested priority)

| Priority | Scenario | Upstream file |
| --- | --- | --- |
| **High** | UserInfo `given_name`/`family_name` with profile scope | **Closed** Jun 2026 |
| **Medium** | `prompt=none` → `account_selection_required` / `interaction_required` | **Closed** Jun 2026 |
| Media | JWT introspection con JWKS remoto | `introspect.test.ts` |
| Media | Metadata con `disableJwtPlugin` asserts completos | `metadata.test.ts` |
| Media | Refresh scope narrow + `auth_time` | `token.test.ts` |
| Baja | Issuer en redirect vs metadata | `authorize.test.ts` |
| Baja | `oauth.test.ts` hooks cookie post-login | `oauth.test.ts` |
| Baja | Userinfo server-only headers API | `userinfo.test.ts` |
| Baja | MCP SDK E2E | `mcp.test.ts` |
| Baja | Unit query-serialization / timestamps | `utils/*.test.ts` |
| Baja | Rate limit 429 | `oauth.test.ts` |

## Verification command

```bash
cargo nextest run -p openauth-oauth-provider
```

## Maintenance

When adding Rust tests, update:

1. This matrix (matching section).
2. [`crates/openauth-oauth-provider/tests/upstream_mapping.md`](../../../crates/openauth-oauth-provider/tests/upstream_mapping.md) (summary table).
3. If the global count changes, this directory README.
