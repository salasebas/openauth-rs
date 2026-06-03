# 04 — Funcionalidades y opciones

## Tabla maestra de funcionalidades

| Funcionalidad | Better Auth 1.6.9 | OpenAuth | Paridad | Motivo si difiere |
| --- | --- | --- | --- | --- |
| Management: generate/list/get/delete provider | ✅ | ✅ | Igual | — |
| Provider ownership (`userId` en fila) | ✅ opt-in | ✅ opt-in | Igual | — |
| `requiredRole` + roles por defecto | ✅ | ✅ | Igual | — |
| Roles comma-separated (`admin,member`) | ✅ | ✅ | Igual | Tests en `organization.rs` |
| Hooks before/after token | ✅ | ✅ | Igual | — |
| `defaultSCIM` (sin DB) | ✅ | ✅ | Igual | Tests / estáticos |
| Token storage plain/hash/encrypt/custom | ✅ | ✅ | Igual | **Default:** plain vs hashed |
| Token rotation | delete + create | upsert misma fila | Divergente | Menos carreras (decisión OA) |
| Users: create link-by-email | ✅ | ✅ | Igual | — |
| Users: org auto-member `member` | ✅ | ✅ | Igual | Requiere plugin organization |
| Users: list filter DB | `userName eq` only | `userName eq` → SQL | Igual | — |
| Users: list filter avanzado | ❌ | ✅ in-memory | Superset | IdP / enterprise attrs |
| Users: paginación `startIndex`/`count` | ❌ (siempre full list) | ✅ cap 200 | Superset | Directorios grandes |
| Users: sort / projection | ❌ | ✅ | Superset | — |
| Users: weak ETag / If-Match | ❌ | ✅ | Superset | Concurrencia |
| Users: PATCH remove | Ignorado | ✅ | Divergente | RFC + `externalId` reset |
| Users: `userName` debe ser email | ❌ (`the-username` OK en tests) | ✅ `validation.rs` | Divergente | [§07](./07-edge-cases-and-integrators.md) |
| Users: `email_verified` en create | No explícito | `true` | Divergente | Cuentas provisionadas |
| Users: un solo `emails[].primary` | Sin límite | 400 si >1 | Divergente menor | — |
| Users: DELETE | `deleteUser` global | `DeleteUser` o `UnlinkAccount` | Igual / extensión | Modo configurable OA |
| Groups SCIM | ❌ | ✅ → `team` | Superset | Requiere org-scoped provider |
| Bulk SCIM | ❌ (metadata false) | ✅ Independent/Atomic | Superset | — |
| POST `.search` | ❌ | ✅ | Superset | — |
| GET `/Me` | ❌ (no ruta) | 501 | Explícito | Tokens de proveedor ≠ usuario |
| Schemas metadata | User only | User + Group + Enterprise | Superset | — |
| `active` en recurso | siempre `true` | configurable vía perfil | Parcial | — |
| Cliente TS `scimClient()` | ✅ | ❌ | N/A | Server-only |
| OpenAPI / HIDE_METADATA | ✅ | Parcial vía `AuthEndpointOptions` | Parcial | No bloquea IdPs |
| Audit estructurado | ❌ | ✅ opt-in | Superset | Operabilidad |
| Tests SQL multi-adapter | helper no usado | ✅ 12 tests | Superset | `db_adapters.rs` |
| `User.groups` en recurso User | Siempre `[]` | Poblado si hay grupos SCIM en org | Superset | IdP membership |
| Perfiles SCIM (`scim_user_profiles`) | No | Atributos extension + ETag | Superset | Enterprise schema |
| Hook `before` en regenerate fallido | Provider ya borrado | Provider preservado | Divergente | Mejora operacional |
| Validación multivalued `primary` | No | `phoneNumbers`, `roles`, etc. | Superset | §07 |

## Opciones de configuración

### Better Auth `SCIMOptions`

| Campo | Default | OpenAuth equivalente |
| --- | --- | --- |
| `providerOwnership.enabled` | false | `provider_ownership.enabled` |
| `requiredRole` | admin + creatorRole | `required_role` |
| `defaultSCIM` | — | `default_scim` |
| `beforeSCIMTokenGenerated` | — | `before_token_generated` |
| `afterSCIMTokenGenerated` | — | `after_token_generated` |
| `storeSCIMToken` | **`"plain"`** | **`ScimTokenStorage::Hashed`** |

### Solo OpenAuth `ScimOptions`

| Campo | Default | Propósito |
| --- | --- | --- |
| `bulk_mode` | `Independent` | `Atomic` = una transacción por request Bulk |
| `deprovision_mode` | `DeleteUser` | `UnlinkAccount` = solo cuenta del provider |
| `audit_event` | `None` | Callback + logs estructurados |

## Filtros

### Upstream (`scim-filters.ts`)

- Regex simple: `attribute op value`
- Operador soportado en DB: **`eq`**
- Atributo soportado: **`userName`** → campo `email`
- Otros ops en regex (`ne`, `co`, …) → error `invalidFilter`

### OpenAuth (`filters.rs`)

| Ruta | Comportamiento |
| --- | --- |
| `GET /Users?filter=userName eq "x"` | `list_user_filter_uses_database_pushdown` → SQL en `users.email` |
| Cualquier otro filter en Users | `parse_filter` + evaluación en memoria sobre JSON SCIM |
| Groups / `.search` | Siempre en memoria |
| Sintaxis inválida | 400 `scimType: invalidFilter` |

## PATCH Users

### Paths soportados (ambos)

| Path | Efecto |
| --- | --- |
| `/name/formatted` | `user.name` |
| `/name/givenName` | recomponer nombre |
| `/name/familyName` | recomponer nombre |
| `/externalId` | `account.account_id` |
| `/userName` | `user.email` (lowercase) |

### Semántica

| Comportamiento | Better Auth | OpenAuth |
| --- | --- | --- |
| `add` / `replace` | ✅ | ✅ |
| `remove` | **skip** | ✅ (p. ej. quitar `externalId`) |
| Dot notation / nested value | ✅ | ✅ |
| `add` idempotente si valor igual | skip update | similar |
| Sin campos válidos | 400 | 400 (puede ser más estricto en no-op) |
| Case-insensitive `op` | ✅ | ✅ |

## Provisioning y aislamiento

| Regla | Ambos |
| --- | --- |
| `accountId` = `externalId` ?? `userName` | ✅ |
| Duplicate account mismo `providerId` | 409 uniqueness |
| List/get/put/patch/delete scoped por provider | ✅ |
| Org-scoped: solo miembros de la org | ✅ |
| **Groups visibles entre providers de la misma org** | N/A upstream | Teams son org-scoped, no por provider (documentado) |

## DELETE usuario — semántica crítica

Al eliminar un usuario SCIM vinculado por email existente:

- **Better Auth:** `internalAdapter.deleteUser` — borra el usuario OpenAuth completo.
- **OpenAuth default:** igual (`ScimDeprovisionMode::DeleteUser`).
- **OpenAuth opcional:** `UnlinkAccount` — solo cuenta + perfil SCIM del provider.

**No es bug de paridad:** es el mismo comportamiento “duro” que upstream por defecto; OA añade modo suave.

## Integración organization

| Caso | Sin plugin `organization` | Con plugin |
| --- | --- | --- |
| Token con `organizationId` en management | Error | Membership + roles |
| User provision org-scoped | — | Crea `member` si falta |
| Groups | — | Requerido (400 sin org en provider) |

Upstream tests montan `organization()` + `sso()`; OpenAuth tests usan `openauth_plugins::organization`.
