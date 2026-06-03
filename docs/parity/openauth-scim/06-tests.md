# 06 — Tests y cobertura

## Conteos

| Fuente | Métrica | Valor |
| --- | --- | --- |
| Upstream | `it(` en `*.test.ts` | **75** |
| Upstream | `it.for(` | **6** (≈12 ejecuciones extra) |
| Upstream | **Total Vitest ≈** | **~87** |
| OpenAuth | `#[test]` | **40** |
| OpenAuth | `#[tokio::test]` | **149** |
| OpenAuth | **Total ≈** | **189** |

Harness Rust: `crates/openauth-scim/tests/scim.rs` (opciones con `Plain` tokens para seeds).

## Archivos upstream

| Archivo | `it(` | Área |
| --- | --- | --- |
| `scim.test.ts` | 19 | Metadata (7) + POST/PUT Users (12) |
| `scim.management.test.ts` | 33 | Token, list/get/delete provider, RBAC |
| `scim-users.test.ts` | 16 | List/get/delete Users, defaultSCIM |
| `scim-patch.test.ts` | 7 + 6×`it.for` | PATCH Users |

## Archivos OpenAuth por área

| Área | Archivos | `#[test]` | `#[tokio::test]` |
| --- | --- | --- | --- |
| Unit: filters | `filters.rs` | 11 | 0 |
| Unit: metadata/schema | `metadata.rs`, `schema.rs`, `metadata_snapshot.rs` | 14 | 0 |
| Unit: patch/mappings/token/validation/errors/resources | varios | 16 | 0 |
| Store | `store.rs` | 0 | 3 |
| DB adapters | `db_adapters.rs` | 0 | 12 |
| Users + auth + isolation | `routes/users.rs`, `auth.rs`, `isolation.rs`, `provisioning.rs`, `concurrency.rs`, `deprovision.rs` | 0 | 55 |
| Management + org + parity gaps | `management.rs`, `organization.rs`, `parity_gaps.rs` | 0 | 33 |
| Groups | `groups.rs`, `groups_auth.rs`, `groups_scope.rs`, `groups_native_team_boundary.rs` | 0 | 14 |
| Bulk + search | `bulk.rs`, `bulk_atomic.rs`, `search.rs` | 0 | 25 |
| Metadata routes | `routes/metadata.rs` | 0 | 4 |
| Audit | `audit.rs` | 0 | 1 |

## Matriz: upstream → OpenAuth

Leyenda: ✅ cubierto · ➖ N/A (server-only) · ➕ extra OpenAuth

### `scim.test.ts`

| Escenario upstream | OpenAuth | Estado |
| --- | --- | --- |
| ServiceProviderConfig | `routes/metadata.rs`, `metadata.rs` | ✅ (flags distintos assertados en metadata) |
| Schemas list / single / 404 | `routes/metadata.rs`, `metadata_snapshot.rs` | ✅ (+ Group/Enterprise) |
| ResourceTypes list / single / 404 | idem | ✅ (+ Group) |
| POST create user (variantes nombre/email) | `routes/users.rs` | ✅ |
| POST link existing user | `routes/provisioning.rs` | ✅ |
| POST duplicate → 409 | `routes/users.rs` | ✅ |
| POST anonymous → 401 | `routes/auth.rs` | ✅ |
| PUT update / 401 / 404 | `routes/users.rs` | ✅ |

### `scim-users.test.ts`

| Escenario | OpenAuth | Estado |
| --- | --- | --- |
| GET list users | `routes/users.rs` | ✅ |
| GET empty list | `routes/users.rs` | ✅ |
| Provider isolation list | `routes/users.rs`, `isolation.rs` | ✅ |
| Org isolation list | `routes/users.rs`, `organization.rs` | ✅ |
| Filter `userName eq` | `filters.rs`, `routes/users.rs` | ✅ |
| GET single / isolation / 404 | `routes/users.rs`, `isolation.rs` | ✅ |
| DELETE user / 401 / 404 | `routes/users.rs` | ✅ |
| defaultSCIM provider | `routes/users.rs`, `token.rs` | ✅ |
| Invalid default token | `token.rs` | ✅ |

