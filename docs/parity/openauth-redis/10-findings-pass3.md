# 10 — Hallazgos adicionales (tercera pasada código/tests)

Revisión directa de `crates/openauth-redis/src/lib.rs`, `openauth-fred`, `openauth-core` (`session.rs`, `rate_limit.rs`, `utils/ip.rs`) y `packages/redis-storage` + `rate-limiter/index.ts` upstream. Complementa [08](./08-logical-keys-and-payloads.md) y [09](./09-upstream-file-audit.md).

## Rate limit: ventana exacta (`>=` vs `>`) — **cerrado**

| Implementación | Condición de “fin de ventana” / reset |
| --- | --- |
| Upstream `onResponseRateLimit` | Reset si `(now - lastRequest) > window * 1000` |
| Upstream `shouldRateLimit` (denegar) | Deniega si `(now - lastRequest) < window * 1000` y `count >= max` |
| `GovernorMemoryRateLimitStore` | Sigue en ventana si `(now - last_request) <= window_ms` |
| Lua `RATE_LIMIT_SCRIPT` (`openauth-redis` / `fred`) | Reset si `(now - last_request) > window_ms` (alineado con upstream) |

En `now - last_request === window_ms`, Lua **no** resetea (misma semántica que upstream en el borde).

**Tests:** `redis_rate_limit_store_does_not_reset_at_exact_window_boundary`, `fred_rate_limit_store_does_not_reset_at_exact_window_boundary`, más `*_resets_after_window` en ambos crates.

**Estado actual:** [11-gap-closure-status.md](./11-gap-closure-status.md).

## Rate limit: `reset_after` con petición permitida

| Backend | `reset_after` cuando `permitted == true` |
| --- | --- |
| `GovernorMemoryRateLimitStore` | `input.rule.window` (segundos fijos) |
| `RedisRateLimitStore` / `FredRateLimitStore` | `ceil_millis_to_seconds(last_request + window_ms - now_ms)` |

En la primera petición de un bucket suelen coincidir (~`window`). En peticiones posteriores dentro de la ventana, Redis/Fred devuelven el tiempo **restante** hasta el borde de ventana; memoria devuelve siempre `window`.

**Clasificación:** decisión OpenAuth unificada en trait; headers `Reset-After` pueden diferir entre memory y Redis en el mismo `OpenAuth` config.

## API del crate: `connect_with_options` — **cerrado**

| Método | `openauth-redis` | `openauth-fred` |
| --- | --- | --- |
| `connect(url)` | Sí | Sí |
| `connect_with_options(url, options)` | Sí (secondary, rate limit, `RedisOpenAuthStores`) | Sí |
| Prefijo vacío | Rechazado (`InvalidConfig`) | Rechazado |

**Estado:** [11-gap-closure-status.md](./11-gap-closure-status.md).

## Dos `connect()` a la misma URL

Cada `RedisRateLimitStore::connect` y `RedisSecondaryStorage::connect` por separado crea dos pools. **`RedisOpenAuthStores::connect_with_options`** comparte un `ConnectionManager` (test `redis_open_auth_stores_share_one_connection_manager`). Upstream suele compartir un solo cliente `ioredis`.

**Clasificación:** documentación operativa; usar bundle o `::new(manager, …)` para un pool.

## `LegacyRateLimitStorageAdapter` no une Redis secondary + rate limit JSON

`RateLimitOptions::custom_storage` es `Arc<dyn RateLimitStorage>`, **no** `SecondaryStorage`.

No hay en el workspace un adaptador que implemente `RateLimitStorage` leyendo/escribiendo JSON vía `RedisSecondaryStorage` como hace upstream con `getRateLimitStorage`. Por tanto **no** existe atajo “solo `secondary_storage`” equivalente a Better Auth para rate limit en Redis.

## Upstream `refreshUserSessions`

`internal-adapter.ts` define `refreshUserSessions` que reescribe entradas de sesión en secondary storage cuando cambia el usuario.

**OpenAuth:** sin equivalente grep-able en el workspace.

**Clasificación:** gap de core, no del crate redis; afecta datos en Redis tras updates de usuario.

## Plugins API key y el mismo Redis

`openauth-plugins` usa `SecondaryStorage` global con claves lógicas `api-key:{hash}`, `api-key:by-id:{id}`, etc. (`api_key/storage/keys.rs`).

El adaptador `RedisSecondaryStorage` es agnóstico; las claves viven bajo `openauth:secondary:api-key:…`. No es parte del paquete npm upstream redis-storage, pero **sí** consume el mismo trait si el usuario cablea Redis.

## CI

`.github/workflows/ci.yml` ejecuta `cargo nextest run -p openauth-redis --all-features` (incluye test TLS si compila).

## Corrección documentación previa

- [06-consumer-integration.md](./06-consumer-integration.md) decía que sesiones usan las “mismas formas de clave lógica” que upstream — **incorrecto**; corregido en ese archivo.
- Paridad “~95%” en `docs/parity/README.md` es solo del adaptador KV; **interoperabilidad de datos** sigue siendo baja ([08](./08-logical-keys-and-payloads.md)).
