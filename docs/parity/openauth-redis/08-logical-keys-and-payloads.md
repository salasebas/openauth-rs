# 08 — Claves lógicas y payloads (core + Redis)

El adaptador `RedisSecondaryStorage` solo aplica prefijo físico (`{prefix}secondary:`). **Qué** se guarda lo define `openauth-core` (o `better-auth` upstream). Migrar datos Redis entre Better Auth y OpenAuth **no** es copiar claves tal cual aunque el adaptador sea equivalente.

## Tabla: claves lógicas (antes del prefijo Redis)

| Dato | Upstream Better Auth 1.6.9 (`internal-adapter.ts`) | OpenAuth (`session.rs` / `verification.rs`) | ¿Compatible? |
| --- | --- | --- | --- |
| Sesión por token | Clave = **token crudo** (p. ej. `abc123…`) | Clave = `session:{token}` | **No** |
| Valor sesión | JSON `{ session, user }` | JSON solo `Session` (usuario va por DB u otra vía en get-session) | **No** |
| Índice por usuario | `active-sessions-{userId}` → `[{ token, expiresAt }, …]` | `session:user:{userId}` → `["token1", …]` | **No** |
| Verificación | `verification:{processedIdentifier}` | `verification:{identifier}` | **Sí** (misma forma; processed depende de opciones) |
| Rate limit (si usa secondary KV) | Clave = `{ip}\|{path}` (sin prefijo extra) | No usa secondary para RL; usa `rate-limit:{ip}\|{path}` en otro namespace | **No** |

## Claves físicas en Redis (ejemplo prefijos por defecto)

| Dato | Upstream (`better-auth:` + key) | OpenAuth (`openauth:secondary:` + key) |
| --- | --- | --- |
| Sesión | `better-auth:{token}` | `openauth:secondary:session:{token}` |
| Índice usuario | `better-auth:active-sessions-{userId}` | `openauth:secondary:session:user:{userId}` |
| Verificación | `better-auth:verification:{id}` | `openauth:secondary:verification:{id}` |

Fuente upstream sesión: `createSession` hook en `packages/better-auth/src/db/internal-adapter.ts` (~350–404).  
Fuente OpenAuth: `SessionStore::set_secondary_session` / `set_user_session_tokens` en `crates/openauth-core/src/session.rs`.

## TTL desde el core (no del adaptador Redis)

| Comportamiento | Upstream | OpenAuth |
| --- | --- | --- |
| Cálculo TTL desde `expiresAt` | `getTTLSeconds` → `Math.max(Math.floor((expires - now) / 1000), 0)` | `ttl_seconds()` → `max(whole_seconds, 0)` como `u64` |
| Si TTL calculado es `0` | **No escribe** sesión/lista (`if (sessionTTL > 0)` / `furthestSessionTTL > 0`) | Si llamara `set(..., Some(0))` → `SET` persistente (adaptador alineado) |
| Índice `active-sessions` / `session:user` | TTL = expiración de la sesión más lejana | `set_user_session_tokens` usa `ttl_seconds: None` (sin expiración Redis en el índice) |

**Implicación:** el adaptador ya no diverge en `Some(0)`; el core sigue evitando escribir cuando TTL derivado ≤ 0. Ver [03](./03-secondary-storage.md), [11](./11-gap-closure-status.md).

## `get` y JSON pre-parseado

Upstream `SecondaryStorage.get` tipado como `unknown`; tests simulan Redis que devuelve objetos ya parseados (`internal-adapter.test.ts`, bloque `safeJSONParse date revival`).  
OpenAuth: `get` → siempre `Option<String>`; deserialización en core con `serde_json`.

**N/A en Rust** para el adaptador redis-rs (siempre string).

## Smoke E2E upstream y `listKeys`

`e2e/smoke/test/redis.spec.ts` asume:

- `auth.options.secondaryStorage.listKeys()` — **no** parte del trait `SecondaryStorage` en core; es método extra de `redisStorage()`.
- Tras sign-up: **2** claves (`active-sessions-…` + token).
- Payload sesión con `.user.id` y `.session.id` en un solo JSON.

OpenAuth no puede pasar ese smoke sin cambiar core o añadir `list_keys` y el layout upstream.

## Verificación

Tests que fijan layout OpenAuth (no upstream):

- `openauth-core` `sign_up_email_route_uses_secondary_storage_for_sessions` — claves `session:{token}`, `session:user:{user_id}`.
- `openauth-fred` `openauth_email_signup_uses_fred_secondary_storage_for_sessions` — mismo contrato OpenAuth con Redis real.
