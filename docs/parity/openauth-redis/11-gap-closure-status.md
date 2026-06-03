# 11 — Estado de cierre de gaps (2026-06-02)

Documento de cierre: qué quedó implementado en `openauth-redis` / hermanos y qué **no** tiene sentido perseguir.

## Cerrado en código (adaptador `@better-auth/redis-storage`)

| Gap | Estado |
| --- | --- |
| `list_keys` / `clear` con `SCAN` | **Hecho** — `RedisSecondaryStorage` |
| `connect_with_options` / `connect_redis` / `connect_valkey` | **Hecho** — ambos stores |
| `RedisOpenAuthStores` (un `ConnectionManager`) | **Hecho** — `src/bundle.rs` |
| `ttl = 0` → `SET` sin TTL (como upstream) | **Hecho** |
| Prefijo vacío rechazado | **Hecho** — secondary + rate limit |
| `scan_count` en options | **Hecho** |
| Tests integración secondary (list/clear/ttl-zero) | **Hecho** — `redis_rate_limit.rs` |
| Tests ventana RL (post-window + borde exacto) | **Hecho** — Lua usa `>` como upstream |
| Lua `>` vs `>=` en reset | **Hecho** — `openauth-redis` + `openauth-fred` |

## Cerrado en tests (crate `openauth-redis`)

**19** tests `nextest` (antes 13): +2 ventana RL, +2 validación options, +1 integración ampliada, +1 bundle.

## No se implementará (sin valor o fuera de alcance del crate)

| Tema | Motivo |
| --- | --- |
| Intercambiar datos de sesión Better Auth ↔ OpenAuth | **Core** (`session.rs`); requiere migración o capa de compat, no el adaptador Redis |
| `listKeys` alcance `prefix*` (incluye rate-limit JSON upstream) | OpenAuth separa namespaces; Fred/redis listan solo `secondary:` — **decisión** |
| Auto `rateLimit.storage` al configurar solo `secondary_storage` | **Core** (`validate_rate_limit_storage`); upstream default en `create-context.ts` |
| `refreshUserSessions` upstream | **Core**; no existe en OpenAuth |
| Puente `RateLimitStorage` JSON sobre `RedisSecondaryStorage` | Usar `RedisRateLimitStore` o implementar adaptador custom en app |
| E2E OAuth stateless smoke upstream | **Core** / providers; `openauth-fred` cubre sesión email |
| Prefijo default `better-auth:` | **Decisión** OpenAuth (`openauth:`); configurable vía options |
| Segmento `secondary:` en clave física | **Decisión** OpenAuth (colisión rate limit / tipos Redis) |
| Re-export desde facade `openauth` | **Decisión** crate opt-in |

## Paridad documental

Los docs [01](./01-overview.md)–[10](./10-findings-pass3.md) describen diferencias históricas; este archivo es la fuente de verdad del **estado actual** del crate.

## Cuándo parar

No tiene sentido más trabajo en **este crate** salvo:

- Producto pida migración de claves Better Auth en core.
- Se añada E2E sign-up en `openauth-redis` duplicando `openauth-fred` (bajo valor; fred ya lo cubre).
- Cambio de política de prefijo default o layout sin `secondary:`.
