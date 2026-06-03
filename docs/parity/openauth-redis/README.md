# Paridad: `openauth-redis` ↔ `@better-auth/redis-storage`

Documentación de paridad **solo servidor** entre OpenAuth y Better Auth **v1.6.9**.

| Campo | Valor |
| --- | --- |
| Upstream npm | `@better-auth/redis-storage@1.6.9` |
| Upstream path | `reference/upstream-src/1.6.9/repository/packages/redis-storage/` |
| Crate Rust | `crates/openauth-redis` (`openauth-redis` en crates.io) |
| Crate hermano (mismo upstream) | [`openauth-fred`](../openauth-fred/README.md) — mismo contrato, cliente `fred` |
| Paridad pin | [`reference/upstream-better-auth/VERSION.md`](../../../reference/upstream-better-auth/VERSION.md) |
| Checklist histórico | [`docs/superpowers/plans/2026-05-12-redis-storage-upstream-checklist.md`](../../superpowers/plans/2026-05-12-redis-storage-upstream-checklist.md) |

## Relación de paquetes (split por cliente Redis)

| Rol | Upstream | OpenAuth |
| --- | --- | --- |
| Adaptador Redis secondary storage | `@better-auth/redis-storage` (`redisStorage`) | `RedisSecondaryStorage` en `openauth-redis` |
| Cliente Redis | `ioredis` (peer, lo crea la app) | `redis-rs` (`ConnectionManager`) |
| Variante alternativa de cliente | — (solo ioredis en el paquete oficial) | `openauth-fred` (`FredSecondaryStorage`, `FredRateLimitStore`) |
| Contrato secondary storage | `@better-auth/core` → `SecondaryStorage` | `openauth-core` → `SecondaryStorage` |
| Rate limit distribuido vía Redis | **No** en el paquete; core reutiliza `secondaryStorage` con JSON | `RedisRateLimitStore` + trait `RateLimitStore` (Lua atómico) |
| Sesiones / verificación / plugins | Consumen `secondaryStorage` en `better-auth` | Consumen `SecondaryStorage` en `openauth-core` / plugins |
| Tests E2E con Redis real | `e2e/smoke/test/redis.spec.ts` | `tests/redis_rate_limit.rs` + tests en `openauth-core` / `openauth-fred` |

**No hay merge de varios paquetes upstream en uno Rust:** es 1 paquete npm → 1–2 crates Rust según el driver (`redis-rs` vs `fred`). La extensión de rate limit es decisión de OpenAuth, no un segundo paquete upstream.

## Índice

| Documento | Contenido |
| --- | --- |
| [01-overview.md](./01-overview.md) | Resumen ejecutivo, alcance, estado de paridad |
| [02-package-mapping.md](./02-package-mapping.md) | Archivos upstream ↔ Rust, dependencias, features |
| [03-secondary-storage.md](./03-secondary-storage.md) | `get` / `set` / `delete`, TTL, prefijos, utilidades |
| [04-rate-limiting.md](./04-rate-limiting.md) | Divergencia principal: upstream vs `RedisRateLimitStore` |
| [05-key-layout-and-commands.md](./05-key-layout-and-commands.md) | Namespaces Redis, comandos, Valkey/TLS |
| [06-consumer-integration.md](./06-consumer-integration.md) | Quién usa secondary storage (core vs upstream) |
| [07-tests.md](./07-tests.md) | Matriz de tests upstream ↔ Rust |
| [08-logical-keys-and-payloads.md](./08-logical-keys-and-payloads.md) | Claves/payloads de sesión (core; migración Redis) |
| [09-upstream-file-audit.md](./09-upstream-file-audit.md) | Inventario completo del paquete npm |
| [10-findings-pass3.md](./10-findings-pass3.md) | Tercera pasada: ventana RL, API connect, prefijo vacío, pools |

## Hallazgos críticos (auditoría código, no README)

1. **Sesiones en Redis no son intercambiables** con Better Auth sin migración: claves y JSON distintos en core — [08](./08-logical-keys-and-payloads.md).
2. **`ttl=0`:** alineado con upstream y `openauth-fred` (`SET` sin TTL) — [03](./03-secondary-storage.md), [11](./11-gap-closure-status.md).
3. **Rate limit:** upstream incrementa en response + JSON en KV; OpenAuth consume en request con Lua — [04](./04-rate-limiting.md).
4. **Sin auto-wire de rate limit** al configurar solo `secondary_storage` (a diferencia de upstream `create-context.ts`).
5. **E2E Redis real** del adaptador upstream: solo `e2e/smoke` (4 casos, usa `listKeys`); producto OpenAuth en `openauth-fred`.
6. **Ventana rate limit:** Lua alineado con upstream (`>` en `window_ms`); tests de borde en redis y fred — [10](./10-findings-pass3.md).
7. **`connect_with_options`** y prefijo vacío rechazado — ver [11](./11-gap-closure-status.md) (histórico en [10](./10-findings-pass3.md)).

## Verificación rápida

```bash
cargo fmt --all --check
cargo clippy -p openauth-redis --all-targets -- -D warnings
cargo nextest run -p openauth-redis
```

Con Redis + Valkey (docker-compose / CI):

```bash
OPENAUTH_REDIS_URL=redis://127.0.0.1:6379 \
OPENAUTH_VALKEY_URL=valkey://127.0.0.1:6380 \
cargo nextest run -p openauth-redis
```

## Estado resumido (servidor)

| Área | Paridad | Notas |
| --- | --- | --- |
| Adaptador KV (`get`/`set`/`delete`) | **Alta** si se ignoran claves de sesión | Prefijo + `secondary:`; ver [03](./03-secondary-storage.md) |
| Datos de sesión en Redis | **Baja (interoperabilidad)** | Layout distinto en core; ver [08](./08-logical-keys-and-payloads.md) |
| `listKeys` / `clear` | **Gap en este crate** | Upstream en adaptador; `openauth-fred` con `SCAN` |
| Rate limit Redis | **Extensión + distinto timing** | Lua + consume en request; ver [04](./04-rate-limiting.md) |
| Valkey URLs / TLS features | **Extensión OpenAuth** | Upstream delega a ioredis |
| Tests del paquete npm | **0** en `packages/redis-storage` | 4 smoke + ~15 `it` secondary en better-auth; Rust 13 en crate + fred/core |
| Sesiones E2E con Redis real | **En `openauth-fred`**, no aquí | Smoke upstream requiere `listKeys` + layout BA |

Última revisión: **2026-06-01** (auditoría código/tests, pin **1.6.9**).
