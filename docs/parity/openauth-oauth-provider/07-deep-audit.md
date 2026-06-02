# 07 — Deep audit (code + tests)

> **Update:** Server gaps listed in “Second pass” and “Suggested actions” were **closed in code on 2026-06-02**. See [08-parity-closeout-2026-06.md](./08-parity-closeout-2026-06.md) for current status and out-of-scope items.

Audit **2026-06-01** from sources:

- `reference/upstream-src/1.6.9/repository/packages/oauth-provider/src/` (27 production `.ts`, 18 `.test.ts`)
- `crates/openauth-oauth-provider/src/` (24 `.rs`) and `tests/oauth_provider/` (**96** tests after closeout)

READMEs were not used as primary sources.

## Verified counts

| Metric | Upstream | OpenAuth |
| --- | --- | --- |
| Production files | 27 | 24 |
| Test files | 18 | 6 modules + harness |
| `it(` | **261** (per-file sum below) | **96** (`#[test]` 9 + `#[tokio::test]` 87) |
| Distinct HTTP endpoints | 25 | 26 (+ `GET /oauth2/continue`) |

### `it(` per upstream file

| Archivo | `it` |
| --- | --- |
| `token.test.ts` | 38 |
| `oauth.test.ts` | 37 |
| `register.test.ts` | 19 |
| `pairwise.test.ts` | 18 |
| `authorize.test.ts` | 18 |
| `oauthClient/endpoints-privileges.test.ts` | 16 |
| `metadata.test.ts` | 15 |
| `types/zod.test.ts` | 14 |
| `introspect.test.ts` | 14 |
| `revoke.test.ts` | 11 |
| `pkce-optional.test.ts` | 11 |
| `oauthClient/endpoints.test.ts` | 10 |
| `userinfo.test.ts` | 9 |
| `utils/query-serialization.test.ts` | 8 |
| `logout.test.ts` | 7 |
| `utils/timestamps.test.ts` | 6 |
| `oauthConsent/endpoints.test.ts` | 6 |
| `mcp.test.ts` | 4 |

## Public exports upstream vs Rust

| Upstream (`index.ts` + subpaths) | OpenAuth | Status |
| --- | --- | --- |
| `oauthProvider` | `oauth_provider()` | Parity |
| `getOAuthProviderState` | — | **Gap:** no request-scoped state; pending en `verification` |
| `authServerMetadata` | `auth_server_metadata()` | Parity |
| `oidcServerMetadata` | `oidc_server_metadata()` | Parity |
| `oauthProviderAuthServerMetadata` | Solo vía endpoint plugin | **Partial:** no standalone exported handler |
| `oauthProviderOpenIdConfigMetadata` | Idem | **Parcial** |
| `mcpHandler` | `mcp::*` no HTTP middleware | **Parcial** |
| `export type *` | Tipos en `options`, `client`, `models` | Parity conceptual |
| `@better-auth/oauth-provider/client` | — | **N/A** browser |
| `@better-auth/oauth-provider/resource-client` | `mcp::protected_resource_metadata`, `validate_bearer_token` | **Parcial** (no SDK, no `externalScopes` / `remoteVerify` wrapper) |

## `OAuthOptions` without Rust equivalent

| Opción upstream | OpenAuth | Notes |
| --- | --- | --- |
| `schema` | — | No custom schema merge in plugin |
| `silenceWarnings` | — | No well-known warnings on init |
| `signup.shouldRedirect` | Solo `signup_page` + `signup_redirect` al ver `prompt=create` | **Gap:** does not evaluate “¿falta registro?” before authorize |
| `selectAccount.shouldRedirect` | Solo si `prompt=select_account` | **Gap:** upstream evalúa con sesión even when prompt does not ask |
| `postLogin.shouldRedirect` | `post_login_page` / `post_login_redirect` si página configurada | **Gap:** upstream uses boolean callback |
| `postLogin.consentReferenceId` | `client_reference` when **creating** client | **Gap:** consent reference in authorize may differ |
| `scopeExpirations` (`number \| string \| Date`) | `BTreeMap<String, u64>` | Seconds only |
| `clientRegistrationClientSecretExpiration` (duración texto) | `Option<u64>` segundos | No parser `"5 minutes"` |
| `storeClientSecret` objeto `{encrypt, decrypt}` | `Encrypted` = `symmetric_*` del `context.secret` | **Gap:** no custom encrypt/decrypt callbacks |
| `storeClientSecret` objeto `{hash, verify}` | `hash_client_secret` / `verify_client_secret_hash` | Parity |
| `advertisedMetadata` | `advertised_scopes_supported` + `advertised_claims_supported` | Parity |

