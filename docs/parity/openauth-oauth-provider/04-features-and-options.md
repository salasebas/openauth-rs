# 04 — Features and options

Comparison of **observable server** capabilities, not TypeScript ergonomics.

## Configuration defaults

| Option | Upstream default | OpenAuth default | Parity |
| --- | --- | --- | --- |
| `scopes` | `openid`, `profile`, `email`, `offline_access` | Same (if list empty) | Yes |
| `codeExpiresIn` | 600 | `code_expires_in: 600` | Yes |
| `accessTokenExpiresIn` | 3600 | 3600 | Yes |
| `m2mAccessTokenExpiresIn` | 3600 | 3600 | Yes |
| `idTokenExpiresIn` | from JWT plugin | 36000 explicit in options | Similar |
| `refreshTokenExpiresIn` | 2_592_000 | 2_592_000 | Yes |
| `grantTypes` | code, client_credentials, refresh | Same | Yes |
| `allowDynamicClientRegistration` | false | false | Yes |
| `allowUnauthenticatedClientRegistration` | false | false | Yes |
| `disableJwtPlugin` | false | false | Yes |
| `storeClientSecret` | hashed (JWT on) / encrypted (JWT off) | `Auto` → same rule | Yes |
| `storeTokens` | hashed | `Hashed` (encrypted **rejected**) | See [05](./05-design-decisions.md) |

## Grants and flows

| Grant / flow | Upstream | OpenAuth | Rust tests |
| --- | --- | --- | --- |
| Authorization code + PKCE S256 | Yes | Yes | `authorization_code_flow_*`, PKCE tests |
| Refresh token + rotation | Yes | Yes | `refresh_token_grant_rotates_*` |
| Refresh replay → revoke family | Yes | Yes | `refresh_token_replay_revokes_refresh_token_family` |
| Client credentials | Yes | Yes | `client_credentials_*` |
| Reject OIDC scopes on M2M | Yes | Yes | `client_credentials_rejects_oidc_scopes` |
| `offline_access` → refresh | Yes | Yes | Main flow |
| Implicit / password / device | No | No | — |

## Token issuance

| Behavior | Upstream | OpenAuth |
| --- | --- | --- |
| Opaque access without `resource` | Yes | Yes |
| JWT access with `resource` / audiences | Yes | Yes | `resource_parameter_*` |
| ID token with `openid` | Yes (JWT plugin) | Yes | `openid_authorization_code_issues_signed_id_token_and_jwks` |
| ID token HS256 without JWT plugin | Yes | Yes | `disable_jwt_plugin: true` in many tests |
| Pairwise `sub` | Yes | Yes | `pairwise_*` |
| JWT access keeps real `sub` (not pairwise) | Yes | Assumed by upstream design | No dedicated test |
| Custom claims (access, id, userinfo, token response) | Yes | Yes | `custom_*` tests |
| Prefixes on returned tokens/secrets | Yes | Yes | `prefixes_and_custom_generators_*` |
| `formatRefreshToken` | Yes | Yes | `format_refresh_token_wraps_*` |
| Custom hash store | Yes | Yes | `custom_store_hash_callbacks_*` |
| Scope-specific expiration (shortest wins) | Yes | Yes | `scope_expirations_use_shortest_matching_scope` |

## Authorization / prompts

| Parameter / prompt | Upstream | OpenAuth | Notes |
| --- | --- | --- | --- |
| `response_type=code` | Yes | Yes | |
| `prompt=none` → `login_required` / `consent_required` | Yes | Yes | Dedicated tests |
| `prompt=none` → `account_selection_required` / `interaction_required` | Yes | Yes | `*_should_redirect` (Jun 2026) |
| `prompt=login` | Yes | via login redirect | |
| `prompt=consent` | Yes | Yes | |
| `prompt=create` | Yes (`signup` option) | `signup_page` + continue | Different API, same intent |
| `prompt=select_account` | Yes (`selectAccount`) | `select_account_page` + continue | |
| Post-login org selection | Yes (`postLogin`) | `post_login_page` + continue | |
| `max_age=0` | Yes | Yes | `authorize_max_age_zero_forces_login_redirect` |
| `request_uri` (PAR resolver) | Yes | Yes | No PAR endpoint |
| `iss` on redirect (RFC 9207) | Yes | Yes | `authorize_success_redirect_includes_iss_parameter` |
| Loopback redirect (RFC 8252) | Yes | Yes | `authorize_loopback_matching_*` |
| Ignore unknown prompts | Yes | Yes | `authorize_ignores_unknown_prompt_values` |
| `login_hint`, `display`, `ui_locales`, `acr_values` | In types / metadata | Metadata advertises ACR; **few authorize effects** | Low parity on optional OIDC params |

## Client registration and management

