# 07 — Casos límite, errores y integración

Detalles que no caben en las tablas generales de [04-features-and-options.md](./04-features-and-options.md) y [03-endpoints.md](./03-endpoints.md).

| Auditoría | Fecha |
| --- | --- |
| Pasada 1–2 | 2026-06-01 |
| Pasada 3 (hooks, respuestas User, validación extensiones) | 2026-06-01 |

---

## Integración en el workspace OpenAuth

| Paso | Acción |
| --- | --- |
| Dependencia | `openauth = { version = "...", features = ["scim"] }` o path + feature |
| Crate directo | `openauth-scim = { ... }` y `use openauth_scim::scim` |
| Re-export | Con feature `scim`, `openauth::scim` re-exporta el crate (`crates/openauth/src/lib.rs`) |
| Plugin | `.plugin(openauth::scim::scim(ScimOptions::default()))` |
| Migraciones | Ejecutar migraciones del adapter tras añadir el plugin (tablas `scim_*`) |
| Organization | Plugin `organization` requerido para tokens/rutas con `organizationId` y para Groups |

Sin feature `scim` en el meta-crate `openauth`, el plugin no se compila ni se re-exporta.

---

## Validación de identidad (divergencia importante)

| Regla | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| `userName` en POST/PUT | `z.string().lowercase()` — **no** exige formato email | Debe resolver a email válido (`validation.rs`) |
| `emails[].value` | `z.email()` cuando hay array | Misma validación + máximo un `primary: true` |
| Ejemplo upstream | `userName: "the-username"` crea usuario (tests `scim.test.ts`) | **400** `invalidValue` si no es email |
| Link-by-email | `userName` arbitrario + `emails[].value` email | Cubierto en `provisioning.rs` |

**Etiqueta:** **Seguridad / producto** (no bug). IdPs enterprise suelen enviar email en `userName`; portar literalmente `the-username` rompería el modelo OpenAuth de `users.email`.

**Tests:** `parity_gaps::users_create_rejects_opaque_user_name_without_emails` documenta el rechazo OpenAuth; no hay test que espere éxito con `userName` no-email (upstream sí).

---

## Regeneración de token y hooks (divergencia importante)

Orden en `POST /scim/generate-token` cuando ya existe un provider:

| Paso | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| Provider existente | `delete` por `id` **antes** del hook | Sin delete; `upsert` **después** de `before_token_generated` |
| `beforeSCIMTokenGenerated` falla | Fila ya borrada → **sin provider** en DB | Fila intacta → token anterior sigue válido |
| `afterSCIMTokenGenerated` | Tras `create` | Tras `upsert` |

**Impacto:** en upstream, un `beforeSCIMTokenGenerated` que rechaza la rotación puede **eliminar** la conexión SCIM sin emitir token nuevo. OpenAuth lo trata como error sin borrar (`management_before_token_hook_failure_preserves_existing_provider` en tests).

**Etiqueta:** mejora intencional / paridad de seguridad operacional.

Hooks upstream lanzan `APIError`; OpenAuth usa `ScimHookError` → JSON management con `code` + `message`.

---

## Management y providers

| Comportamiento | Better Auth | OpenAuth |
| --- | --- | --- |
| `providerId` único en DB | Sí (`unique: true`) | Sí |
| Regenerar token (mismo `providerId`) | `find` por `providerId` + `organizationId` opcional → delete → create | `find_by_provider_id` → si `organization_id` del body ≠ fila → **403** `"SCIM provider exists for a different scope"` → `upsert` |
| Cambiar scope org en regenerate | Con `providerId` único, crear otra fila con distinto `organizationId` **fallaría** en unique; lookup compuesto solo elige fila a reemplazar | Explícito 403 antes de upsert |
| Delete provider `where` | Solo `providerId` | Por `provider_id` (equivalente con unicidad global) |
| List sin plugin `organization` | Org providers: sin membership → no listados | Filas con `organization_id` **omitidas** del list (`continue`) |
| Get/delete org provider sin plugin org | `assertSCIMProviderAccess` → 403 org plugin required | **403** mismo mensaje (`provider_scope_supported_for_management`) |
| `get-provider-connection` sin query | Zod error | **400** `"providerId is required"` |
| Ownership + `user_id` null | Legacy: cualquier usuario accede | Igual si ownership deshabilitado; con ownership habilitado, `user_id == None` ⇒ **cualquier** usuario (igual que upstream) |

