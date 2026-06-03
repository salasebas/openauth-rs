# Inventario de funcionalidades del adaptador

Leyenda de estado:

| Estado | Significado |
| --- | --- |
| **Igual** | Mismo comportamiento observable en el rol del adaptador |
| **Superset** | OpenAuth hace más o es más explícito |
| **Parcial** | Cubierto con diferencias |
| **N/A** | No aplica (TS-only, otro framework, o server-only) |
| **Hueco** | Upstream tiene algo que el adaptador Axum no expone |

## Montaje y routing

| Funcionalidad | Upstream (referencia) | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| Catch-all bajo `basePath` | Router `better-call` con `basePath` | `routes`: `any("/")` + `any("/{*path}")` | **Igual** | Core resuelve path relativo a `base_path` |
| Path HTTP derivado solo de `baseURL` | `new URL(ctx.baseURL).pathname` | `OpenAuthOptions::base_path` separado | **Divergente** | Ver [07-path-mounting-and-footguns.md](./07-path-mounting-and-footguns.md) |
| Métodos en docs Hono/Next | GET+POST en ejemplos docs | `any()` todos los métodos | **Superset** | Implementación Next tiene 5; snippets docs no |
| Montaje automático en `base_path` | Documentación + path implícito | `router()` → `Router::nest` | **Superset** | Valida paths inválidos para Axum |
| `base_path` = `/` o vacío | Soportado en opciones | `normalize_base_path` → montaje en raíz | **Igual** | |
| `base_path` con trailing `/` | Normalización en URL utils | Trim en `normalize_base_path` | **Igual** | |
| Rechazar patrones `{param}` en mount | N/A en TS | `InvalidBasePath` | **Superset** | Evita colisión con sintaxis Axum |
| Métodos HTTP en catch-all | Todos los que llegan al handler | `any()` acepta todos; core puede 404 | **Parcial** | Tests: `HEAD`/`OPTIONS` en `/ok` → 404 (no delegados) |
| Rutas fuera de `base_path` | No manejadas por handler | 404 Axum | **Igual** | |
| Endpoints custom / plugins | Misma tabla de rutas | Test `extra_async_endpoint`, `plugin` | **Igual** | |
| Registry completo de rutas core | Tests en suite general BA | `every_core_auth_route_is_mounted` | **Superset** | Test único en crate axum |

## Conversión request

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| Preservar method, URI, version, headers | Web `Request` / Node adapter | `Request::from_parts` tras leer body | **Igual** | |
| Preservar extensions | Sí (Fetch) | Sí, salvo inserciones adaptador | **Igual** | |
| Leer body a bytes antes del core | Node: según body-parser; Fetch: stream | `to_bytes` + límite | **Parcial** | OpenAuth siempre bufferiza |
| Límite configurable de body | Express / servidor | `body_limit` (default 10 MiB) | **Superset** | JSON `413 PAYLOAD_TOO_LARGE` |
| Body inválido / error lectura | Host-dependent | `400 INVALID_REQUEST_BODY` | **Superset** | |
| `fromNodeHeaders` helper | `better-auth/node` | No exportado | **Hueco menor** | Usar conversión manual si hace falta |
| Pre-parsed body Express | `better-call` | N/A | **N/A** | Modelo Axum distinto |

## Conversión response

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| Status, headers, body | `Response` | `from_api_response` | **Igual** | |
| Headers duplicados (`Set-Cookie`) | Sí | Test preserva 2 cookies | **Igual** | |
| Response extensions | Sí | Test `ResponseExtensionMarker` | **Igual** | |
| HTTP version en respuesta | Sí | Test HTTP/2 | **Igual** | |
| Query string en URI | Sí | Test `handle_ref` | **Igual** | |

## IP cliente y rate limiting

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| IP para rate limit | Socket / headers configurables | `ConnectInfo<SocketAddr>` → `RequestClientIp` | **Parcial** | Requiere `into_make_service_with_connect_info` |
| Ignorar `X-Forwarded-For` por defecto | Comportamiento documentado BA | Test spoof sin efecto | **Igual** | |
| Confiar proxy para IP | `ipAddress.headers` | Misma config core + headers en test | **Parcial** | IP vía core, no reimplementado en axum |
| Desactivar `ConnectInfo` | N/A explícito en integración | `use_connect_info_for_ip(false)` | **Superset** | |

