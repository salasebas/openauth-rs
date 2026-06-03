# 09 — Auditoría línea a línea del paquete upstream

Inventario exhaustivo de `reference/upstream-src/1.6.9/repository/packages/redis-storage/` (2026-06-01).

## Archivos versionados

| Archivo | Líneas | Rol |
| --- | --- | --- |
| `src/redis-storage.ts` | 75 | Implementación única |
| `src/index.ts` | 1 | `export { RedisStorageConfig, redisStorage }` |
| `package.json` | 56 | Peer: `@better-auth/core`, `ioredis ^5` |
| `tsconfig.json` | — | Typecheck |
| `README.md` | 17 | Install + link docs |
| `CHANGELOG.md` | 79 | Solo bumps `@better-auth/core` 1.6.0-beta → 1.6.9 |

**No hay:** `*.test.ts`, `*.spec.ts`, `vitest.config`, `dist/` en el clone fuente (build artifact).

## `redisStorage()` — comportamiento exacto

```37:74:reference/upstream-src/1.6.9/repository/packages/redis-storage/src/redis-storage.ts
export function redisStorage(config: RedisStorageConfig) {
	const { client, keyPrefix = "better-auth:" } = config;
	// get → client.get(`${keyPrefix}${key}`)
	// set → ttl !== undefined && ttl > 0 ? setex : set
	// delete → del prefixed key
	// listKeys → KEYS `${keyPrefix}*` then map replace(keyPrefix, "")
	// clear → KEYS then del(...keys)
}
```

### Detalles fáciles de pasar por alto

| Detalle | Comportamiento |
| --- | --- |
| `ttl` omitido | `SET` sin EX |
| `ttl === 0` | Rama `else` → `SET` sin EX (**no** borra) |
| `ttl < 0` | Igual que 0 (rama else) |
| `listKeys` strip | `String.prototype.replace` — solo **primera** coincidencia del prefijo |
| `clear` sin claves | `del()` sin argumentos (comportamiento depende de ioredis) |
| `get` retorno | Lo que devuelve ioredis (`null` si miss) |
| Cluster/Sentinel | Documentado en `docs/.../database.mdx`, no en código del paquete |

## Referencias upstream fuera del paquete (grep en monorepo)

| Archivo | Uso |
| --- | --- |
| `e2e/smoke/test/redis.spec.ts` | Único import de `@better-auth/redis-storage` en tests |
| `e2e/smoke/package.json` | devDep del paquete + ioredis |
| `docs/content/docs/concepts/database.mdx` | Instalación + ejemplo `redisStorage` + ejemplo manual `redis` npm |
| `docs/content/blogs/1-5.mdx` | Extracción del paquete desde core |
| `.changeset/config.json` | Paquete en release set |

**No** aparece en `packages/better-auth/package.json` como dependencia runtime (opt-in del usuario).

## Documentación manual vs paquete oficial

`database.mdx` incluye implementación manual con cliente `redis` (npm) **sin** prefijo en `get(key)` — el paquete oficial **sí** prefija. OpenAuth documenta prefijo + segmento `secondary:`.

## Scripts npm del paquete

| Script | Propósito |
| --- | --- |
| `build` | tsdown |
| `test` | vitest (**sin archivos de test en el paquete**) |
| `lint:package` / `lint:types` | publint, attw |
