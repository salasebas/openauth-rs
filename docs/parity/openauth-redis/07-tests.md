# 07 — Tests: inventario desde archivos (no README)

Conteos obtenidos leyendo fuentes en pin **1.6.9** y el workspace OpenAuth.

## Resumen numérico

| Ubicación | Archivos | Casos | Redis/Valkey real |
| --- | --- | --- | --- |
| `packages/redis-storage` | **0** tests | **0** | — |
| `e2e/smoke/test/redis.spec.ts` | 1 | **4** `t.test` | Sí (`flushall`, `redis://localhost:6379`) |
| `better-auth/.../secondary-storage.test.ts` | 1 | **4** `it` | No (Map mock) |
| `better-auth/.../internal-adapter.test.ts` | 1 | **33** `it` total; **~14** tocan secondary/verification/TTL/pre-parse | No (SQLite + mock storage) |
| `better-auth/.../rate-limiter.test.ts` | 1 | **24** `it`; **1** usa mock `secondaryStorage` + JSON RL | No |
| `openauth-redis` unit (`src/lib.rs`) | 1 módulo | **5** (4 + 1 `cfg` TLS) | No |
| `openauth-redis` integration | `tests/redis_rate_limit.rs` | **8** (4 sync env + 4 `tokio`) | Sí si Redis 6379 + Valkey 6380 |
| `openauth-plugins` | `integration_matrix/mod.rs` | **1** `#[ignore]` smoke RL | Opcional docker |
| `openauth-fred` | `tests/*.rs` | **~22** funciones test | Sí (mayoría live) |
| `openauth-core` | mocks secondary | **3** flujos HTTP + **2** schema | No Redis |

**Total funciones test en crate `openauth-redis`:** **13** (12 sin feature TLS).

## Upstream: `packages/redis-storage`

- `package.json` declara `"test": "vitest"` pero **no existe** `*.test.ts` ni `*.spec.ts` bajo el paquete.
- Toda la cobertura del adaptador npm está **fuera** del paquete.

## Upstream: `e2e/smoke/test/redis.spec.ts` (4 casos)

| Test | Qué ejercita del adaptador | Cubierto en OpenAuth |
| --- | --- | --- |
| Email signup → Redis | `listKeys()`, 2 claves, JSON `{session,user}` | **`openauth-fred`** signup tests (layout OpenAuth, no BA) |
| `storeSessionInDatabase: true` | Igual + DB | **`openauth-fred`** `..._with_database_sessions_...` |
| Stateless + Google OAuth | Redis + JWE + MSW | **No** en crates redis/fred |
| Custom Google `authorizationEndpoint` | Casi solo OAuth URL | **No** |

Dependencias del test: `@better-auth/redis-storage`, `ioredis`, `better-auth`, `msw`, SQLite migrations.

## Upstream: `secondary-storage.test.ts` (4 `it`)

| Test | Assert |
| --- | --- |
| string return E2E | sign-in → 2 keys → getSession → list → revoke → store vacío |
| object return E2E | `get` devuelve objeto parseado; `active-sessions-{userId}` array |
| preserveSession false + DB | revoke borra token del Map |
| preserveSession true + DB | revoke borra secondary aunque quede en DB |

OpenAuth equivalente (mock, **sin Redis**):

- `openauth-core/tests/api/routes/sign_up_email.rs` — `sign_up_email_route_uses_secondary_storage_for_sessions` (claves `session:{token}`, `session:user:{id}`).
- No hay test de `get` devolviendo objeto pre-parseado (Rust `String` only).

## Upstream: `internal-adapter.test.ts` (secondary-related)

| Tema | Tests relevantes (grep en fuente) |
| --- | --- |
| TTL `Math.floor` secondary | `should calculate TTL correctly with Math.floor for secondary storage` |
| Session CRUD secondary | create / delete / update + `active-sessions` list |
| Verification secondary | store, find, delete, fallback DB, TTL expiresAt |
| Pre-parsed Redis objects | 4× `safeJSONParse date revival` |

OpenAuth: lógica repartida en `verification.rs` / `session.rs` + tests unitarios de core; **no** replica bloque pre-parseado.

