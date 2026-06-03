# 05 — Layout de claves y comandos Redis

## Prefijos y patrones

| Uso | Patrón de clave Redis | Comandos principales |
| --- | --- | --- |
| Secondary storage (OpenAuth) | `{key_prefix}secondary:{logical_key}` | `GET`, `SET`, `SETEX`, `DEL` |
| Rate limit (OpenAuth) | `{key_prefix}rate-limit:{logical_key}` | `EVAL` (Lua), `HMGET`, `HSET`, `PEXPIRE` |
| Secondary storage (upstream) | `{key_prefix}{logical_key}` | `GET`, `SET`, `SETEX`, `DEL` |
| Rate limit (upstream vía secondary) | `{key_prefix}{logical_key}` (mismo espacio) | `GET`, `SET`, `SETEX` en string JSON |
| list/clear (upstream) | `{key_prefix}*` | `KEYS`, `DEL` |
| list/clear (`openauth-fred`) | `{key_prefix}secondary:*` | `SCAN`, `DEL` |

`key_prefix` por defecto: `openauth:` vs `better-auth:`.

## Script Lua (`RedisRateLimitStore`)

Comportamiento resumido (ver `crates/openauth-redis/src/lib.rs`):

1. `HMGET` `count`, `last_request`.
2. Si no hay datos o ventana expirada → reset `count=1`, `last_request=now`, `PEXPIRE window_ms`.
3. Si `count >= max` → denegar, refrescar `PEXPIRE`.
4. Si no → incrementar `count`, actualizar `last_request`, `PEXPIRE`.

**Decisión:** usar `HSET` (no `HMSET` deprecado) — test unitario `rate_limit_script_uses_current_hash_set_command`.

## Valkey y URLs

| URL entrada | Normalizado a | Motivo |
| --- | --- | --- |
| `valkey://host:port` | `redis://host:port` | `redis-rs` no registra esquema Valkey |
| `valkeys://host:port` | `rediss://host:port` | TLS + alias Valkey |

**Extensión OpenAuth** — upstream no documenta Valkey en el paquete redis-storage (ioredis usa URL estándar).

## TLS

| | Upstream | `openauth-redis` |
| --- | --- | --- |
| Activación | Opciones TLS en constructor ioredis | Features `rustls` o `native-tls` en crate |
| URLs `rediss://`, `valkeys://` | Soportadas vía ioredis | Requieren feature; sin feature → error al abrir cliente |
| Test | — | `tls_urls_open_as_tls_connections` (cfg feature) |

## Errores

| Situación | OpenAuth |
| --- | --- |
| Error de conexión / comando Redis | `OpenAuthError::Adapter(message)` |
| `window == 0` o `max == 0` en rate limit | `OpenAuthError::InvalidConfig` |
| Resultado Lua inválido | `OpenAuthError::Adapter` con mensaje explícito |

Upstream propaga errores de ioredis como rechazos de promesa sin tipo Rust unificado.

## Concurrencia

| | Upstream ioredis | `openauth-redis` |
| --- | --- | --- |
| Patrón | Un cliente compartido | `ConnectionManager` clonado por operación |
| Rate limit | No atómico entre GET/SET JS | Lua atómico |

El código actual clona `ConnectionManager` por operación (**sin** `Arc<Mutex>` en `src/lib.rs`). Un plan histórico (`2026-05-16-rate-limit-hardening-followups.md`) mencionaba serialización con mutex; no está en el código revisado.

## Interoperabilidad entre crates OpenAuth

`openauth-fred` tests verifican que `FredSecondaryStorage` y `RedisSecondaryStorage` comparten lectura/escritura en `{prefix}secondary:{key}` sobre la misma instancia.

**No** comparten layout de rate limit entre fred y redis-rs más allá del concepto (ambos usan Lua similar en sus crates).