## Base URL pública

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| `baseURL` estático en config | `options.baseURL` | `OpenAuthOptions::base_url` | **Igual** | En core |
| Inferir desde `request.url` | `getBaseURL` en handler | `infer_base_url` + `uri_origin` | **Parcial** | OpenAuth: **opt-in** (`infer_base_url_from_request`) |
| Inferir desde `Host` | Implícito vía URL / entorno | `host_header_origin` (http loopback, else https) | **Parcial** | Heurística documentada en README |
| `x-forwarded-host` + `x-forwarded-proto` | Con `trustedProxyHeaders` | Con `trust_proxy_headers_for_base_url` | **Igual** | Ambos opt-in |
| Variables de entorno (`BETTER_AUTH_URL`, …) | `getBaseURL` loadEnv | **N/A en adaptador** | **N/A** | Configurar en despliegue Rust / env en builder |
| `window.location` fallback | `getBaseURL` | **N/A** | **N/A** | Client-only |
| Error si no hay base URL | `BetterAuthError` en handler | Core + sin inferencia | **Parcial** | OpenAuth exige `base_url` o inferencia explícita |
| Inyección `RequestBaseUrl` / OAuth override | Contexto dinámico BA | `RequestBaseUrl`, `OAuthBaseUrlOverride` | **Igual** | Vía extensions |
| `isDynamicBaseURLConfig` / host patterns | `resolveRequestContext`, `url.test.ts` | **Hueco** | **N/A** | No en Rust 1.6.9 |
| Validación proxy `validateProxyHeader` | 54 tests en `url.test.ts` | `is_valid_host` / `is_valid_proto` | **Parcial** | Ver [06-gaps-and-hardening.md](./06-gaps-and-hardening.md) |
| `fromNodeHeaders` | `better-auth/node` | No exportado | **Hueco menor** | Conversión manual |
| Solo `handler_async` en adaptador | `handler` async en BA | `handler_async` | **Igual** | `OpenAuth::handler` sync no se usa |
| Fail-closed rate limit sin IP en production | BA production | `use_connect_info_for_ip(false)` | **Superset** | Test en `security.rs` |

## Errores y logging

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| Error interno handler | Respuesta genérica | `INTERNAL_SERVER_ERROR` JSON + log | **Igual** | Sin filtrar panic al cliente |
| Errores JSON del core en rutas auth | Sí | Tests `error_contract` | **Igual** | Vía core, no adaptador |

## Integraciones framework (upstream only)

| Funcionalidad | Upstream | OpenAuth | Estado | Notas |
| --- | --- | --- | --- | --- |
| `toNextJsHandler` | Sí | **N/A** | **N/A** | |
| `nextCookies` (RSC / Server Actions) | Plugin | **N/A** | **N/A** | Cookies por `Set-Cookie` HTTP |
| `toNodeHandler` / Express | Sí | **N/A** | **N/A** | Crate Node futuro o manual |
| `toSvelteKitHandler` / middleware SK | Sí | **N/A** | **N/A** | |
| `toSolidStartHandler` | Sí | **N/A** | **N/A** | |
| `tanstackStartCookies` | Sí | **N/A** | **N/A** | |
| Hono `auth.handler(raw)` | Patrón doc | `handle_ref` equivalente | **Igual** | |

## API ergonómica Rust

| API OpenAuth | Propósito |
| --- | --- |
| `OpenAuthAxumExt::into_router` | Builder style |
| `router` / `router_with_options` | Funciones libres |
| `routes` / `routes_with_options` | Nest manual |
| `handle` / `handle_ref` / `*_with_options` | Integración custom sin `Router` |
| `OpenAuthAxumOptions` | `#[non_exhaustive]` para evolución |

## Qué NO debe entrar en paridad de este crate

Cualquier fila de la tabla de endpoints (`/sign-in/email`, OAuth, SCIM, Stripe, etc.) se
documenta en la paridad del crate de dominio (`openauth`, plugins). Aquí solo importa que
el adaptador **no devuelva 404** por error de montaje y que headers/cookies lleguen al core.