## Authorize behavior: gaps confirmed in code

Reading `authorize.ts` (upstream) vs `endpoints/authorization.rs` + `authorize.rs` (Rust).

| Behavior | Upstream | Rust | Severity |
| --- | --- | --- | --- |
| `prompt=none` + login → `login_required` | Sí | Sí | — |
| `prompt=none` + consent → `consent_required` | Sí | Sí (`decide_authorize`) | — |
| `prompt=none` + `selectAccount.shouldRedirect()` true → `account_selection_required` | Sí | **No** (no existe `shouldRedirect`) | **Medium** |
| `prompt=none` + `signup.shouldRedirect()` true → `interaction_required` | Sí | **No** | **Medium** |
| `prompt=none` + `postLogin.shouldRedirect()` true → `interaction_required` | Sí | **No** (only redirect si `post_login_page` fijada) | **Medium** |
| `iss` en redirect éxito/error (RFC 9207) | Sí + tests | Código añade `iss` en `issue_authorization_code_redirect` / `authorization_error_redirect` | Tests **weak** |
| Issuer metadata = redirect `iss` | Test dedicado | **No test** | Low |
| PAR descarta params front-channel | Test | Parcial | Low |
| `max_age=0` | Sí | Sí | — |

## UserInfo: `profile` claims gap

- Upstream: `userNormalClaims()` en `userinfo.ts` includes `given_name` / `family_name` (split from `name`).
- Rust: `user_normal_claims()` en `token/claims.rs` **does** compute them para ID token.
- Rust: `endpoints/userinfo.rs` **does not** use `user_normal_claims`; only `name`, `picture`, `email` → **missing `given_name`/`family_name` on `/oauth2/userinfo`**.
- Test upstream: `profile only` con given/family; Rust: `userinfo_returns_claims_by_explicit_*` — revisar si assert includes given/family (probable gap).

## Token / introspect / revoke

| Tema | Upstream | Rust |
| --- | --- | --- |
| JWT introspect/revoke | Dedicated tests with JWT plugin | `introspection.rs` verifica JWT si `!disable_jwt_plugin`; tests mostly with JWT off |
| Introspect con usuario deslogueado | 3 tests | **No test** |
| `auth_time` tras refresh en ID token | Test OIDC 12.2 | **No test** explicit |
| Refresh scopes más amplios | Test rechazo | Logic in `token/mod.rs` L401-407; **sin test** narrow/expand |
| Refresh quitando `offline_access` | Test upstream | **No test** |
| Auth code sin `state` | Test | **No test** |
| Encrypted secret mismatch / custom decrypt error | 2 tests | Encrypted built-in; **sin tests** de error decrypt custom |
| Pre-parsed form body en token | `oauth.test.ts` | **No test** |
| `application/json` en token | — | Rust **acepta** JSON + form (`endpoints/token.rs`) — **superset** |

## PKCE (`pkce-optional.test.ts`)

| Test upstream | Rust |
| --- | --- |
| Public sin PKCE falla | Sí |
| Confidential default PKCE | Sí |
| Confidential opt-out | Sí |
| offline_access sin PKCE falla | Cubierto en policy test |
| PKCE only en auth / only en token | **No tests** separados |
| Challenge mismatch | Sí (`rejects_spurious_pkce_verifier`) |
| Admin create persiste `require_pkce` | **No test** admin path |

## Client / DCR