| Feature | Upstream | OpenAuth |
| --- | --- | --- |
| DCR RFC 7591 | Yes | Yes |
| DCR cannot set `skip_consent` / `enable_end_session` | Yes | Yes |
| Unauthenticated DCR → `auth_method=none` | Yes | Yes |
| Pairwise requires same redirect sector | Yes | Yes (host:port) |
| `client_reference` / org ownership | Yes | Yes |
| `client_privileges` hook | Yes | Yes |
| `cached_trusted_clients` | Yes | Yes |
| Rotate secret | Yes | Yes |
| Safe redirect URL (`javascript:`, etc.) | Zod | Validation + tests | `dynamic_registration_rejects_unsafe_redirect_urls` |
| `jwks` / `jwks_uri` in DCR | Accepted, not always persisted | Similar per checklist | |

## Introspection / revocation / userinfo

| Feature | Upstream | OpenAuth |
| --- | --- | --- |
| Introspect JWT / opaque / refresh | Yes | Yes (JWT if plugin active) |
| Revoke JWT → no-op | Yes | Yes |
| Revoke opaque / refresh + replay | Yes | Yes |
| `token_type_hint` | Yes | Yes |
| Userinfo scope-gated | Yes | Yes |
| Userinfo without `openid` → error | Yes | Yes |
| Pairwise on introspect/userinfo opaque | Yes | Yes |
| UserInfo `given_name` / `family_name` | Yes | Yes | `user_normal_claims` (Jun 2026) |

## Metadata / discovery

| Field / behavior | Upstream | OpenAuth |
| --- | --- | --- |
| OAuth AS metadata RFC 8414 | Yes | Yes on `/.well-known/oauth-authorization-server` |
| OIDC discovery | Yes | Yes on `/.well-known/openid-configuration` |
| **oauth-authorization-server with `openid`** | Full **OIDC** document | **Same** (Jun 2026) |
| `Cache-Control` with stale-* | Yes | Yes |
| `advertisedMetadata` scopes/claims | Yes | `advertised_scopes_supported`, `advertised_claims_supported` |
| No `jwks_uri` if JWT off | Yes | Yes |
| Remote `jwks_uri` (JWT plugin) | Yes | `advertised_jwks_uri` + `oauth_provider_with_jwt` |
| `id_token_signing_alg_values_supported` | From JWT plugin | `advertised_id_token_signing_algorithms` or defaults |
| Protected resource metadata (RFC 9728) | resource-client + tests | `mcp::protected_resource_metadata` |

## Token / register response headers

| Header | Upstream | OpenAuth |
| --- | --- | --- |
| Token: `Cache-Control: no-store`, `Pragma: no-cache` | Yes | Yes (Jun 2026) |
| DCR: `Cache-Control: no-store` on `201` | Yes | Yes (Jun 2026) |

## MCP and resource server

| Feature | Upstream | OpenAuth |
| --- | --- | --- |
| `mcpHandler` middleware | Yes | No middleware; functions in `mcp` |
| `WWW-Authenticate` + `resource_metadata` | Yes | `www_authenticate_for_resources` |
| `validate_bearer_token` | via introspect/JWT | `mcp::validate_bearer_token` |
| Remote introspection verify | In resource-client | App calls `/oauth2/introspect` |
| MCP SDK E2E test | Yes | No |

## Upstream-only options (documentation)

| Upstream option | OpenAuth |
| --- | --- |
| `silenceWarnings` (well-known path) | No |
| `schema` custom merge | via `openauth-core` schema contributions |
| `secondaryStorage` + DB session validation | Not in this crate |

## Hooks and state (summary)

| Mechanism | Upstream | OpenAuth |
| --- | --- | --- |
| `getOAuthProviderState()` / `oAuthState` | Request-scoped signed query | Pending auth in `verification` |
| Sign `oauth_query` | Utils + browser client | `verify_oauth_query` on prelogin |
| Post-login auto-resume authorize | Cookie hook | Manual `/oauth2/continue` |

## Upstream options without a Rust field (audit)

| Upstream `OAuthOptions` | OpenAuth |
| --- | --- |
| `schema` | — |
| `silenceWarnings` | — |
| `signup` / `selectAccount` / `postLogin` (with `shouldRedirect`) | `signup_page`, `select_account_page`, `post_login_page` + `*_should_redirect` |
| `postLogin.consentReferenceId` | `consent_reference_id` resolver |
| `storeClientSecret` `{ encrypt, decrypt }` custom | `SecretStorage::Encrypted` fixed |
| `scopeExpirations` with string/Date | `scope_expirations: BTreeMap<String, u64>` |
| `clientRegistrationClientSecretExpiration` string | `Option<u64>` |

See decisions: [05-design-decisions.md](./05-design-decisions.md).
