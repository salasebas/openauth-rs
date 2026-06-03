# 01 — Resumen ejecutivo

## Qué es cada lado

**Upstream (`@better-auth/redis-storage`)** es un adaptador mínimo (~75 líneas) que implementa el contrato `SecondaryStorage` de `@better-auth/core` sobre un cliente **ioredis** inyectado por la aplicación. Expone además `listKeys()` y `clear()` para pruebas y operaciones admin. No define rate limiting propio: cuando `secondaryStorage` está configurado, Better Auth elige por defecto `rateLimit.storage = "secondary-storage"` y el rate limiter del core lee/escribe JSON en las **mismas** claves KV que sesiones y verificación (solo con el prefijo global).

**OpenAuth (`openauth-redis`)** implementa dos superficies sobre **redis-rs**:

1. **`RedisSecondaryStorage`** — trait `SecondaryStorage` de `openauth-core`.
2. **`RedisRateLimitStore`** — trait `RateLimitStore` con script Lua atómico (hash `count` / `last_request`), namespace `rate-limit:` separado del secondary storage.

Es un crate **opt-in** del workspace; el facade `openauth` no lo enlaza por defecto.

## Mapa de código

| Upstream | OpenAuth |
| --- | --- |
| `packages/redis-storage/src/index.ts` | `crates/openauth-redis/src/lib.rs` (todo el crate) |
| `packages/redis-storage/src/redis-storage.ts` | `RedisSecondaryStorage` + helpers en `lib.rs` |
| *(no existe)* | `RedisRateLimitStore` + `RATE_LIMIT_SCRIPT` en `lib.rs` |
| `packages/redis-storage/package.json` | `crates/openauth-redis/Cargo.toml` |
| `e2e/smoke/test/redis.spec.ts` | `tests/redis_rate_limit.rs` (parcial) + `openauth-fred` / `openauth-core` |

## Alcance de este análisis

**Incluido**

- API y comportamiento Redis del paquete `@better-auth/redis-storage`.
- Cómo upstream y OpenAuth usan secondary storage para sesiones, verificación y rate limit a nivel de **contrato** (sin reimplementar toda la lógica de `internal-adapter` / `session.rs` aquí).
- Tests que ejercitan Redis o el contrato secondary storage.

**Fuera de alcance (por diseño server-only / no pertenecen al crate)**

- SDK cliente, cookies en navegador, React, etc.
- Empaquetado npm (`tsdown`, `attw`, `publint`).
- Lógica completa de OAuth stateless / Google en smoke Redis (pertenece a `better-auth` / `openauth` core).
- Implementación de `openauth-fred` salvo comparación explícita (crate hermano; ver su [`PARITY.md`](../../../crates/openauth-fred/PARITY.md)).

## Conclusión de paridad (revisión código/tests)

| Dimensión | Valoración |
| --- | --- |
| Adaptador `get`/`set`/`delete` (TTL > 0, sin TTL) | **Alta** |
| Interoperabilidad datos sesión Better Auth ↔ OpenAuth | **Baja** — ver [08-logical-keys-and-payloads.md](./08-logical-keys-and-payloads.md) |
| `listKeys` / `clear` | **Sí** en `openauth-redis` y fred (`SCAN`) |
| `ttl = 0` | **Alineado** con upstream y fred |
| Rate limit Redis | **Extensión** + timing request vs response — [04](./04-rate-limiting.md) |
| Tests paquete npm upstream | **0** archivos; 4 smoke E2E |
| Tests crate `openauth-redis` | **19** `nextest`; sin E2E sign-up (fred sí) — [07](./07-tests.md) |

La paridad del **adaptador KV** no implica paridad del **contenido** que escribe el core. Smoke upstream (`listKeys`, token como clave, `{session,user}`) no lo reproduce `openauth-redis` ni el layout de `openauth-core`.
