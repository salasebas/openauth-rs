# 02 — Mapeo de paquetes y API pública

## Identidad

| | Upstream | OpenAuth |
| --- | --- | --- |
| Nombre publicado | `@better-auth/redis-storage` | `openauth-redis` |
| Versión pin paridad | `1.6.9` | workspace `0.0.6` (ver `Cargo.toml`) |
| Licencia | MIT | MIT (workspace) |
| Documentación oficial | [better-auth.com/docs/storage](https://www.better-auth.com/docs/storage) | README del crate + esta carpeta |

## Dependencias

| Concern | Upstream | OpenAuth (`openauth-redis`) |
| --- | --- | --- |
| Cliente Redis | `ioredis` **peer** `^5.0.0` (app lo instancia) | `redis` `0.32` con `tokio-comp`, `connection-manager`, `script` |
| Contrato storage | `@better-auth/core` (`SecondaryStorage`) | `openauth-core` (`SecondaryStorage`, `RateLimitStore`) |
| Runtime async | Promises (Node) | Tokio + futures en traits (`SecondaryStorageFuture`, `RateLimitFuture`) |
| TLS | Configuración del cliente ioredis | Features crate: `native-tls` \| `rustls` → flags de `redis-rs` |
| Tests en paquete | `vitest` declarado, **sin archivos de test** | `cargo test` / `nextest`, integration con Redis/Valkey |
| `thiserror` | — | En `Cargo.toml`; **no usado** en `lib.rs` |

## Features Cargo (solo OpenAuth)

| Feature | Efecto |
| --- | --- |
| `default` | Sin TLS en `redis-rs` |
| `native-tls` | `redis/tokio-native-tls-comp` |
| `rustls` | `redis/tokio-rustls-comp` |

Upstream no tiene features equivalentes en el paquete: TLS vive en ioredis.

## API pública — tabla comparativa

| Upstream | OpenAuth | Paridad | Notas |
| --- | --- | --- | --- |
| `RedisStorageConfig` | `RedisSecondaryStorageOptions` + conexión separada para rate limit | Parcial | Upstream unifica config; nosotros dos structs de opciones |
| `redisStorage(config)` | `RedisSecondaryStorage::connect` / `::new` | Parcial | Rust: URL o `ConnectionManager`; **sin** `connect_with_options` (fred sí); prefijo custom solo con `::new` |
| `config.client` | `ConnectionManager` vía `connect` | Diseño | Idiomático Rust: pool/manager del crate `redis` |
| `config.keyPrefix` | `key_prefix` en options | Sí | Default distinto (ver [03](./03-secondary-storage.md)) |
| `get` / `set` / `delete` | Trait `SecondaryStorage` | Sí | Firmas async vía trait objects en core |
| `listKeys()` | — | **Gap** | Ver `openauth-fred::FredSecondaryStorage::list_keys` |
| `clear()` | — | **Gap** | Ver `openauth-fred::FredSecondaryStorage::clear` |
| — | `RedisRateLimitStore` | **Extensión** | No existe en paquete upstream |
| — | `RedisRateLimitOptions` | **Extensión** | |
| — | `VERSION` | **Extensión** | Constante de crate |
| — | `normalize_redis_url` (privado) | **Extensión** | `valkey://` / `valkeys://` |

## Crate hermano: `openauth-fred`

| Capacidad | `openauth-redis` | `openauth-fred` |
| --- | --- | --- |
| Secondary storage | Sí | Sí (mismo layout de claves `secondary:`) |
| Rate limit store | Sí (Lua, redis-rs) | Sí (Lua, fred) |
| `list_keys` / `clear` | No | Sí (`SCAN`, prefijo escapado) |
| Cliente | `redis-rs` | `fred` |

**Decisión de empaquetado:** upstream tiene un solo paquete npm; OpenAuth divide por **driver Redis** (2 crates), no por dominio funcional. Ambos crates apuntan al mismo paquete upstream de referencia.

## Re-exports en el workspace

| Crate | Depende de `openauth-redis` |
| --- | --- |
| `openauth` (facade) | No (opt-in del usuario) |
| `examples/full-app` | Sí — `RedisRateLimitStore` |
| `openauth-plugins` | dev-dep — smoke rate limit |
| `openauth-fred` | dev-dep — prueba cruzada secondary storage |

## Archivos de referencia upstream (inventario)

| Archivo | Líneas aprox. | Contenido |
| --- | --- | --- |
| `src/redis-storage.ts` | 75 | Implementación completa |
| `src/index.ts` | 1 | Re-export |
| `README.md` | Corto | Instalación + link docs |
| `CHANGELOG.md` | Solo bumps de deps 1.6.x | Sin cambios de comportamiento documentados |

No hay más módulos, plugins ni rutas HTTP en el paquete upstream. Inventario línea a línea: [09-upstream-file-audit.md](./09-upstream-file-audit.md).