---

## Bearer token y `defaultSCIM`

| Comportamiento | Better Auth | OpenAuth |
| --- | --- | --- |
| Decode base64 | `base64Url.decode` | `URL_SAFE_NO_PAD`, fallback **`URL_SAFE` (padded)** |
| `defaultSCIM` match | `providerId` solo si token sin segmento org; si hay org, ambos deben coincidir | `default_provider_matches` equivalente |
| Token en `defaultSCIM` | Comparación plain `===` | `plain_token_matches` (constant-time) |
| `default_scim` + `Hashed` default | N/A (plain en config) | **`default_scim` no pasa por `ScimTokenStorage`** — secretos en config son plain; solo filas DB se hashean |
| Bearer org ≠ fila DB `organization_id` | Lookup con ambos campos → miss → 401 | **401** tras mismatch explícito (`auth_context.rs`) |
| Rutas `/scim/v2/*` con provider org-scoped sin plugin `organization` | No hay guard previo en middleware | **400** SCIM `invalidValue` en cada ruta bearer (`ensure_scim_provider_scope_supported`) |

---

## Respuesta SCIM User (campos extra OpenAuth)

| Campo / comportamiento | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| `groups[]` en User | Siempre vacío en `createUserResource` | Relleno con grupos SCIM de la org (`merge_scim_user_groups`) |
| `meta.version` (ETag) | No | Weak ETag desde perfil / `resource_version` |
| `additional_fields` / URN enterprise | No persistidos | JSON en `scim_user_profiles.attributes` + `schemas` extra |
| Atributos core en body “extra” | Ignorados | **400** `scimType: mutability` si se envían como perfil (`is_reserved_scim_user_profile_attribute`) |
| `phoneNumbers`, `roles`, etc. | No validados | Máximo un `primary: true` por multivalued (`validate_multivalued_primary_attributes`) |
| `active` | Siempre `true` en serialización base | `true` en base; perfil puede añadir campos vía `additional_fields` (no `active` reservado en perfil) |

Upstream no expone membresía de grupo en el recurso User; IdPs que lean `groups` solo obtienen datos en OpenAuth.

---

## Users: provisioning y persistencia

| Comportamiento | Better Auth | OpenAuth |
| --- | --- | --- |
| Create user nuevo | `createUser({ email, name })` | `CreateUserInput` + **`email_verified(true)`** |
| Cuenta OAuth al crear | `accessToken: ""`, `refreshToken: ""` | Campos `None` en `CreateOAuthAccountInput` | Equivalente según adapter |
| `userName` normalizado | Zod `.lowercase()` | `to_ascii_lowercase()` en handler | Alineado |
| Create/link transaccional | `adapter.transaction` | `adapter.transaction` + perfil `scim_user_profiles` |
| PATCH apply | `Promise.all` user + account, **sin** transacción | Transacción en `update_scim_user_account_and_merge_profile` |
| PATCH `op` inválido (ej. `update`) | **Zod** → cuerpo `VALIDATION_ERROR` (no SCIM) | **400** SCIM `invalidSyntax` — test `parity_gaps::users_patch_rejects_invalid_update_operation_with_scim_invalid_syntax` |
| List con `filter` | Log `info` con filtros parseados | Sin log equivalente (audit opcional aparte) |
| DELETE media types | `application/json`, `application/scim+json`, **`""`** | Solo JSON/SCIM en metadata de endpoint |
| `active` en respuesta | Siempre `true` | Desde perfil / recurso (puede reflejar estado) |
| Campos extension en User | No persistidos | JSON en `scim_user_profiles.attributes` |

---

## Groups / Bulk (solo OpenAuth — límites)

| Límite / regla | Valor / nota |
| --- | --- |
| Bulk `maxOperations` | 1000 (`metadata::SCIM_BULK_MAX_OPERATIONS`) |
| Bulk `maxPayloadSize` | 1_048_576 bytes |
| Filter `maxResults` | 200 en ServiceProviderConfig |
| Groups | Rechazo miembros grupo anidado (`reject_nested_group_members`) |
| Groups | `displayName` vacío → error |
| Bulk | Sin `If-Match` por operación |
| Bulk Atomic | Requiere adapter con transacciones nativas |

