# Tests: inventario completo `openauth-axum`

Basado en `cargo nextest list -p openauth-axum` y lectura de cada archivo bajo `tests/` y `src/router.rs` (2026-06-01).

## Conteos

| Fuente | Métrica | Cantidad |
| --- | --- | --- |
| OpenAuth | Tests totales | **72** |
| OpenAuth | `#[test]` en crate lib (`router.rs`) | **10** |
| OpenAuth | Integration tests (`tests/*.rs`) | **62** |
| Upstream | `integrations/next-js.test.ts` | **5** `it(` |
| Upstream | `integrations/*.test.ts` (otros) | **0** |
| Upstream | `utils/url.test.ts` (`getBaseURL`, proxy, dynamic URL) | **54** `it(` |
| Upstream | `api/routes/sign-in.test.ts` (urlencoded + cross-site) | **≥4** escenarios HTTP relevantes |
| Upstream | `api/routes/sign-up.test.ts` (idem) | **≥4** escenarios HTTP relevantes |
| Upstream | `better-call/node` (npm, fuera del clone) | **~23** |

## Clasificación de los 72 tests OpenAuth

| Clase | # (aprox.) | Descripción |
| --- | --- | --- |
| **A — Adaptador puro** | **~28** | Montaje, contrato HTTP, body limit, errores, `base_path`/`base_url`, inferencia, `adapter_regression`, TCP |
| **B — Adaptador + core (IP / rate)** | **7** | `security.rs` + TCP `ConnectInfo` |
| **C — Core vía montaje Axum** | **8** | `security_upstream.rs`; fetch metadata, urlencoded, callbacks |
| **D — E2E auth montado** | **~29** | Flujos sesión, OAuth, cuentas, etc. (regresión del puente) |

La línea entre A y D es: si falla sin Axum pero pasa con `handler_async` directo en core, el test D es redundante; en la práctica varios D **no** duplican un test core idéntico con Tower.

## Inventario exhaustivo (nombre → archivo)

### `src/router.rs` (unit, 10)

| Test | Enfoque |
| --- | --- |
| `normalize_base_path_trims_trailing_slashes_except_root` | `""`, `/`, `/api/auth/` |
| `normalize_base_path_rejects_axum_pattern_syntax_and_non_absolute_paths` | `{`, `*`, `?`, `#`, path relativo |
| `infer_base_url_*` (4) | Proxy malicioso, forwarded válido, URI absoluta, loopback → `http` |
| `validate_base_url_*` (3) | Coincidencia pathname, mismatch, URL inválida |
| `axum_state_clones_only_the_shared_auth_pointer` | `Arc` en state Axum |

### `tests/adapter_regression.rs` (11)

| Test | Enfoque |
| --- | --- |
| `routes_accepts_stripped_paths_on_unmounted_router` | `routes_with_options` sin nest |
| `disabled_paths_are_enforced_through_axum_router` | `disabled_path` |
| `on_request_plugin_can_short_circuit_before_core_handler` | `PluginRequestAction::Respond` |
| `on_request_plugin_runs_before_core_handler` | Mutación request |
| `inbound_request_extensions_reach_core_handler` | Extensions Axum → core |
| `handle_with_options_preserves_response_contract` | `handle_with_options(&auth, …)` |
| `pre_set_request_client_ip_is_not_overwritten_by_connect_info` | IP manual |
| `base_url_inference_skips_when_oauth_override_is_present` | Skip infer |
| `social_sign_in_infers_from_absolute_request_uri` | URI absoluta |
| `tcp_listener_connect_info_enables_rate_limit_without_manual_injection` | `axum::serve` + TCP |

### `tests/routing.rs` (14)

| Test | Enfoque |
| --- | --- |
| `ok_route_is_mounted_under_default_base_path` | GET `/api/auth/ok` |
| `default_base_path_accepts_trailing_slash_root` | `/api/auth` y `/api/auth/` → **404** (Axum no mapea raíz del nest) |
| `skip_trailing_slashes_reaches_core_routes_over_axum` | `AdvancedOptions::skip_trailing_slashes` + `/ok/` |
| `custom_base_path_mounts_all_auth_routes` | `base_path("/auth")` |
| `root_base_path_mounts_auth_routes_at_root` | `base_path("/")` |
| `empty_base_path_mounts_auth_routes_at_root` | `base_path("")` |
| `trailing_slash_base_path_is_mounted_without_panicking` | `base_path("/api/auth/")` |
| `invalid_base_paths_are_rejected_before_mounting` | `OpenAuthAxumError::InvalidBasePath` |
| `inconsistent_base_url_path_is_rejected_before_mounting` | `InconsistentBaseUrlPath` |
| `non_auth_paths_and_wrong_methods_return_not_found` | Path typo; POST/HEAD/OPTIONS en `/ok` → 404 |
| `into_routes_can_be_nested_manually` | `Router::nest` + `into_routes()` |
| `extra_async_endpoint_is_reachable_through_catch_all` | Endpoint custom async |
| `plugin_endpoint_is_reachable_through_catch_all` | `AuthPlugin` + ruta |
| `every_core_auth_route_is_mounted_through_axum` | Itera `endpoint_registry()`; assert ≠ 404 |

