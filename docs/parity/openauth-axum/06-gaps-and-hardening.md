# Huecos, hardening y comparación `getBaseURL` ↔ inferencia Axum

Auditoría contra **código fuente** (no READMEs), Better Auth **1.6.9**.

## API pública real (`src/lib.rs`)

Todo lo exportado:

| Símbolo | Tipo | ¿Testeado directamente? |
| --- | --- | --- |
| `OpenAuthAxumError` | enum | Sí (`invalid_base_paths_are_rejected_before_mounting`) |
| `OpenAuthAxumOptions` | struct + builder | Sí (body, IP, infer, proxy en varios tests) |
| `router` / `router_with_options` | fn | Sí (mayoría de tests) |
| `routes` / `routes_with_options` | fn | Parcial (`into_routes` usa `into_routes`; no hay test de `routes_with_options` aislado) |
| `handle` / `handle_with_options` | fn | Sí (`handle_with_options_preserves_response_contract`; firma `&OpenAuth`) |
| `handle_ref` / `handle_ref_with_options` | fn | Sí (`http_contract` usa `handle_ref`) |
| `OpenAuthAxumExt` | trait | Sí (`into_router`, `into_routes`; no `into_router_with_options` por nombre) |

## Comportamiento no documentado en README del crate

| Comportamiento | Ubicación | Detalle |
| --- | --- | --- |
| Estado compartido `Arc<OpenAuth>` | `routes_from_shared` | Cada request usa `Arc` clonado; test `axum_state_clones_only_the_shared_auth_pointer` |
| Solo `handler_async` | `router.rs:126` | No se expone `OpenAuth::handler` síncrono por Axum |
| Errores adaptador cortocircuitan | `handle_ref_with_options` | `to_api_request` devuelve `Response` en `Err` sin llamar al core |
| `json_error_response` + `original_message` | `error.rs:40-44` | Parámetro existe; adaptador **nunca** pasa `Some(...)` al cliente |
| Inferencia inserta dos extensions | `router.rs:179-184` | `RequestBaseUrl` y `OAuthBaseUrlOverride` con el mismo string |
| Skip infer si override presente | `router.rs:168` | Si ya hay `OAuthBaseUrlOverride`, no inferir |
| Skip IP si ya hay `RequestClientIp` | `request.rs:28` | Permite inyectar IP manualmente antes del adaptador |
| Rate limit fail-closed sin IP | `security.rs` `axum_connect_info_ip_can_be_disabled` | Con `production(true)` + IP desactivada en adaptador → `429` en `/ok` |

## `getBaseURL` upstream vs `infer_base_url` OpenAuth

Upstream: `packages/better-auth/src/utils/url.ts` + **54** tests en `url.test.ts`.

OpenAuth: lógica en `crates/openauth-axum/src/router.rs` (`infer_base_url`, `forwarded_origin`, …), activada solo con `infer_base_url_from_request(true)`.

| Capacidad | Better Auth (`getBaseURL` / handler) | OpenAuth Axum |
| --- | --- | --- |
| Env `BETTER_AUTH_URL`, `NEXT_PUBLIC_*`, … | Sí (`loadEnv`) | **No** en adaptador (configurar en build/deploy) |
| `baseURL` estático en opciones | Sí | `OpenAuthOptions::base_url` (core) |
| Inferir desde URL absoluta del request | Sí (`getOrigin(request.url)`) | Sí (`uri_origin`) si el URI trae scheme+authority |
| Inferir desde `Host` (https salvo loopback) | Implícito vía URL / host helpers | Sí (`host_header_origin`, loopback → `http`) |
| `x-forwarded-host` + `x-forwarded-proto` | Con `trustedProxyHeaders` | Con **doble** opt-in adaptador |
| Validación estricta proxy (`validateProxyHeader`) | Sí (null, `..`, `javascript:`, regex host, …) | **Parcial** (`is_valid_host` / `is_valid_proto` más simples) |
| Fallback `window.location` | Sí | **N/A** (server) |
| Error si no hay base URL | `BetterAuthError` en handler | Core con `base_url` vacío; sin inferencia las URLs pueden ser incorrectas |
| `isDynamicBaseURLConfig` / `resolveRequestContext` | Sí (clone ctx por request) | **No portado** en Rust 1.6.9 |
| `resolveBaseURL` / host patterns | Sí (`url.test.ts`) | **No** en adaptador |

### Diferencias de validación de host/proxy (riesgo)

Upstream `validateProxyHeader` rechaza, entre otros:

- `javascript:` / `file:` / `data:` en proto
- Host con null bytes, espacios, punto inicial, caracteres HTML
- Patrones de inyección en host

OpenAuth `is_valid_host`:

