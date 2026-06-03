# 03 — Secondary storage (`SecondaryStorage`)

Contrato upstream: `packages/core/src/db/type.ts` (`SecondaryStorage`).  
Contrato OpenAuth: `crates/openauth-core/src/options/storage.rs`.

## Métodos del contrato

| Método | Upstream `redisStorage` | `RedisSecondaryStorage` | Paridad |
| --- | --- | --- | --- |
| `get(key)` | `GET(prefixKey(key))` | `GET` en `{prefix}secondary:{key}` | Alta |
| `set(key, value, ttl?)` | Ver tabla TTL abajo | Ver tabla TTL abajo | Media-alta |
| `delete(key)` | `DEL(prefixKey(key))` | `DEL` en clave namespaced | Alta |

## Formato de claves

| Aspecto | Upstream | OpenAuth | Motivo |
| --- | --- | --- | --- |
| Prefijo por defecto | `better-auth:` | `openauth:` | Convención del proyecto |
| Prefijo custom | `keyPrefix` en config | `RedisSecondaryStorageOptions.key_prefix` | Igual intención |
| Clave Redis final | `{prefix}{logicalKey}` | `{prefix}secondary:{logicalKey}` | **Decisión OpenAuth:** separar namespace de rate limit (`rate-limit:`) y evitar colisión con claves que upstream mezcla bajo un solo prefijo |
| Ejemplo lógica `session:abc` | `better-auth:session:abc` | `openauth:secondary:session:abc` | Migración desde Better Auth requiere reescritura o prefijo compatible |

`openauth-fred` usa el **mismo** layout `secondary:` que `openauth-redis` (test de cruce en `fred_rate_limit.rs`).

## TTL en `set`

| `ttl` | Upstream `redis-storage.ts` | OpenAuth `RedisSecondaryStorage` | Clasificación |
| --- | --- | --- | --- |
| `undefined` / omitido | `SET` sin expiración | `Option::None` → `SET` | Igual |
| `> 0` (segundos) | `SETEX` | `Some(n)` → `set_ex` | Igual |
| `0` | `SET` sin expiración | `Some(0)` → `SET` sin EX (como upstream) | Igual |
| Negativo (si llegara) | `SET` sin expiración | No hay rama explícita; `Some(n)` con cast u64 en API Rust | N/A en API tipada |

### `ttl = 0` — tres capas

| Capa | Comportamiento |
| --- | --- |
| `redis-storage.ts` | `SET` sin EX |
| Upstream `internal-adapter` | Si `getTTLSeconds` ≤ 0, **no** llama `set` |
| `openauth-redis` | `SET` persistente (test en `redis_secondary_storage_supports_get_set_delete_list_and_clear`) |
| `openauth-fred` | `SET` persistente (**≈ upstream adaptador**) |

Ver [11-gap-closure-status.md](./11-gap-closure-status.md) y [08-logical-keys-and-payloads.md](./08-logical-keys-and-payloads.md).

**TTL desde timestamps:** el adaptador Redis **no** calcula TTL; lo hace el consumidor (sesión, verificación) como en upstream. Upstream usa `Math.floor` en core al derivar segundos desde `expiresAt`; eso vive en `openauth-core` / `session.rs`, no en este crate.

## Tipo de valor

| | Upstream interface | OpenAuth |
| --- | --- | --- |
| `get` retorno en interfaz core | `Awaitable<unknown>` (permite objeto ya parseado) | `Option<String>` siempre |
| Serialización JSON | Responsabilidad del consumidor (`JSON.stringify` en rate limit, sesiones en internal-adapter) | Igual — solo strings en Redis |

Upstream documenta y prueba clientes Redis que devuelven JSON ya parseado (`safeJSONParse` + tests “pre-parsed storage” en `internal-adapter.test.ts`). **OpenAuth no soporta** `get` devolviendo tipos distintos de `String` en el trait; es limitación idiomática Rust + contrato estricto, no del driver Redis.

## Utilidades fuera del trait

| Método | Upstream | `openauth-redis` | `openauth-fred` |
| --- | --- | --- | --- |
| `listKeys()` | `KEYS ${prefix}*`, strip prefix | `list_keys()` con `SCAN` | `list_keys()` con `SCAN`, escape de glob en prefijo |
| `clear()` | `KEYS` + `DEL(...keys)` | `clear()` vía `list_keys` + `DEL` | `clear()` vía `list_keys` + `DEL` |

**Nota:** ambos crates OpenAuth usan `SCAN` (más seguro que `KEYS` del upstream). Estado: [11-gap-closure-status.md](./11-gap-closure-status.md).

**Riesgo upstream `clear()`:** `client.del(...keys)` con array vacío — OpenAuth-fred evita `DEL` vacío explícitamente.

## Configuración del cliente

| Patrón | Upstream | OpenAuth |
| --- | --- | --- |
| Conexión | App crea `new Redis({ host, port, tls, cluster, ... })` y pasa `client` | `RedisSecondaryStorage::connect(url)` crea `Client` + `ConnectionManager` |
| Reutilizar pool | Misma instancia ioredis | `RedisSecondaryStorage::new(manager, options)` |
| Cluster / Sentinel | vía ioredis | vía URL / config `redis-rs` (no documentado en crate) |

**Decisión:** Rust expone URL/manager en lugar de factory TS `redisStorage({ client })` — equivalente operativo, distinta forma.

## Tabla resumen de gaps secondary storage

| Gap | Severidad | Acción sugerida |
| --- | --- | --- |
| Sin `list_keys` / `clear` | Baja (dev/test) | Portar patrón de `openauth-fred` o documentar usar fred |
| Namespace `secondary:` vs prefijo plano | Media (migración) | Documentar mapping; opcional compat flag |
| `ttl = 0` borra vs `SET` | Media (compat) | Documentar; evaluar alinear con upstream si se necesita migración drop-in |
| `get` solo `String` | Baja | Cubierto en core; Redis auto-parse no aplica en redis-rs |