### `tests/http_contract.rs` (4)

| Test | Enfoque |
| --- | --- |
| `borrowed_handle_preserves_response_status_version_headers_body_and_query` | `handle_ref`; HTTP/2, 201, query string |
| `axum_adapter_preserves_duplicate_response_headers` | 2× `Set-Cookie`, 2× header custom |
| `axum_adapter_preserves_response_extensions` | Extension en respuesta |
| `axum_adapter_preserves_empty_response_bodies` | 204 + body vacío |

### `tests/body_limit.rs` (2)

| Test | Enfoque |
| --- | --- |
| `configurable_body_limit_rejects_oversized_requests` | `body_limit(8)` → 413 JSON |
| `configurable_body_limit_allows_requests_within_limit` | `body_limit(1024)` acepta sign-in |

### `tests/error_contract.rs` (3)

| Test | Enfoque |
| --- | --- |
| `invalid_json_body_returns_stable_json_error` | JSON malformado → 400 `INVALID_REQUEST_BODY` (core parse, vía Axum) |
| `unsupported_content_type_returns_415_json_error` | `text/plain` en sign-in |
| `internal_endpoint_errors_are_sanitized` | Endpoint que falla → 500 sin leak del mensaje interno |

### `tests/security.rs` (6)

| Test | Enfoque |
| --- | --- |
| `csrf_origin_checks_are_preserved_over_axum` | `INVALID_ORIGIN` con Origin malo |
| `core_rate_limit_runs_without_axum_middleware` | Rate limit en `/ok` sin middleware Tower extra |
| `axum_rate_limit_uses_connect_info_without_ip_headers` | Misma IP socket → 429 |
| `axum_rate_limit_ignores_spoofed_forwarded_for_by_default` | XFF distinto no cambia bucket si solo ConnectInfo |
| `axum_rate_limit_uses_forwarded_for_when_proxy_headers_are_configured` | `advanced.ip_address.headers` + XFF |
| `axum_connect_info_ip_can_be_disabled` | `use_connect_info_for_ip(false)` + production → fail-closed 429 |

### `tests/security_upstream.rs` (8)

| Test | Enfoque | Referencia upstream |
| --- | --- | --- |
| `fetch_metadata_blocks_cross_site_navigation_without_cookies` | `CROSS_SITE_NAVIGATION_LOGIN_BLOCKED` | `sign-in.test.ts` / `origin-check` middleware |
| `fetch_metadata_allows_same_origin_navigation` | navigate + same-origin | idem |
| `fetch_metadata_allows_same_origin_cors_requests` | cors + sign-up | idem |
| `fetch_metadata_with_cookies_uses_origin_validation` | cross-site + cookie → no bloqueo por fetch metadata solo | idem |
| `form_urlencoded_sign_up_and_sign_in_work_over_axum` | `application/x-www-form-urlencoded` | `sign-up.test.ts` / `sign-in.test.ts` "should accept form-urlencoded" |
| `form_urlencoded_cross_site_navigation_is_blocked` | urlencoded + evil origin | sign-up cross-site cases |
| `form_urlencoded_same_site_navigation_from_trusted_origin_is_allowed` | same-site + document | idem |
| `callback_and_redirect_urls_are_validated_from_body_and_query` | `INVALID_CALLBACK_URL` body + query | core callback validation |

Estos 8 **no** están en `integrations/`; validan que headers/body llegan intactos al core a través del buffer Axum.

### `tests/password.rs` (3)

| Test | Enfoque |
| --- | --- |
| `password_reset_flow_works_over_axum` | request-reset, reset, GET callback, sign-in |
| `password_reset_url_uses_inferred_base_url` | `infer_base_url_from_request` + `Host: app.example.com` |
| `password_reset_url_does_not_infer_host_by_default` | `base_url` fijo ignora `Host` malicioso |

### `tests/social.rs` (4)

| Test | Enfoque |
| --- | --- |
| `social_sign_in_oauth2_and_callback_routes_work_over_axum` | `/sign-in/social`, `/sign-in/oauth2`, GET/POST callback |
| `social_sign_in_infers_base_url_from_host_when_unconfigured` | `redirect_uri` con https + host |
| `social_sign_in_rejects_host_origin_callback_by_default` | callback evil + `Host` evil → 403 |
| `social_sign_in_uses_trusted_proxy_headers_only_when_enabled` | XFH/XFP con y sin `trust_proxy_headers_for_base_url` |

### `tests/email_verification.rs` (2)

| Test | Enfoque |
| --- | --- |
| `email_verification_routes_work_over_axum` | send + verify GET |
| `email_verification_url_uses_inferred_base_url` | URL en email con inferencia |

