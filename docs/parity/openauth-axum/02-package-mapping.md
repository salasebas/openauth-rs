# Mapeo de paquetes y módulos

## Empaquetado: 1 paquete upstream → 1 crate Rust (+ core)

| Capa | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| Runtime auth | `better-auth` | `openauth` (re-exporta API pública) |
| Router / endpoints | `better-auth` + `better-call` | `openauth-core` |
| Adaptador HTTP Axum | **No existe** | **`openauth-axum`** |
| Adaptador Node/Express | `better-auth/node` → `better-call/node` | **No portado** (usar otro crate futuro o integración manual) |
| Integraciones meta-framework | Subpaths en `better-auth` | **N/A** (server-only) |

No hay merge ni split adicional respecto a un plugin npm: es un **crate de integración**
opcional, análogo a cómo `openauth-stripe` es 1:1 con `@better-auth/stripe`.

## Mapa archivo ↔ módulo

### OpenAuth (`crates/openauth-axum`)

| Archivo Rust | Responsabilidad |
| --- | --- |
| `src/lib.rs` | Re-exports públicos |
| `src/router.rs` | `router`, `routes`, `handle*`, `OpenAuthAxumExt`, `base_path`, inferencia `base_url` |
| `src/request.rs` | `to_api_request`, body limit, `ConnectInfo` → `RequestClientIp` |
| `src/response.rs` | `from_api_response` |
| `src/options.rs` | `OpenAuthAxumOptions` |
| `src/error.rs` | `OpenAuthAxumError`, respuestas JSON `400`/`413`/`500` del adaptador |
| `tests/common/mod.rs` | Helpers Tower `oneshot`, endpoints de contrato |
| `tests/routing.rs` | Montaje, catch-all, registry de rutas core |
| `tests/http_contract.rs` | Preservación HTTP metadata |
| `tests/body_limit.rs` | Límite de cuerpo |
| `tests/error_contract.rs` | Errores adaptador + sanitización 500 |
| `tests/security.rs` | Rate limit + `ConnectInfo` / `X-Forwarded-For` |
| `tests/security_upstream.rs` | Seguridad core (CSRF fetch metadata, etc.) vía Axum |
| `tests/*.rs` (resto) | Flujos auth E2E montados (email, OAuth, cuentas, …) |

### Upstream (`reference/upstream-src/1.6.9/repository/packages/better-auth/src/integrations/`)

| Archivo TS | Export / API | Paridad OpenAuth |
| --- | --- | --- |
| `next-js.ts` | `toNextJsHandler`, `nextCookies` | Montaje ≈ `router`/`routes`; cookies plugin **N/A** |
| `node.ts` | `toNodeHandler`, `fromNodeHeaders` | Conversión parcial en `request.rs` + sin API `fromNodeHeaders` pública |
| `solid-start.ts` | `toSolidStartHandler` | **N/A** |
| `svelte-kit.ts` | `toSvelteKitHandler`, `svelteKitHandler`, `isAuthPath`, `sveltekitCookies` | **N/A** (path filter en SK; Axum usa `nest` + catch-all) |
| `tanstack-start.ts` | `tanstackStartCookies` | **N/A** |
| `tanstack-start-solid.ts` | `tanstackStartCookies` (Solid) | **N/A** |
| `next-js.test.ts` | Tests `nextCookies` / RSC | Sin equivalente |

### Upstream core (acoplamiento handler)

| Archivo TS | Relación con adaptador |
| --- | --- |
| `auth/base.ts` | `handler(request)` → `router().handler(request)`; resuelve `baseURL` si falta |
| `utils/url.ts` | `getBaseURL`, `getOrigin`, validación proxy headers |
| `api/index.ts` | Registro de endpoints y `createRouter` |

### Dependencia externa `better-call` (Node)

| Módulo | Rol | Equivalente OpenAuth |
| --- | --- | --- |
| `better-call/node` `toNodeHandler` | `IncomingMessage` ↔ `Request`/`Response` | No hay crate; patrones similares en buffering/IP en axum |
| `adapters/node/request.ts` | Body pre-parseado Express, `baseUrl`, streaming | Axum siempre bufferiza en adaptador |

El monorepo clonado **no incluye** el código fuente de `better-call`; la versión pin está
en el catalog de pnpm del upstream (`1.3.5`).

## API pública comparada

### Montaje

| Upstream | OpenAuth |
| --- | --- |
| `toNextJsHandler(auth)` → `{ GET, POST, PATCH, PUT, DELETE }` | `router(auth)` → `Result<Router, OpenAuthAxumError>` |
| `app.all("/api/auth/*", toNodeHandler(auth))` | `OpenAuth::into_router()` o `nest(base_path, into_routes())` |
| `auth.handler(request)` directo | `handle_ref(&auth, request)` |

### Opciones solo adaptador

| OpenAuth | Upstream más cercano |
| --- | --- |
| `OpenAuthAxumOptions::body_limit` | Tamaño máximo body en `better-call` / middleware Express |
| `use_connect_info_for_ip` | IP socket en Node; headers con `advanced.ipAddress` |
| `infer_base_url_from_request` | `getBaseURL` en `handler` cuando `options.baseURL` vacío |
| `trust_proxy_headers_for_base_url` | `advanced.trustedProxyHeaders` + `x-forwarded-*` en `getBaseURL` |

### Errores adaptador

| OpenAuth | Upstream |
| --- | --- |
| `OpenAuthAxumError::InvalidBasePath` | No aplica (paths TS no usan sintaxis Axum `{*path}`) |
| JSON `PAYLOAD_TOO_LARGE` / `INVALID_REQUEST_BODY` | Depende del host HTTP |

## Exports npm relevantes (`better-auth/package.json`)

Subpaths que compiten conceptualmente con `openauth-axum`:

- `better-auth/next-js`
- `better-auth/node`
- `better-auth/svelte-kit`
- `better-auth/solid-start`
- `better-auth/tanstack-start` (+ `/solid`)

Ninguno menciona Axum, Actix, ni Rocket.

## Dependencias

| Crate / paquete | Propósito |
| --- | --- |
| `openauth-axum` → `openauth` | Handler y tipos `ApiRequest`/`ApiResponse` |
| `openauth` → `openauth-core` | Router y lógica auth |
| `better-auth` → `better-call` | Router HTTP Fetch |
| `better-auth` → `@better-auth/core` | Tipos y middleware de plugins |

## Ejemplos en el repo (código real)

| Ejemplo | Montaje | Notas |
| --- | --- | --- |
| `examples/full-app` | `auth.into_routes()` + `.nest("/api/auth", …)` | No usa `router()` one-shot; `main` con `into_make_service_with_connect_info::<SocketAddr>` |
| `examples/stripe-smoke-server` | `Router::new().nest(AUTH_BASE_PATH, auth.into_routes())` | Smoke Stripe; `OpenAuthAxumExt` importado |

## Dependencias dev del crate

| Dev-dep | Uso en tests |
| --- | --- |
| `tower` | `ServiceExt::oneshot` en todos los integration tests |
| `tokio` | Runtime async |
| `url` | Parse query en `FakeProvider` / OAuth tests |