### `scim-patch.test.ts`

| Escenario | OpenAuth | Estado |
| --- | --- | --- |
| replace / add (`it.for`) | `patch.rs`, `routes/users.rs` | ✅ |
| mixed ops | `routes/users.rs` | ✅ |
| name sub-attributes | `patch.rs` | ✅ |
| nested path prefix | `patch.rs` | ✅ |
| no explicit path | `patch.rs` | ✅ |
| dot notation | `patch.rs` | ✅ |
| case insensitive op | `patch.rs` | ✅ |
| skip add if exists | `patch.rs` | ✅ |
| ignore bad path | `patch.rs` | ✅ |
| invalid op | `patch.rs` | ✅ |
| 404 / 400 empty / 401 | `routes/users.rs` | ✅ |
| **remove** operations | `patch.rs`, `routes/users.rs` | ➕ extra |

### `scim.management.test.ts`

| Escenario | OpenAuth | Estado |
| --- | --- | --- |
| Session required | `management.rs` | ✅ |
| Org membership deny | `organization.rs` | ✅ |
| Invalid providerId `:` | `management.rs` | ✅ |
| generate via **client** | — | ➖ N/A |
| plain/hashed/encrypted/custom storage | `management.rs`, `token.rs` | ✅ |
| org-scoped token | `organization.rs` | ✅ |
| before/after hooks | `management.rs` | ✅ |
| ownership regenerate deny | `management.rs` | ✅ |
| cross-org deny | `management.rs` | ✅ |
| list empty / org / owned | `management.rs` | ✅ |
| get provider (varios) | `management.rs` | ✅ |
| delete provider + invalidate | `management.rs` | ✅ |
| member cannot generate | `organization.rs` | ✅ |
| admin can generate | `organization.rs` | ✅ |
| multiple roles `admin,member` | `organization.rs` | ✅ |
| custom requiredRole | `organization.rs` | ✅ |
| custom creator role | `organization.rs` | ✅ |
| list filtered by role | `organization.rs` | ✅ |

## Cobertura solo OpenAuth (no hay test upstream equivalente)

| Módulo | Qué valida |
| --- | --- |
| `routes/groups.rs` | CRUD Groups, members, displayName |
| `routes/groups_auth.rs` | Bearer en Groups |
| `routes/groups_scope.rs` | Provider sin org → 400 |
| `routes/groups_native_team_boundary.rs` | Teams nativos vs SCIM |
| `routes/bulk.rs` | Bulk POST/PUT/PATCH/DELETE, failOnErrors, bulkId |
| `routes/bulk_atomic.rs` | Rollback transaccional |
| `routes/search.rs` | POST `.search` |
| `routes/concurrency.rs` | ETag, `If-Match: *` |
| `routes/isolation.rs` | Item routes provider/org, duplicate externalId |
| `routes/deprovision.rs` | `UnlinkAccount` |
| `routes/audit.rs` | Audit resolver |
| `db_adapters.rs` | SQLite, Postgres, MySQL migrations + provider SQL |
| `validation.rs` | Email en create/put/patch/bulk |
| `filters.rs` | Parser extendido + pushdown helper |
| `metadata_snapshot.rs` | Drift CI de schemas |

## Gaps / no objetivo

| Item | Estado |
| --- | --- |
| Test `authClient.scim.generateToken` | ➖ N/A server-only |
| `_createSqlTestInstance` en upstream (unused) | Reemplazado por `db_adapters.rs` |
| MongoDB adapter SCIM | Pendiente adapter OpenAuth |
| Redis/Valkey como store SCIM | No — solo rate-limit en workspace |

## Escenarios upstream sin test Rust equivalente (segunda auditoría — cerrados)

Todos los escenarios de esta tabla tienen regresión Rust salvo los marcados como N/A o no objetivo.

