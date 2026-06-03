# 02 — Mapeo de paquetes y código

## 1:1 npm ↔ crate

| Upstream | OpenAuth |
| --- | --- |
| `packages/scim/` | `crates/openauth-scim/` |
| `@better-auth/scim` | `openauth-scim` |
| Export `.` → `scim()` | `scim(ScimOptions) -> AuthPlugin` |
| Export `./client` → `scimClient()` | **No existe** |

No confundir con otros paquetes del monorepo BA:

| Paquete BA | Relación con SCIM |
| --- | --- |
| `better-auth` (core) | Host del plugin; sesión, adapter, crypto |
| `packages/sso` | Solo en **tests** upstream (`scim.test.ts` monta SSO); SCIM guarda `providerId` como string |
| `organization` plugin | Requerido para tokens/org-scoped y membership en provision |
| `@better-auth/mongo-adapter` | No usado por SCIM directamente; OpenAuth tampoco tiene adapter Mongo para SCIM aún |

## Archivos upstream → Rust

| Upstream `packages/scim/src/` | OpenAuth | Notas |
| --- | --- | --- |
| `index.ts` | `lib.rs` | Plugin id `scim`, version, endpoints, schema |
| `types.ts` | `options.rs`, `store.rs` | + enums `ScimBulkMode`, `ScimDeprovisionMode`, audit |
| `routes.ts` | `routes.rs`, `routes/management.rs`, `routes/users.rs`, `routes/metadata_routes.rs` | BA: un archivo; OA: dividido |
| `middlewares.ts` | `routes/auth_context.rs` | Bearer decode + provider lookup |
| `scim-tokens.ts` | `token.rs`, `auth_context.rs` | Hash SHA-256, encrypt vía `openauth_core::crypto` |
| `mappings.ts` | `mappings.rs` | `accountId`, email, nombre |
| `scim-filters.ts` | `filters.rs` | OA: parser RFC + pushdown |
| `patch-operations.ts` | `patch.rs` | OA: `remove` + perfiles |
| `scim-resources.ts` | `resources.rs` | + Group resource, ETag |
| `scim-error.ts` | `errors.rs` | Envelope RFC 7644 |
| `user-schemas.ts` | `metadata.rs` (schemas), `routes.rs` (input) | OA: Group + Enterprise schema |
| `scim-metadata.ts` | `metadata.rs` (tipos OpenAPI-like) | Config inline en BA `routes.ts` |
| `utils.ts` | `mappings.rs` (`resource_url`) | |
| `version.ts` | `lib.rs` `VERSION` | |
| `client.ts` | — | **TS client-only** |
| `scim.test.ts` | `tests/scim/routes/metadata.rs`, `routes/users.rs`, … | |
| `scim-users.test.ts` | `routes/users.rs`, `isolation.rs`, … | |
| `scim-patch.test.ts` | `patch.rs`, `routes/users.rs` | |
| `scim.management.test.ts` | `routes/management.rs`, `organization.rs`, `token.rs` | |
| — | `routes/groups.rs`, `bulk.rs`, `search.rs`, … | Solo OpenAuth |
| — | `schema.rs`, `validation.rs`, `audit.rs` | Solo OpenAuth |

## Esquema de base de datos

| Modelo | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| `scimProvider` | `providerId` unique, `scimToken` unique, `organizationId?`, `userId?` (si ownership) | Tabla `scim_providers` — mismos campos |
| Perfil SCIM usuario | No (todo en `user` + `account`) | `scim_user_profiles` (JSON extensiones) |
| Perfil SCIM grupo | No (no hay Groups) | `scim_group_profiles` |
| Groups / teams | — | `team` + `team_member` vía plugin organization |

**Decisión:** tablas extra no rompen paridad Users; permiten extensiones enterprise y Groups sin inflar `users`.

### Campos `scim_user_profiles` / `scim_group_profiles` (solo OpenAuth)

| Campo | Uso |
| --- | --- |
| `provider_id` | Vínculo al provider SCIM |
| `user_id` / (`team_id` + `organization_id`) | Recurso OpenAuth subyacente |
| `external_id` | `externalId` SCIM |
| `attributes` | JSON (extensiones enterprise) |
| `version` | Weak ETag |

### Tablas core compartidas

`users`, `accounts`, `member` (org-scoped). Tests upstream montan también `session`, `ssoProvider`; OpenAuth usa `openauth-plugins::organization` en fixtures.

## Dependencias

### Upstream `package.json`

| Tipo | Paquete |
| --- | --- |
| `dependencies` | `zod` |
| `peerDependencies` | `better-auth`, `better-call`, `@better-auth/core`, `@better-auth/utils` |
| `devDependencies` (tests) | `@better-auth/sso` |

Runtime BA importa también: `better-auth/api`, `better-auth/crypto`, `better-auth/plugins` (Member).

### OpenAuth `Cargo.toml`

| Tipo | Crate |
| --- | --- |
| Runtime | `openauth-core`, `serde`, `serde_json`, `base64`, `sha2`, `subtle`, `http`, `time`, `tokio`, `indexmap` |
| Dev (tests) | `openauth-plugins` (organization), `openauth-sqlx`, postgres adapters, `sqlx` |

**No** hay dependencia de crate a `openauth-plugins` en producción: organización se detecta en runtime.

## Equivalencias de stack (plan histórico)

| Upstream | OpenAuth |
| --- | --- |
| Zod body schemas | Serde structs + `validation.rs` |
| `@better-auth/utils` base64/hash | `base64` + `sha2` + `subtle` |
| `symmetricEncrypt` (BA crypto) | `openauth_core::crypto` |
| `sessionMiddleware` | `SessionAuth` + cookie en tests |
| `createAuthMiddleware` (bearer) | `authenticate_scim_request` |
| `internalAdapter.deleteUser` | `DbUserStore` + `ScimDeprovisionMode` |
| Vitest + memory adapter | `#[tokio::test]` + memory/SQL adapters |

## Exports públicos

| Upstream export | OpenAuth |
| --- | --- |
| `scim(options?)` | `scim(ScimOptions)` |
| `SCIMOptions`, `SCIMProvider`, `SCIMName`, `SCIMEmail` | `ScimOptions`, `ScimProviderRecord`, tipos en routes/mappings |
| `scimClient()` | — |
| Registry TS `BetterAuthPluginRegistry` | — |

OpenAuth re-exporta módulos de soporte (`filters`, `patch`, `metadata`, …) para integradores y tests.
