# Decisiones de diseño y divergencias

Cada ítem indica **por qué** OpenAuth no replica literalmente Better Auth en la capa Axum.

## 1. Crate separado en lugar de subpath en `openauth`

| | Better Auth | OpenAuth |
| --- | --- | --- |
| Forma | `better-auth/next-js`, `better-auth/node`, … | `openauth-axum` en workspace |

**Motivo:** En Rust, las integraciones de framework son dependencias opcionales con
grafos distintos (Axum trae `tokio`, `tower`, etc.). Mantener el core sin `axum` permite
binarios que solo usan Actix u otro stack, o tests del core sin framework.

**Tipo:** Decisión de diseño (ecosistema Rust).

## 2. No portar plugins de cookies (`nextCookies`, `sveltekitCookies`, …)

Upstream escribe cookies vía APIs de framework (`next/headers`, SvelteKit `cookies`) cuando
el handler se invoca fuera del camino HTTP “puro” (RSC, server actions).

**OpenAuth:** server-only; la sesión sale en `Set-Cookie` en la respuesta HTTP. El cliente
(navegador u otro servicio) persiste cookies como en cualquier API REST.

**Tipo:** Server-only + sin runtime Next/Svelte en Rust.

## 3. Inferencia de `base_url` desactivada por defecto

Upstream (`auth/base.ts`) puede derivar `baseURL` del primer request si no está configurado,
usando `getBaseURL` (origin, proxy, env).

**OpenAuth:** `infer_base_url_from_request(false)` por defecto; producción debe setear
`OpenAuthOptions::base_url`. La inferencia existe para dev/smoke y paridad funcional con
OAuth redirects cuando se opta explícitamente.

**Motivo:** Inferencia desde `Host` o headers forjables es un footgun detrás de proxies;
el changelog 0.0.6 documenta endurecimiento (no tratar `Host` como trusted origin).

**Tipo:** Decisión de seguridad (más estricto que BA por defecto; alineable activando flags).

## 4. Proxy headers para base URL: doble opt-in

Upstream: `advanced.trustedProxyHeaders` en opciones globales.

OpenAuth: requiere **ambos** `infer_base_url_from_request(true)` y
`trust_proxy_headers_for_base_url(true)`.

**Tipo:** Decisión de seguridad (defensa en profundidad en el adaptador).

## 5. Buffer obligatorio del body

Axum expone `Body` como stream; el core OpenAuth consume `Vec<u8>` (paridad con modelo
`ApiRequest` tipo HTTP estándar materializado).

**Trade-off:** Memoria acotada por `body_limit` (default 10 MiB) vs streaming ilimitado.

**Comparación upstream:** `better-call` puede reutilizar `req.body` ya parseado en Express;
en Fetch/Next el body también se consume al leer.

**Tipo:** Limitación práctica Rust + diseño API core (no TS stream en handler).

## 6. Validación de `base_path` para Axum

Paths con `{auth}`, `*`, `?`, `#` se rechazan con `OpenAuthAxumError::InvalidBasePath`.

**Motivo:** `Router::nest` y rutas catch-all `{*path}` reservan sintaxis; Better Auth solo
usa strings de configuración sin validar contra un router Rust.

**Tipo:** Requisito del framework (Axum).

## 7. `ConnectInfo` para IP del socket

No existe equivalente portable en la Web Fetch API. Axum requiere
`into_make_service_with_connect_info::<SocketAddr>()`.

**Motivo:** Rate limiting por IP real en despliegues sin proxy; alineado con recomendaciones
del README del crate.

**Tipo:** Decisión de diseño + requisito Axum.

## 8. Errores JSON en la capa adaptador

Códigos `PAYLOAD_TOO_LARGE`, `INVALID_REQUEST_BODY`, `INTERNAL_SERVER_ERROR` se generan en
`openauth-axum` antes o en lugar de propagar detalles internos.

**Motivo:** Contrato HTTP estable para clientes aunque el core devuelva otros errores en
rutas montadas.

**Tipo:** Decisión de diseño (contrato explícito del adaptador).

## 9. Catch-all `any()` vs exportar métodos HTTP

`toNextJsHandler` expone solo GET/POST/PATCH/PUT/DELETE para el App Router.

**OpenAuth:** un handler `any` delega al core, que decide método permitido por ruta.

**Motivo:** Paridad con router interno que ya distingue métodos; evita duplicar tabla en
el adaptador. Tests documentan 404 para métodos no soportados en rutas de diagnóstico.

**Tipo:** Decisión de diseño (menos superficie, mismo core).

## 10. Tests E2E de auth en el crate axum

Muchos tests (`email_password`, `social`, `accounts`, …) podrían vivir solo en `openauth`.
Se mantienen aquí para garantizar que **ningún** flujo se rompe por el puente HTTP.

**Upstream:** esos escenarios están en tests generales de `better-auth`, no en
`integrations/`.

**Tipo:** Decisión de calidad (contrato de integración), no paridad 1:1 de archivos de test.

## 11. Sin `better-call` / Node en el workspace

No se empaqueta `openauth-node` aún. Usuarios Express deben convertir manualmente o esperar
un crate futuro.

**Tipo:** Alcance del producto (Rust server ecosystem centrado en Axum primero).

## 12. Estado “Experimental beta”

README del crate advierte cambios en opciones y composición del router.

**Tipo:** Política de release, no divergencia funcional con BA.

## Matriz rápida: ¿bug o diseño?

| Observación | Clasificación |
| --- | --- |
| No hay `nextCookies` | Diseño server-only |
| `infer_base_url` off por defecto | Seguridad / diseño |
| 54 tests vs 5 en `integrations/` | Más cobertura de montaje + E2E; no comparable 1:1 |
| Sin adaptador Actix/Rocket en repo | Alcance; telemetry los lista como futuros |
| HEAD → 404 en `/ok` de prueba | Comportamiento core + montaje; documentado en tests |

## 13. `security_upstream.rs` no prueba el adaptador — prueba el tubo HTTP

Los 8 tests de fetch metadata y `application/x-www-form-urlencoded` ejercitan el **core** con
cuerpo ya bufferizado por Axum. Upstream los tiene en `sign-in.test.ts` / `sign-up.test.ts`
contra `auth.handler(Request)` directo, no en `integrations/`.

**Motivo de mantenerlos en `openauth-axum`:** garantizar que Content-Type y headers
(`Sec-Fetch-*`, `Origin`) sobreviven `to_api_request`.

## 14. Validación de proxy más simple que `url.test.ts`

Better Auth tiene **54** tests en `utils/url.test.ts` para `getBaseURL` y headers proxy.
OpenAuth concentra la inferencia en el adaptador con validadores locales más cortos.

**Riesgo documentado:** no asumir paridad bit-a-bit con `validateProxyHeader` hasta portar
tests o compartir lógica con el core. Ver [06-gaps-and-hardening.md](./06-gaps-and-hardening.md).

## Referencias internas

- [`crates/openauth-axum/CHANGELOG.md`](../../../crates/openauth-axum/CHANGELOG.md) — cambios de inferencia base URL
- [`docs/superpowers/plans/2026-05-16-openauth-axum.md`](../../superpowers/plans/2026-05-16-openauth-axum.md) — checklist implementación
- [`05-tests.md`](./05-tests.md) — inventario de los 54 tests
- [`06-gaps-and-hardening.md`](./06-gaps-and-hardening.md) — huecos y API sin cobertura
