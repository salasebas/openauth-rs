# 03 — Endpoints HTTP

Rutas relativas al **base URL de auth** (ej. `https://app.example.com/api/auth`).

## Leyenda de autenticación

| Auth | Descripción |
| --- | --- |
| **Session** | Cookie/sesión OpenAuth (management) |
| **Bearer** | `Authorization: Bearer <base64url(baseToken:providerId[:organizationId])>` |
| **Public** | Sin bearer ni sesión |
| **501** | Ruta existe pero no soportada por diseño |

## Paridad directa (17 rutas)

Presentes en Better Auth 1.6.9 y OpenAuth con la misma ruta y rol de auth salvo notas.

| Método | Ruta | Upstream auth | OpenAuth auth | Paridad |
| --- | --- | --- | --- | --- |
| POST | `/scim/generate-token` | Session | Session | ✅ |
| GET | `/scim/list-provider-connections` | Session | Session | ✅ |
| GET | `/scim/get-provider-connection` | Session | Session | ✅ |
| POST | `/scim/delete-provider-connection` | Session | Session | ✅ |
| POST | `/scim/v2/Users` | Bearer | Bearer | ✅ |
| GET | `/scim/v2/Users` | Bearer | Bearer | ✅ (+ paginación OA) |
| GET | `/scim/v2/Users/:userId` | Bearer | Bearer | ✅ (+ ETag OA) |
| PUT | `/scim/v2/Users/:userId` | Bearer | Bearer | ✅ (+ If-Match OA) |
| PATCH | `/scim/v2/Users/:userId` | Bearer | Bearer | ✅ (+ remove, If-Match OA) |
| DELETE | `/scim/v2/Users/:userId` | Bearer | Bearer | ✅ (+ deprovision mode OA) |
| GET | `/scim/v2/ServiceProviderConfig` | Public | Public | ⚠️ flags distintos |
| GET | `/scim/v2/Schemas` | Public | Public | ⚠️ más schemas OA |
| GET | `/scim/v2/Schemas/:schemaId` | Public | Public | ⚠️ Group + Enterprise OA |
| GET | `/scim/v2/ResourceTypes` | Public | Public | ⚠️ + Group OA |
| GET | `/scim/v2/ResourceTypes/:resourceTypeId` | Public | Public | ⚠️ + Group OA |

### Códigos de estado alineados (Users + management)

| Operación | Éxito | Errores típicos |
| --- | --- | --- |
| generate-token | 201 | 401, 403, 400 (`:` en providerId) |
| create user | 201 + `Location` | 401, 409 uniqueness |
| list/get user | 200 | 401, 404 |
| put user | 200 | 401, 404 |
| patch user | 204 | 401, 404, 400 no-op |
| delete user | 204 | 401, 404 |
| delete provider | 200 `{ success: true }` | 401, 403, 404 |

Management errors: upstream `APIError` JSON; OpenAuth `OpenAuthError` JSON (misma familia que el core, no envelope SCIM).

### Errores adicionales (segunda auditoría)

| Situación | OpenAuth | Upstream |
| --- | --- | --- |
| PATCH con `op` desconocido | 400 SCIM `invalidSyntax` | Zod `VALIDATION_ERROR` (no SCIM) |
| Bearer org-scoped sin plugin `organization` | 400 SCIM `invalidValue` | Sin guard en middleware bearer |
| Regenerate token, distinto `organizationId` en body | 403 `"SCIM provider exists for a different scope"` | Lookup compuesto; `providerId` unique impide segunda fila |
| `get-provider-connection` sin `providerId` | 400 `"providerId is required"` | Zod query error |

Ver [07-edge-cases-and-integrators.md](./07-edge-cases-and-integrators.md).

## Rutas solo OpenAuth (9)

| Método | Ruta | Auth | Motivo |
| --- | --- | --- | --- |
| POST | `/scim/v2/Users/.search` | Bearer | IdPs que exigen POST search |
| POST | `/scim/v2/Groups` | Bearer + org provider | Groups → organization `team` |
| GET | `/scim/v2/Groups` | Bearer + org provider | |
| GET | `/scim/v2/Groups/:groupId` | Bearer + org provider | |
| PUT | `/scim/v2/Groups/:groupId` | Bearer + org + If-Match | |
| PATCH | `/scim/v2/Groups/:groupId` | Bearer + org + If-Match | |
| DELETE | `/scim/v2/Groups/:groupId` | Bearer + org + If-Match | |
| POST | `/scim/v2/Groups/.search` | Bearer + org provider | |
| POST | `/scim/v2/.search` | Bearer | Búsqueda combinada Users (+ Groups si org) |
| POST | `/scim/v2/Bulk` | Bearer | RFC 7644 bulk |
| GET | `/scim/v2/Me` | — | **501** — tokens de proveedor no son alias de usuario final |

**Tipo de gap:** extensión intencional (conformidad IdP / enterprise), no omisión de upstream.

## Rutas solo upstream (cliente)

| Superficie | Ruta HTTP | Notas |
| --- | --- | --- |
| `authClient.scim.*` | Delega a las 4 management | Tipos TS; un test llama `generateSCIMToken` vía client |

OpenAuth no expone cliente; integradores usan HTTP o su SDK.

## Bearer token (ambos)

```
base64url( baseToken + ":" + providerId [ + ":" + organizationId ] )
```

- `providerId` no puede contener `:`.
- `organizationId` puede contener `:` (resto del string tras el segundo `:`).
- Persistencia: solo `baseToken` transformado (plain/hash/encrypt/custom).

## ServiceProviderConfig — diferencia observable

| Flag | Better Auth 1.6.9 | OpenAuth |
| --- | --- | --- |
| `patch.supported` | true | true |
| `bulk.supported` | **false** | **true** (+ maxOperations, maxPayloadSize) |
| `filter.supported` | true | true (+ maxResults 200) |
| `changePassword.supported` | false | false |
| `sort.supported` | **false** | **true** |
| `etag.supported` | **false** | **true** |

**Por qué:** upstream no implementa bulk/sort/etag pero OpenAuth sí; BA declara `false` por honestidad mínima; OA declara capacidades reales ([05-design-decisions.md](./05-design-decisions.md)).

## Registro en código

OpenAuth — `src/routes.rs` función `endpoints()` (26 handlers):

```77:106:crates/openauth-scim/src/routes.rs
pub fn endpoints(options: ScimOptions) -> Vec<openauth_core::api::AsyncAuthEndpoint> {
    let options = Arc::new(options);
    vec![
        management::generate_token_endpoint(Arc::clone(&options)),
        // ... management, users, groups, bulk, metadata ...
    ]
}
```

Upstream — `src/index.ts` objeto `endpoints` (14 handlers nombrados; sin Groups/Bulk/search/Me).
