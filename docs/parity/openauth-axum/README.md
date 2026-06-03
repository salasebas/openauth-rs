# Paridad: `openauth-axum` ↔ integraciones HTTP de Better Auth

Documentación de paridad **solo servidor** entre el adaptador Axum de OpenAuth y las
integraciones de framework de Better Auth **v1.6.9**.

| Campo | Valor |
| --- | --- |
| Crate Rust | `crates/openauth-axum` (`openauth-axum` en crates.io) |
| Paridad pin | [`reference/upstream-better-auth/VERSION.md`](../../../reference/upstream-better-auth/VERSION.md) |
| Upstream principal | `packages/better-auth/src/integrations/*` |
| Upstream secundario | `packages/better-auth/src/auth/base.ts` (handler + `baseURL`) |
| Upstream Node (dependencia) | `better-call@1.3.5` → `better-call/node` (no está en el clone del monorepo) |
| Checklist histórico | [`docs/superpowers/plans/2026-05-16-openauth-axum.md`](../../superpowers/plans/2026-05-16-openauth-axum.md) |
| README del crate | [`crates/openauth-axum/README.md`](../../../crates/openauth-axum/README.md) |

## Cómo leer esta comparación

Better Auth **no tiene** paquete `@better-auth/axum` ni equivalente Rust. El crate
`openauth-axum` cumple el rol de **adaptador HTTP del servidor** que en el ecosistema
TS se reparte entre:

| Patrón upstream | Analogía |
| --- | --- |
| `better-auth/next-js` → `toNextJsHandler` | Montaje de rutas + passthrough `Request` |
| `better-auth/node` → `toNodeHandler` | Conversión request/response (vía `better-call`) |
| Hono / Fetch directo → `auth.handler(c.req.raw)` | **Más cercano conceptualmente** a Axum: ya hay un `Request` tipado |
| Plugins `nextCookies`, `sveltekitCookies`, … | **No portados** (cookies vía `Set-Cookie` en respuesta HTTP) |

La lógica de auth (rutas, plugins, sesiones, CSRF, rate limit) vive en `openauth` /
`openauth-core`. Este crate solo **monta** y **traduce** HTTP.

## Relación de paquetes

| Rol | Upstream (1.6.9) | OpenAuth |
| --- | --- | --- |
| Handler framework-neutral | `better-auth` → `auth.handler(Request)` | `openauth` → `OpenAuth::handler_async(ApiRequest)` |
| Integración Next / Solid | Subpath `better-auth/next-js`, `solid-start` | **N/A** (TS / meta-framework) |
| Integración Node / Express | Subpath `better-auth/node` → `better-call/node` | **Parcial** en `openauth-axum` (cuerpo, IP, base URL) |
| Integración SvelteKit / TanStack | Subpaths + plugins cookies | **N/A** |
| Adaptador Axum | **No existe** | **`openauth-axum`** (crate dedicado) |

**Split de empaquetado:** upstream concentra todas las integraciones en el mismo paquete
`better-auth` como exports. OpenAuth **separa** el adaptador Axum en un crate opcional
(del mismo modo que en el futuro podrían existir `openauth-actix`, etc., sin inflar el
crate principal).

## Índice

| Documento | Contenido |
| --- | --- |
| [01-overview.md](./01-overview.md) | Resumen ejecutivo, alcance, diagrama de flujo |
| [02-package-mapping.md](./02-package-mapping.md) | Mapa módulo ↔ archivo upstream, API pública |
| [03-adapter-features.md](./03-adapter-features.md) | Tabla función por función y estado de paridad |
| [04-design-decisions.md](./04-design-decisions.md) | Divergencias intencionales y límites Rust/server-only |
| [05-tests.md](./05-tests.md) | Inventario exhaustivo de los 72 tests + matriz upstream |
| [06-gaps-and-hardening.md](./06-gaps-and-hardening.md) | API sin test, `getBaseURL` vs inferencia, validación proxy, huecos |
| [07-path-mounting-and-footguns.md](./07-path-mounting-and-footguns.md) | URI/nest, `base_path` vs `base_url`, Hono/Express docs, body middleware |

## Verificación rápida

```bash
cargo fmt --all --check
cargo clippy -p openauth-axum --all-targets -- -D warnings
cargo nextest run -p openauth-axum
```

| Métrica | Upstream (integraciones) | OpenAuth (`openauth-axum`) |
| --- | --- | --- |
| Archivos de integración | 6 `.ts` (+ 1 test) | 6 módulos `src/*.rs` |
| LOC adaptador (aprox.) | ~350 TS integraciones | ~543 Rust `src/` |
| Tests Vitest integración | **5** (`next-js.test.ts`, plugin cookies) | — |
| Tests Vitest `getBaseURL` / proxy | **54** (`utils/url.test.ts`) | Parcial (4 tests inferencia en `password`/`social`) |
| Tests Vitest handler HTTP | **0** dedicados a `toNextJsHandler` | — |
| Tests `better-call/node` | **~23** (paquete externo) | Cubierto parcialmente por tests Axum |
| Tests Rust (`nextest list`) | — | **72** (ver [05-tests.md](./05-tests.md)) |
| Clase A adaptador puro | — | **20** |
| Clase B IP/rate + Axum | — | **6** |
| Clase C core vía Axum | sign-in/up `.test.ts` (fetch metadata, urlencoded) | **8** |
| Clase D E2E auth montado | Suites API `better-auth` | **20** |

## Estado resumido (capa adaptador)

| Área | Paridad con BA 1.6.9 | Notas |
| --- | --- | --- |
| Montaje catch-all bajo `base_path` | **Alta** | Axum `any()` + `nest`; upstream delega al router `better-call` |
| Preservación headers / status / body | **Alta** | Web API ↔ `ApiRequest`/`ApiResponse` |
| Límite de cuerpo configurable | **Superset** | OpenAuth: 10 MiB por defecto + `413` JSON; Node: límites de `better-call`/body-parser |
| IP cliente (rate limit) | **Equivalente (diferente mecanismo)** | `ConnectInfo<SocketAddr>` vs socket Node / headers |
| Inferencia `base_url` | **Equivalente (opt-in)** | Upstream en `handler` si falta `baseURL`; OpenAuth en adaptador con flags explícitos |
| Plugins cookies framework | **N/A** | Diseño server-only |
| `toNextJsHandler` / RSC | **N/A** | TS-only |
| Cobertura de flujos auth montados | **Superset en este crate** | Muchos tests E2E que upstream no tiene en `integrations/` |

Última auditoría documentada: **2026-06-01** (segunda pasada: código + tests + docs upstream Hono/Express/Next + `path.rs`).

**Nuevo en segunda pasada:** [07-path-mounting-and-footguns.md](./07-path-mounting-and-footguns.md) — URI/nest, desacople `base_path`/`base_url`, no parsear body antes del adaptador, `is_loopback` duplicado, CLI vs `into_routes`.
