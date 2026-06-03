# 05 — Decisiones de diseño y divergencias

Cada fila indica **por qué** OpenAuth no copia upstream literalmente. Categorías:

| Etiqueta | Significado |
| --- | --- |
| **Paridad** | Mismo comportamiento observable |
| **Seguridad** | Default más seguro en servidor |
| **IdP / RFC** | Requisito de proveedores enterprise |
| **Server-only** | Upstream es TS/client o UI; no aplica |
| **Rust / idioma** | Implementación idiomática |
| **Producto** | Elección explícita OpenAuth |

## Divergencias con upstream 1.6.9

| Tema | Better Auth | OpenAuth | Etiqueta | Por qué |
| --- | --- | --- | --- | --- |
| Default token storage | `plain` | SHA-256 hashed | **Seguridad** | Filtraciones de DB no exponen secretos; migración = regenerar tokens |
| Rotación de token | DELETE provider + CREATE | UPSERT misma `provider_id` | **Rust / producto** | ID estable, menos ventana sin token |
| ServiceProviderConfig `bulk/sort/etag` | `false` | `true` | **IdP / RFC** | OA implementa lo que BA no tiene; metadata honesta |
| PATCH `remove` | Ignorado en `buildUserPatch` | Implementado | **IdP / RFC** | Clientes SCIM estándar envían remove |
| Filtros list | Solo `userName eq` | + parser en memoria | **IdP / RFC** | Enterprise extension attrs sin segundo DSL |
| Paginación list | Devuelve todos | `startIndex`/`count`, max 200 | **IdP / RFC** | Escala en directorios grandes |
| Validación email | Permisiva | `validation.rs` | **Seguridad** | Evitar identidades basura en provision |
| DELETE usuario | Solo delete global | + `UnlinkAccount` | **Producto** | Permite desprovisionar un IdP sin borrar usuario multi-cuenta |
| Groups / Bulk / search | No existen | Implementados | **IdP / RFC** | Azure AD, Okta, etc. suelen exigir Groups/Bulk |
| GET `/Me` | No hay ruta | 501 SCIM error | **Server-only / claridad** | Bearer de proveedor ≠ sesión de usuario |
| Cliente `scimClient()` | Export `./client` | No portado | **Server-only** | Inferencia TS; integradores HTTP/Rust |
| Plugin registry TS | `BetterAuthPluginRegistry` | No | **Server-only** | Solo tipos en ecosistema BA |
| Management errors | `APIError` | `OpenAuthError` | **Rust / idioma** | Misma familia que resto del servidor |
| Tablas `scim_*_profiles` | No | Sí | **IdP / RFC** | Extensiones sin ensuciar `users` |
| Groups → `team` | — | Mapeo organization | **Producto** | Reutilizar modelo org existente en OpenAuth |
| Constant-time compare tokens | String `===` | `subtle` | **Seguridad** | Endurecimiento Rust |
| Metadata snapshot CI | No en paquete | `metadata_snapshot.rs` | **Producto** | Evitar drift de schemas publicados |
| `userName` debe ser email | Permite identificadores opacos | `validate_scim_user_identity` | **Seguridad** | Upstream tests usan `the-username`; ver §07 |
| `email_verified` en provision | Implícito/default core | `true` en SCIM create | **Producto** | IdP-provisioned users |
| PATCH en transacción | `Promise.all` sin txn | Txn en `user_resources` | **Rust / producto** | Atomicidad user+account+profile |
| PATCH op inválido | Zod (no SCIM) | SCIM `invalidSyntax` | **Idioma** | Frontera validación vs protocolo |
| Bearer base64 padded | Utils decode | Fallback `URL_SAFE` | **Interop** | Tokens con padding |
| `default_scim` hashing | Plain compare | Plain en config; hash solo DB | **Documentación** | No aplicar `Hashed` a static providers |
| Org bearer sin plugin org | Sin guard en middleware | 400 en rutas SCIM | **Producto** | Falla explícita vs 401 opaco |
| List management sin plugin org | Providers org inaccesibles | Omitidos del list | **Producto** | §07 |
| Regenerar token: orden delete vs hook | Delete fila, luego `before` hook | `before` hook, luego `upsert` | **Seguridad / producto** | Hook fallido no deja sin provider |
| `User.groups` en GET/POST User | Vacío | Desde teams SCIM de la org | **IdP / RFC** | Lectura de membresías |
| Extensiones en perfil (`phoneNumbers`, URN) | No | Perfil JSON + validación `primary` | **Superset** | §07 |

## Alineaciones explícitas (no cambiar sin documentar)

| Comportamiento | Notas |
| --- | --- |
| `providerId` globalmente único | Igual que BA; no composite `(providerId, organizationId)` |
| Formato bearer base64url con `:` | Compatible con tokens generados en BA |
| DELETE default borra usuario global | Paridad con `deleteUser`; sorpresa compartida si email compartido |
| Hooks y ownership en management | Misma intención |
| `userName eq` → email en SQL | Paridad list filter |
| Metadata User schema (core) | Misma URN y campos principales |
| 409 uniqueness en duplicate account | Mismo `scimType` |

## Qué NO portamos (y no es gap)

| Upstream | Razón |
| --- | --- |
| `scimClient()` | Solo tipos cliente Better Auth |
| `HIDE_METADATA` en OpenAPI público | Detalle de documentación BA |
| Vitest fixtures `createAuthClient` | Reemplazado por helpers Rust en `routes/support.rs` |
| Dashboard Infra SCIM UI | Fuera de alcance server crate |
| Dependencia runtime a `@better-auth/sso` | SCIM no llama SSO; solo id de provider en `account` |

## Decisiones abiertas (producto)

| Tema | Estado | Notas |
| --- | --- | --- |
| DELETE con email compartido entre IdPs | Documentado | Upstream también borra usuario entero; OA ofrece `UnlinkAccount` |
| Unicidad composite provider+org | Rechazado | No es upstream; usar `providerId` distintos |
| MongoDB SCIM tests | Pendiente | Falta `DbAdapter` Mongo en OpenAuth |
| Bulk `If-Match` por operación | Por diseño omitido | Solo rutas directas PUT/PATCH/DELETE |

## Referencia rápida upstream

Lógica principal en un solo archivo grande:

- `reference/upstream-src/1.6.9/repository/packages/scim/src/routes.ts`

PATCH ignore remove:

```133:136:reference/upstream-src/1.6.9/repository/packages/scim/src/patch-operations.ts
	for (const operation of operations) {
		if (operation.op !== "add" && operation.op !== "replace") {
			continue;
		}
```

ServiceProviderConfig upstream:

```1052:1058:reference/upstream-src/1.6.9/repository/packages/scim/src/routes.ts
		return ctx.json({
			patch: { supported: true },
			bulk: { supported: false },
			filter: { supported: true },
			changePassword: { supported: false },
			sort: { supported: false },
			etag: { supported: false },
```