| Test upstream | Rust |
| --- | --- |
| zod safe URL (14 tests) | 2 tests redirect (subset) |
| Unauthenticated DCR overrides (5 tests) | Parcial en metadata rejection |
| PKCE flow tras DCR override | **No test** |
| `client cannot become public` via update | **No test** |
| `client_secret` no actualizable | **No test** |
| Privileges (16 tests) | 1 test `client_privileges_can_deny_*` |

## Pairwise (`pairwise.test.ts`)

| Test upstream | Rust |
| --- | --- |
| Cross-RP unlinkability (2 clients) | Parcial (sector test) |
| Same client determinism | Sí |
| Public client → `user.id` | **No test** |
| JWT access `sub` = user.id real | **No test** |
| `subject_types_supported` metadata | **No test** |
| Round-trip DCR `subject_type` | Parcial |

## Metadata / init (`metadata.test.ts`, `oauth.test.ts`)

| Escenario | Rust |
| --- | --- |
| Remote JWKS URL | **No** |
| `disableJwtPlugin` metadata completo | Code yes; minimal tests |
| Dynamic base URL metadata wrappers | **No** |
| Protected resource + `externalScopes` | **No** (`externalScopes` no existe) |
| Init: JWT plugin required | Parcial (merge en `oauth_provider`) |
| Secondary storage + DB session | **No** (core) |
| Rate limit **enforcement** (429) | Rules yes; **no test** that hits limit |
| Generic OAuth / sign-in integration | **No** |
| JSON redirect tras login fetch | **No** (N/A browser) |
| Client deleted/disabled mid-flow JSON error | **No** |

## MCP (`mcp.test.ts`)

| Escenario | Rust |
| --- | --- |
| Challenge header | Sí |
| SDK E2E DCR + resource | **No** |
| `resourceMetadataMappings` URN | Parcial en `www_authenticate_for_resources` |

## Global hooks (`oauth.ts`)

| Hook upstream | Rust |
| --- | --- |
| `before`: `oauth_query` en body → state + forward sign-in | **No** in crate |
| `after`: cookie → re-entrada `/oauth2/authorize` | **No**; `/oauth2/continue` manual |
| `publicSessionMiddleware` | Inline logic en `public-client-prelogin` | Parity funcional |

## Upstream utilities (`utils/index.ts`) not exported in Rust

| Upstream function | Rust |
| --- | --- |
| `searchParamsToQuery` / `deleteFromPrompt` | No module; prompt logic in `authorize.rs` |
| `normalizeTimestampValue` / `resolveSessionAuthTime` | Timestamps vía `time` crate; **no unit tests** |
| `verifyOAuthQueryParams` | `verify_oauth_query` en `clients.rs` |
| `getOAuthProviderPlugin` / `getJwtPlugin` | N/A |

## DB schema: minor differences

| Campo | Upstream | Rust |
| --- | --- | --- |
| `client_secret_expires_at` | No en schema.ts base | **Sí** en `oauth_clients` |
| `id` PK interno | Implícito BA | `id` explicit en schema contribution |
| Nombres físicos | camelCase modelo | `oauth_clients` plural snake_case |

## Revised parity summary

| Area | Verdict after audit |
| --- | --- |
| Endpoints protocolo + CRUD | **High** |
| Token grants core | **High** |
| PKCE policy | **High** (tests PKCE auth/token parciales) |
| Prompts OIDC avanzados | **Medium** (`account_selection_required`, `interaction_required`, `shouldRedirect`) |
| UserInfo profile claims | **Medium-low** (given_name/family_name) |
| Hooks login OAuth | **Low / N/A** |
| Cliente TS / resource-client SDK | **N/A** |
| Test coverage vs 261 `it` | **~40–50%** escenarios con test Rust explicit; many covered indirectly |

## Second pass (code/tests only, Jun 2026) — **closed** in [08](./08-parity-closeout-2026-06.md)

Findings **new** vs the first audit the same day.

### Metadata / discovery

