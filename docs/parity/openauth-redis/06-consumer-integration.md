# 06 — Integración: quién consume secondary storage

El paquete `@better-auth/redis-storage` **no** implementa sesiones ni plugins; solo KV. Esta sección describe **consumidores upstream** vs **OpenAuth** para saber qué debe funcionar cuando se cablea `RedisSecondaryStorage`.

## Claves lógicas (lo que el adaptador Redis no decide)

Antes de prefijo Redis, Better Auth y OpenAuth **no** usan las mismas claves para sesiones. Tabla completa: [08-logical-keys-and-payloads.md](./08-logical-keys-and-payloads.md).

Resumen:

- Upstream: clave = **token**, índice = `active-sessions-{userId}`, valor sesión = `{ session, user }`.
- OpenAuth: clave = `session:{token}`, índice = `session:user:{userId}`, valor = solo `Session` JSON.

Verificación en tests:

- Upstream smoke: `redis.spec.ts` líneas 96–104 (`listKeys`, `JSON.parse` con `.user` y `.session`).
- OpenAuth: `sign_up_email_route_uses_secondary_storage_for_sessions` en `openauth-core/tests/api/routes/sign_up_email.rs` líneas 122–123.

## Upstream — consumidores de `secondaryStorage`

| Consumidor | Ubicación aproximada | Uso de Redis |
| --- | --- | --- |
| Sesiones (token → payload) | `better-auth` `internal-adapter.ts` | `set`/`get`/`delete` por session token; lista `active-sessions-{userId}` |
| Rate limit default | `api/rate-limiter/index.ts` | JSON en mismas claves si `storage === "secondary-storage"` |
| Verificación OTP/email | `internal-adapter` + verification helpers | Claves `verification:{id}` con TTL desde `expiresAt` |
| API keys (opcional) | `packages/api-key` | `storage: "secondary-storage"` o fallback; claves `api-key:*` |
| Device authorization | `plugins/device-authorization` | Estado temporal con TTL |
| SSO | `packages/sso` | Estado OIDC/SAML, domain verification cuando no está en DB |
| OAuth provider | validaciones con secondary + DB | Requiere `storeSessionInDatabase` en algunos modos |
| Schema / migraciones | `get-tables.ts` | Omite tablas session/verification si solo secondary |

**Pruebas relevantes (sin Redis real en paquete redis):**

- `packages/better-auth/src/db/secondary-storage.test.ts` — flujo sign-in/list/revoke con mock Map.
- `packages/better-auth/src/db/internal-adapter.test.ts` — TTL, verification, JSON pre-parseado.
- `packages/better-auth/src/api/rate-limiter/rate-limiter.test.ts` — custom + secondary-storage adapter (mock).
- `e2e/smoke/test/redis.spec.ts` — **único** test que importa `@better-auth/redis-storage`.

## OpenAuth — consumidores de `SecondaryStorage`

| Consumidor | Crate / módulo | Notas paridad |
| --- | --- | --- |
| Sesiones | `openauth-core/src/session.rs` | Claves **distintas** a upstream (`session:{token}`, `session:user:{id}`); ver [08](./08-logical-keys-and-payloads.md) |
| Verificación | `openauth-core/src/verification.rs` | TTL y delete alineados con tests core |
| Email / password flows | `auth/email_password.rs`, tests `sign_up_email`, `request_password_reset` | Mocks `TestSecondaryStorage`, no Redis |
| Schema | `openauth-core/src/db/schema.rs` | Tests de omisión de tablas con secondary |
| Plugins (API key, SSO, device, …) | Varios crates | Deben usar el mismo trait; Redis es opt-in |
| Rate limit | **No** usa `RedisSecondaryStorage` por defecto | Usa `RedisRateLimitStore` — ver [04](./04-rate-limiting.md) |

## Matriz: ¿lo cubre `openauth-redis` solo?

| Escenario upstream | ¿Necesita solo adaptador KV? | Dónde se valida en OpenAuth |
| --- | --- | --- |
| Sign-up email → 2 claves en Redis | Sí | `openauth-fred` e2e-style tests; smoke upstream |
| `storeSessionInDatabase: true` + Redis | Sí | `openauth-fred` |
| Revoke session borra Redis | Sí | core tests + fred |
| Rate limit en Redis | **No** (upstream usa KV) | `RedisRateLimitStore` + core rate_limit tests |
| `listKeys` en smoke test | Utilidad adaptador | **fred**, no `openauth-redis` |
| Stateless OAuth + Redis | Core OAuth | Fuera del crate redis |
| API key secondary | Plugin | `openauth-api-key` tests (mock), no redis crate |

## Cableado recomendado (OpenAuth)

Equivalente funcional a upstream “sesiones + rate limit en Redis”:

```rust
// Dos handles sobre la misma URL (o dos ConnectionManager del mismo pool)
let rl = RedisRateLimitStore::connect(redis_url).await?;
let sec = RedisSecondaryStorage::connect(redis_url).await?;
// Cada connect() abre su propio ConnectionManager (dos pools). Para uno solo:
// let manager = ConnectionManager::new(client).await?;
// let rl = RedisRateLimitStore::new(manager.clone(), rl_opts);
// let sec = RedisSecondaryStorage::new(manager, sec_opts);

OpenAuth::builder()
    .options(
        OpenAuthOptions::new()
            .secret("...")
            .secondary_storage(Arc::new(sec)),
    )
    .rate_limit(
        RateLimitOptions::secondary_storage(rl)
            .enabled(true)
            .window(10)
            .max(100),
    )
    .build()?;
```

Upstream equivalente (una sola instancia):

```ts
const redis = new Redis(process.env.REDIS_URL);
betterAuth({
  secondaryStorage: redisStorage({ client: redis }),
  // rateLimit.storage defaults to "secondary-storage"
});
```

## Fuera de alcance (TS-only / client-only)

| Capacidad | Motivo exclusión |
| --- | --- |
| Cookie cache en navegador | Cliente |
| MSW / Node test harness de smoke | Entorno TS |
| `safeJSONParse` con objetos ya parseados en trait público | TS `unknown` vs Rust `String` |
| Empaquetado dual ESM/CJS | npm |