- Rechaza vacío y `..`
- Permite solo bytes alfanuméricos ASCII y `. - _ : [ ]`
- **No** replica todas las reglas de `validateProxyHeader` (p. ej. host que contenga subcadena `javascript:` podría comportarse distinto si pasara el filtro de bytes)

**Estado:** paridad **parcial** en hardening de headers proxy; los tests de `url.test.ts` **no** tienen réplica en `openauth-axum` (solo escenarios happy-path en `social.rs` / `password.rs`).

## Funcionalidad upstream sin equivalente Axum

| Upstream | Archivo | Notas |
| --- | --- | --- |
| `toNextJsHandler` | `next-js.ts` | 24 líneas; delega a `handler` |
| `nextCookies` | `next-js.ts` | Plugin ~90 líneas |
| `fromNodeHeaders` | `node.ts` | Helper público para `auth.api.*` con headers Node |
| `toNodeHandler` | `node.ts` → `better-call` | Conversión `IncomingMessage` |
| `isAuthPath` | `svelte-kit.ts` | Filtro pathname para middleware SK |
| `svelteKitHandler` | `svelte-kit.ts` | Middleware con `resolve` chain |
| `sveltekitCookies` | `svelte-kit.ts` | Plugin cookies SK |
| `toSvelteKitHandler` | `svelte-kit.ts` | Thin handler |
| `toSolidStartHandler` | `solid-start.ts` | Igual patrón que Next |
| `tanstackStartCookies` | `tanstack-start.ts`, `tanstack-start-solid.ts` | Plugins cookies TanStack |

Ninguno es obligatorio para server-only Rust; todos son **N/A por diseño** salvo que se quiera un crate `openauth-node` futuro.

## Ejemplos en el repo (patrones reales)

| Ejemplo | Montaje | `ConnectInfo` |
| --- | --- | --- |
| `examples/full-app` | `auth.into_routes()` + `.nest(AUTH_BASE_PATH, …)` | Sí en `main` (`into_make_service_with_connect_info`) |
| `examples/stripe-smoke-server` | `Router::new().nest(AUTH_BASE_PATH, auth.into_routes())` | Listener Tokio (revisar si connect_info en smoke) |

`full-app` **no** usa `router()` de un solo paso: compone router estático + nest manual (patrón documentado en `into_routes`).

## Integración workspace

| Consumidor | Uso |
| --- | --- |
| `openauth-cli` | Detecta dependencia `openauth-axum` en metadata (`workspace.rs`) |
| `RELEASE.md` | Orden de publicación después de `openauth` |

## `base_path` desacoplado de `base_url.pathname`

Upstream fija el path del router HTTP con `new URL(ctx.baseURL).pathname`. OpenAuth mantiene
`base_path` y `base_url` como campos separados. **`router()` / `router_with_options()`**
rechazan montaje si el pathname de `base_url` no coincide con `base_path` (salvo `base_url`
vacío). **`into_routes()`** no valida: footgun documentado en
[07-path-mounting-and-footguns.md](./07-path-mounting-and-footguns.md).

## Cerrado en código (2026-06-01)

| Gap | Resolución |
| --- | --- |
| Validación proxy débil | `openauth_core::utils::url::{is_valid_forwarded_host,is_valid_forwarded_proto}` + tests `tests/utils/forwarded_headers.rs` |
| `is_loopback` duplicado | Axum usa `openauth::utils::host::is_loopback_host` |
| Tests `url.test.ts` (subset) | Unit tests en `router.rs` + core forwarded_headers |
| `handle_with_options`, extensions request, `disabled_paths`, plugin `on_request` | `tests/adapter_regression.rs` |
| Path `/ok` en router sin nest | `routes_accepts_stripped_paths_on_unmounted_router` |
| URI absoluta + OAuth override skip + `RequestClientIp` pre-set | Tests en `adapter_regression.rs` |
| Footgun body middleware | Nota en `crates/openauth-axum/README.md` |
| Validación `base_url` pathname vs `base_path` | `validate_base_url_matches_base_path` en `router_with_options` + tests unit/integration |
| Plugin `on_request` short-circuit | `on_request_plugin_can_short_circuit_before_core_handler` |
| E2E TCP + `ConnectInfo` | `tcp_listener_connect_info_enables_rate_limit_without_manual_injection` |
| `handle` / `handle_with_options` por referencia | Firma `&OpenAuth`; `examples/full-app` actualizado |

## Pendiente (diseño / fuera de alcance)

1. `DynamicBaseURLConfig` / `resolveRequestContext` por request en Rust.
2. Derivar `base_path` automáticamente del pathname de `base_url` (upstream lo hace en el handler).
3. Validación en `into_routes()` sin romper API (`Router` sin `Result`).
4. Crate `openauth-node` / `fromNodeHeaders`.
5. Réplica amplia de los **54** `it` de `url.test.ts` (subset cubierto en core + Axum).