| Finding | Upstream | Rust | Severity |
| --- | --- | --- | --- |
| `GET /.well-known/oauth-authorization-server` with `openid` in scopes | Returns full OIDC metadata | Was RFC 8414-only | **Closed** |
| `id_token_signing_alg_values_supported` | From JWT plugin | Was hardcoded EdDSA | **Closed** (`advertised_*`, `oauth_provider_with_jwt`) |
| Remote `jwks_uri` | JWT plugin `remoteUrl` | Was local `/jwks` only | **Closed** (`advertised_jwks_uri`) |

### HTTP responses

| Finding | Upstream | Rust |
| --- | --- | --- |
| Token response headers | `Cache-Control: no-store`, `Pragma: no-cache` | Was missing | **Closed** |
| DCR `POST /oauth2/register` | `201` + `Cache-Control: no-store` | Was missing | **Closed** |
| Redirect SPA / fetch | `{ redirect, url }` for fetch / JSON Accept | Was 302 only | **Closed** (`redirect_or_json_response`) |
| Revoke body | Empty body | Was JSON `null` | **Closed** (`empty_success_response`) |

### Authorize / consent (different model)

| Finding | Upstream | Rust |
| --- | --- | --- |
| Estado entre login y consent | `oAuthState` + query serializada (`oauth_query` en body consent) | `request_id` + `PendingAuthorizationValue` en `verification` |
| Consent `referenceId` | `postLogin.consentReferenceId()` at consent time | `reference_id` from client / pending authorization |
| Query OIDC opcionales en authorize | `display`, `ui_locales`, `acr_values`, `login_hint`, `id_token_hint` en `types/zod.ts` (passthrough) | Only `max_age` implemented; rest ignored |
| Verification value | JSON con `query` completa (`verificationValueSchema`) | `AuthorizationCodeValue` estructurado without storing full query |
| Código inválido en token | (revisar mensaje upstream) | `invalid_verification` / `"Invalid code"` — may differ from `invalid_grant` |

### UserInfo (reconfirmed)

- **Was:** `userinfo.rs` omitted `given_name`/`family_name`; **closed** via `user_normal_claims`.

### Tests that would fail if ported verbatim

| Test upstream | Why there is no Rust equivalent |
| --- | --- |
| `metadata.test.ts` — `getOAuthServerConfig` === `getOpenIdConfig` | Different behavior en ruta oauth-authorization-server |
| `oauth.test.ts` — JSON redirect tras sign-in | No `handleRedirect` |
| `token.test.ts` — headers no-store en token | No headers |
| `register.test.ts` — cache no-store en DCR | No header |

### Reviewed and confirmed **no** new gap

- Endpoints HTTP: same 25 (+ `GET /continue` en Rust).
- `prompt` compuesto `login consent` / `select_account consent`: Rust `prompt_contains` usa `split_whitespace` — reasonably aligned behavior.
- PKCE `plain` rejected — test `authorize` con `code_challenge_method=plain`.
- `resource` en token (string, array, repeated) — tests dedicados en Rust.
- Revoke JWT no-op, replay refresh — implementado.
- `contacts` en DCR sin `min(1)` estricto como upstream zod admin — more lax validation, not stricter.

## Suggested actions (product — **done** in [08](./08-parity-closeout-2026-06.md))

1. Add `given_name`/`family_name` en `userinfo.rs` reutilizando `user_normal_claims`.
2. Align `/.well-known/oauth-authorization-server` con upstream cuando `openid` ∈ scopes (devolver OIDC metadata o documentar divergencia).
3. Add `Cache-Control` / `Pragma` en token y DCR; consider respuestas JSON `{ redirect, url }` para clientes fetch.
4. Implement errors `account_selection_required` / `interaction_required` or document as unsupported.
5. Tests: JWT introspect, metadata oauth===oidc, refresh scope narrowing, `auth_time` tras refresh, rate limit 429, `iss` vs metadata.
6. Optional export de handlers metadata standalone si integradores lo necesitan.
7. Propagate algorithm de firma ID token desde `openauth-plugins::jwt` instead of hardcoding `EdDSA`.