### `tests/email_password.rs` (1)

| Test | Enfoque |
| --- | --- |
| `email_password_session_lifecycle_works_over_axum` | sign-up, get-session, sign-out, sign-in |

### `tests/accounts.rs` (1)

| Test | Enfoque |
| --- | --- |
| `account_list_unlink_and_token_routes_work_over_axum` | list-accounts, get-access-token, account-info, refresh-token, unlink |

### `tests/session_fields.rs` (1)

| Test | Enfoque |
| --- | --- |
| `update_session_additional_fields_work_over_axum` | `update-session` + campo `theme` |

### `tests/user_session_lifecycle.rs` (2)

| Test | Enfoque |
| --- | --- |
| `session_and_user_management_routes_work_over_axum` | list-sessions, update-user, revoke-session, change-password |
| `delete_user_route_works_over_axum` | delete-user habilitado |

### `tests/storage_smoke.rs` (1)

| Test | Enfoque |
| --- | --- |
| `memory_adapter_smoke_flow_runs_through_axum` | sign-up persiste user/session en memoria |

## Helpers de test (`tests/common/mod.rs`)

No son tests, pero fijan el contrato de integración:

| Helper | Rol |
| --- | --- |
| `request` / `json_request` | Construye `Request<Body>` con cookie opcional |
| `RequestHeaderExt::with_header` | Añade header estático |
| `body_json` / `body_text` | Lee respuesta (límite 10 MiB) |
| `cookie_header` | Concatena `Set-Cookie` para siguiente request |
| `FakeProvider` | OAuth social falso para `social.rs` / registry |
| `response_contract_endpoint` / `empty_response_endpoint` / `failing_endpoint` | Endpoints de prueba HTTP |

## Matriz upstream ↔ OpenAuth (tests HTTP relacionados con adaptador)

| Tema | Upstream (dónde) | openauth-axum |
| --- | --- | --- |
| Montaje catch-all | Implícito en router BA | `routing.rs` (13) |
| `getBaseURL` / proxy | `url.test.ts` (54) | **Sin réplica unitaria**; parcial en `password`/`social` (4) |
| form-urlencoded | `sign-in.test.ts`, `sign-up.test.ts` | `security_upstream.rs` (3 tests) |
| Cross-site navigation | `sign-in.test.ts`, `sign-up.test.ts` | `security_upstream.rs` (5 tests) |
| Next cookies / RSC | `next-js.test.ts` (5) | **N/A** |
| Node/Express mount | `better-call` (~23) | **N/A** (sin crate Node) |
| Handler genérico | Miles en `better-auth` API tests | E2E en este crate (20) + `openauth-core` |

## Huecos de test confirmados (post-auditoría)

| Hueco | Severidad | Evidencia |
| --- | --- | --- |
| Sin tests `url.test.ts`-style para proxy malicioso | **Media** | Ver [06-gaps-and-hardening.md](./06-gaps-and-hardening.md) |
| `handle` / `handle_with_options` sin uso en tests | Baja | Solo `handle_ref` |
| `routes_with_options` sin test directo | Baja | Cubierto indirectamente por `router_with_options` |
| Request extensions entrantes no assertadas | Baja | Solo response extensions en `http_contract` |
| Servidor TCP real + `connect_info` end-to-end | Baja | Tests inyectan `ConnectInfo` manualmente |
| `uri_origin` con request URI absoluta | Baja | No hay test |
| Dynamic base URL config | N/A diseño | No existe en Rust |
| `disabled_paths` montado en Axum | Media | Cubierto en `openauth-core` tests, no en axum |
| Plugin `on_request` vía Axum | Media | Solo plugin **endpoint** en `routing.rs` |
| `handle` / `handle_with_options` | Baja | Sin llamadas en tests |
| Path relativo `/ok` con `router()` default | Baja | Ver [07-path-mounting-and-footguns.md](./07-path-mounting-and-footguns.md) |
| Casos maliciosos `url.test.ts` | Media | 54 tests upstream sin portar |

## Comandos

```bash
cargo nextest list -p openauth-axum
cargo nextest run -p openauth-axum

# Solo adaptador (aprox.)
cargo nextest run -p openauth-axum -E 'test(/routing|http_contract|body_limit|error_contract|normalize_base_path|axum_state/)'

# Seguridad montada
cargo nextest run -p openauth-axum -E 'test(/security/)'
```

## Tests del core que cubren lo mismo sin Axum

Antes de añadir tests al adaptador, revisar duplicados en:

| Crate | Archivos útiles |
| --- | --- |
| `openauth-core` | `tests/api/router.rs`, `tests/api/body.rs`, `tests/utils/trusted_origins.rs` |
| `openauth` | `tests/public_api.rs` (`handler_async` directo) |

`openauth-axum` sigue siendo necesario para regresiones de **montaje**, **buffer body**, **ConnectInfo** y **inferencia base URL** en la capa Axum.