| Escenario | Upstream | OpenAuth | Notas |
| --- | --- | --- | --- |
| Create `userName: "the-username"` (no email) | `scim.test.ts` éxito | `parity_gaps` → 400 `invalidValue` | Divergencia documentada en §07 |
| Create solo `givenName`/`familyName` | `scim.test.ts` | `parity_gaps::users_create_with_given_and_family_name_parts` | ✅ |
| Create primer email no-primary | `scim.test.ts` | `parity_gaps::users_create_with_primary_email_in_emails_array` | ✅ |
| PATCH op inválido `update` | Zod `VALIDATION_ERROR` | `parity_gaps::users_patch_rejects_invalid_update_operation_with_scim_invalid_syntax` | Divergencia intencional (SCIM vs Zod) |
| Regenerate token, distinto `organizationId` | Implícito unique `providerId` | `parity_gaps::management_regenerate_rejects_different_organization_scope` | ✅ |
| `get-provider-connection` sin `providerId` | Implícito | `parity_gaps::management_get_provider_connection_requires_provider_id` | ✅ |
| `requiredRole: []` permite cualquier miembro | Implícito | `parity_gaps::management_empty_required_role_allows_any_org_member` | ✅ |
| DELETE sin `Content-Type` | Implícito | `parity_gaps::users_delete_succeeds_without_content_type_header` | ✅ |
| Filtro `userName eq ""` | Implícito | `filters::rejects_empty_user_name_eq_filter_value` | ✅ |
| List filter debug log | `logger.info` | No | Operacional |

## Cobertura OpenAuth que upstream no tiene (recordatorio)

`routes/isolation.rs`, `routes/bulk.rs` (22 tests), `routes/groups*.rs`, `db_adapters.rs` (12), `metadata_snapshot.rs`, `routes/deprovision.rs`, `routes/concurrency.rs`, `routes/audit.rs`, `routes/auth.rs` (org plugin guard en bearer).

### `routes/parity_gaps.rs` (regresión documentada)

| Test | Qué cubre |
| --- | --- |
| `management_regenerate_rejects_different_organization_scope` | 403 al regenerar con `organizationId` distinto del scope del provider |
| `management_get_provider_connection_requires_provider_id` | 400 si falta `providerId` |
| `management_empty_required_role_allows_any_org_member` | `requiredRole: []` permite miembro no-admin |
| `users_create_with_given_and_family_name_parts` | `displayName` / `name.formatted` desde partes |
| `users_create_with_primary_email_in_emails_array` | `userName` desde email `primary: true` |
| `users_create_with_external_id_uses_external_id_as_account_id` | `externalId` distinto de email |
| `users_create_sets_email_verified_on_new_user` | `emailVerified` en usuario nuevo |
| `users_create_rejects_opaque_user_name_without_emails` | Divergencia vs BA (`the-username`) |
| `users_patch_rejects_invalid_update_operation_with_scim_invalid_syntax` | Op `update` → SCIM `invalidSyntax` (upstream: Zod) |
| `users_delete_succeeds_without_content_type_header` | DELETE sin cabecera `Content-Type` |

### Tests OpenAuth sin par upstream (pasada 3)

| Test | Qué cubre |
| --- | --- |
| `management_before_token_hook_failure_preserves_existing_provider` | Upstream borraría la fila antes del hook |
| `management_before_token_hook_failure_aborts_persistence` | Primer generate sin fila previa |
| `routes/users.rs` — proyección, sort, enterprise filter | Superset |
| `store.rs` — `list_by_user`, `find_by_organization_id` | API store extra (no rutas públicas) |

## Comandos

```bash
# Crate completo
cargo nextest run -p openauth-scim

# Solo unit sync
cargo test -p openauth-scim --test scim filters:: -- --nocapture

# Un módulo de rutas
cargo test -p openauth-scim --test scim routes::users::
```

## Mantenimiento

Al añadir tests:

1. Actualizar esta matriz si cubre escenario upstream nuevo.
2. Mantener `tests/support/scim_parity.md` como índice corto para desarrolladores del crate.
3. Preferir nombre de test que cite comportamiento RFC o escenario BA (“should deny org-scoped token generation for a regular member”).