---

## Matriz ampliada de errores SCIM

| Situación | HTTP | `scimType` (si aplica) | Upstream | OpenAuth |
| --- | --- | --- | --- | --- |
| Sin bearer | 401 | — | ✅ | ✅ |
| Bearer malformado | 401 | — | ✅ | ✅ |
| Filtro inválido | 400 | `invalidFilter` | ✅ | ✅ |
| PATCH sin campos | 400 | — | ✅ | ✅ |
| PATCH op inválido | 400 Zod | — | Zod no-SCIM | SCIM `invalidSyntax` |
| Duplicate account | 409 | `uniqueness` | ✅ | ✅ |
| Org provider sin plugin (bearer) | — | — | No guard | 400 `invalidValue` |
| Bulk payload grande | — | — | N/A | 400 `tooMany` |
| ETag stale | 412 | — | N/A | ✅ Users/Groups |

Management sigue usando JSON core (`UNAUTHORIZED`, `FORBIDDEN`, `BAD_REQUEST`, `NOT_FOUND`) — no envelope SCIM.

---

## OpenAPI / rate limit

Upstream documenta respuesta SCIM **429** en `SCIMErrorOpenAPISchemas`; ningún paquete SCIM aplica rate limiting (lo hace el host si aplica).

---

## Escenarios de test upstream sin equivalente explícito

| Escenario upstream | OpenAuth |
| --- | --- |
| POST user `userName: "the-username"` solo | Fallaría validación; **sin** test de “éxito” |
| POST user givenName + familyName con userName opaco | Fallaría validación; **sin** test de ruta |
| POST user primary en `emails[]` + userName opaco | Fallaría si userName no es email |
| PATCH op `update` + envelope Zod | Test con `multiply` + SCIM (forma distinta) |
| Regenerate token cambiando `organizationId` en body | **403** scope; **sin** test |
| `before` hook falla con provider ya existente | Upstream deja DB sin provider; OA preserva — test `preserves_existing_provider` |
| `scimClient().generateSCIMToken` | ➖ N/A |

## Escenarios con cobertura OpenAuth extra (recordatorio)

Ver [06-tests.md](./06-tests.md): `isolation.rs`, `bulk.rs`, `groups_*.rs`, `db_adapters.rs`, `metadata_snapshot.rs`, `deprovision.rs`, `concurrency.rs`, `audit.rs`.

---

## Referencias de código

Upstream `userName` sin validación email:

```3:5:reference/upstream-src/1.6.9/repository/packages/scim/src/user-schemas.ts
export const APIUserSchema = z.object({
	userName: z.string().lowercase(),
	externalId: z.string().optional(),
```

OpenAuth validación:

```23:38:crates/openauth-scim/src/validation.rs
pub fn validate_scim_user_identity(
    user_name: &str,
    emails: &[ScimEmail],
) -> Result<String, ScimError> {
    // ...
    if !is_valid_email(&email) {
        return Err(ScimError::bad_request(
            "userName and emails.value must resolve to a valid email address",
        )
        .with_scim_type("invalidValue"));
    }
```

Upstream regenerate: delete antes del hook `before`:

```273:297:reference/upstream-src/1.6.9/repository/packages/scim/src/routes.ts
			if (scimProvider) {
				await assertSCIMProviderAccess(
					ctx,
					user.id,
					scimProvider,
					requiredRole,
				);
				await ctx.context.adapter.delete<SCIMProvider>({
					model: "scimProvider",
					where: [{ field: "id", value: scimProvider.id }],
				});
			}

			const baseToken = generateRandomString(24);
			// ...
			if (opts.beforeSCIMTokenGenerated) {
				await opts.beforeSCIMTokenGenerated({
```

Upstream PATCH sin transacción:

```960:973:reference/upstream-src/1.6.9/repository/packages/scim/src/routes.ts
			await Promise.all([
				Object.keys(userPatch).length > 0
					? ctx.context.internalAdapter.updateUser(userId, {
							...userPatch,
							updatedAt: new Date(),
						})
					: Promise.resolve(),
				Object.keys(accountPatch).length > 0
					? ctx.context.internalAdapter.updateAccount(account.id, {
							...accountPatch,
							updatedAt: new Date(),
						})
					: Promise.resolve(),
			]);
```