## Upstream: `rate-limiter.test.ts`

| Test | Storage | Clave assert |
| --- | --- | --- |
| `should use custom storage` | Mock `secondaryStorage` Map | `127.0.0.1\|/sign-in/email` JSON `RateLimit` |

OpenAuth: `openauth-core/tests/rate_limit/rate_limiter.rs` (**24** `#[tokio::test]` / tests) con `GovernorMemoryRateLimitStore` y mocks — **no** importa `openauth-redis`.

## `openauth-redis` — detalle por archivo

### `src/lib.rs` (unit)

| Test | Archivo:línea aprox. |
| --- | --- |
| `normalizes_valkey_urls_to_redis_urls` | Valkey aliases |
| `leaves_non_valkey_urls_unchanged` | redis/rediss/unix |
| `rate_limit_script_uses_current_hash_set_command` | Lua HSET |
| `secondary_storage_uses_separate_key_namespace` | `test:secondary:...` |
| `tls_urls_open_as_tls_connections` | cfg `rustls` \| `native-tls` |

### `tests/redis_rate_limit.rs` (integration)

| Test | Redis live |
| --- | --- |
| `redis_targets_*` (×4) | No — solo lógica env |
| `redis_rate_limit_store_enforces_atomic_max_one` | Sí |
| `redis_rate_limit_store_allows_exactly_one_concurrent_request` | Sí |
| `redis_secondary_storage_supports_get_set_delete_list_and_clear` | Sí — `list_keys`, `clear`, `Some(0)` persistente |
| `redis_rate_limit_store_resets_after_window` | Sí |
| `redis_rate_limit_store_does_not_reset_at_exact_window_boundary` | Sí |
| `redis_open_auth_stores_share_one_connection_manager` | Sí — bundle |

**Solo en `openauth-fred`** (E2E producto / cruces):

| Escenario | Test fred (referencia) |
| --- | --- |
| Handler + Fred RL | `openauth_handler_async_uses_fred_rate_limit_store` |
| Email signup + secondary | `openauth_email_signup_uses_fred_secondary_storage_for_sessions` |
| Password reset verification | `openauth_password_reset_uses_fred_secondary_storage_for_verification` |
| Cruce redis-rs ↔ fred | `fred_and_redis_secondary_storage_share_physical_key_layout` |
| Prefijo glob / clear isolation | `fred_secondary_storage_clear_keeps_other_prefixes`, `..._glob_metacharacters_...` |

### `openauth-plugins/tests/integration_matrix/mod.rs`

- `docker_redis_and_valkey_rate_limit_store_are_atomic` — `#[ignore]`, duplica atomicidad max=1 en Redis/Valkey con env `OPENAUTH_TEST_REDIS_URL` / `OPENAUTH_VALKEY_URL`.

### `examples/full-app`

- Usa `RedisRateLimitStore::connect` en `shared_redis_rate_limit` — **no** es test automatizado.

## Gaps restantes (prioridad)

| Gap | Severidad |
| --- | --- |
| Sin E2E sign-up/session en crate `openauth-redis` | Baja — cubierto en fred |
| Sin test compat claves Better Auth | Alta **solo** si producto pide migración de datos |
| Sin test rate limit JSON en secondary KV | N/A por diseño |
| Prefijo glob / clear isolation | Baja — solo tests en fred |

Cerrados (ver [11-gap-closure-status.md](./11-gap-closure-status.md)): `list_keys`/`clear`, `ttl=0`, ventana RL `>`, `connect_with_options`, prefijo vacío.

## Comandos

```bash
cargo nextest run -p openauth-redis
OPENAUTH_REDIS_URL=redis://127.0.0.1:6379 OPENAUTH_VALKEY_URL=valkey://127.0.0.1:6380 cargo nextest run -p openauth-redis
cargo nextest run -p openauth-fred
cargo nextest run -p openauth-core --test rate_limit
```

CI: `.github/workflows/ci.yml` — job `package: openauth-redis` con servicios redis + valkey (`docker-compose.yml`).
