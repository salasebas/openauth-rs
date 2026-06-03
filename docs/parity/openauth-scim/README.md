# Paridad: `openauth-scim` ↔ `@better-auth/scim`

Documentación de paridad **solo servidor** entre OpenAuth y Better Auth **v1.6.9**.

| Campo | Valor |
| --- | --- |
| Upstream npm | `@better-auth/scim@1.6.9` |
| Upstream path | `reference/upstream-src/1.6.9/repository/packages/scim/` |
| Crate Rust | `crates/openauth-scim` (`openauth-scim` en crates.io) |
| Paridad pin | [`reference/upstream-better-auth/VERSION.md`](../../../reference/upstream-better-auth/VERSION.md) |
| Checklist histórico | [`docs/superpowers/plans/2026-05-12-scim-upstream-parity.md`](../../superpowers/plans/2026-05-12-scim-upstream-parity.md) |
| Notas en crate (legado) | [`crates/openauth-scim/docs/better-auth-design-differences.md`](../../../crates/openauth-scim/docs/better-auth-design-differences.md) |
| Matriz tests (legado) | [`crates/openauth-scim/tests/support/scim_parity.md`](../../../crates/openauth-scim/tests/support/scim_parity.md) |

## Relación de paquetes

| Rol | Upstream | OpenAuth |
| --- | --- | --- |
| Plugin SCIM | `@better-auth/scim` (paquete npm independiente) | `openauth-scim` (crate independiente) |
| Cliente tipado | `@better-auth/scim/client` (`scimClient()`) | **No portado** (server-only) |
| Core / router | `better-auth`, `better-call` (peers) | `openauth-core` |
| Organizaciones | Plugin `organization` en runtime (tests montan SSO + org) | Plugin `organization` opcional en runtime (`has_plugin`) |
| Tablas extra | Solo `scimProvider` | `scim_providers` + `scim_user_profiles` + `scim_group_profiles` |

**No hay split ni merge de paquetes upstream:** es **1 paquete npm → 1 crate Rust**. OpenAuth **extiende** el mismo plugin con tablas y rutas adicionales (Groups, Bulk, `.search`), no divide el dominio SCIM en varios crates.

## Índice

| Documento | Contenido |
| --- | --- |
| [01-overview.md](./01-overview.md) | Resumen ejecutivo, alcance, estado de paridad |
| [02-package-mapping.md](./02-package-mapping.md) | Mapa archivo ↔ módulo, dependencias, esquema DB |
| [03-endpoints.md](./03-endpoints.md) | Inventario HTTP, auth, rutas solo OpenAuth |
| [04-features-and-options.md](./04-features-and-options.md) | Opciones, filtros, PATCH, metadata, capacidades |
| [05-design-decisions.md](./05-design-decisions.md) | Divergencias intencionales y por qué |
| [06-tests.md](./06-tests.md) | Conteos, matriz upstream Vitest ↔ Rust, cobertura extra |
| [07-edge-cases-and-integrators.md](./07-edge-cases-and-integrators.md) | Validación email, tokens, errores, feature `scim`, gaps de tests |

## Verificación rápida

```bash
cargo fmt --all --check
cargo clippy -p openauth-scim --all-targets -- -D warnings
cargo nextest run -p openauth-scim
```

| Métrica | Upstream (Vitest) | OpenAuth (`openauth-scim`) |
| --- | --- | --- |
| Archivos de test | 4 en `src/*.test.ts` | 28 módulos bajo `tests/scim/` |
| Declaraciones `it(` | 75 | — |
| Declaraciones `it.for(` | 6 (×2 casos c/u ≈ +12 runs) | — |
| Tests Rust `#[test]` | — | 40 |
| Tests Rust `#[tokio::test]` | — | 149 |
| **Total aprox.** | **~87** ejecuciones Vitest | **~189** tests |

## Estado resumido (servidor)

| Área | Paridad con BA 1.6.9 | Notas |
| --- | --- | --- |
| Management (4 rutas) | **Alta** | Misma semántica; token rotation vía `upsert` en Rust |
| Users CRUD + metadata | **Alta** | Mismo modelo provider/org; DELETE borra usuario global |
| Filtro list `userName eq` | **Alta** | SQL pushdown equivalente |
| PATCH Users (add/replace) | **Alta** | OpenAuth además implementa `remove` |
| Token storage modes | **Alta** | Default distinto: plain (BA) vs hashed (OpenAuth) |
| Client `scimClient()` | **N/A** | TS-only; no aplica server-only |
| Groups / Bulk / `.search` | **Superset** | No existen en upstream 1.6.9 |
| ServiceProviderConfig flags | **Divergente honesto** | BA subdeclara; OpenAuth refleja implementación real |

Última auditoría documentada: **2026-06-01** (tres pasadas; Better Auth `v1.6.9`, commit `f484269`).

### Hallazgos críticos (consolidado)

| Tema | Resumen |
| --- | --- |
| `userName` no-email | Upstream permite (`the-username`); OpenAuth exige email — **divergencia intencional** ([§07](./07-edge-cases-and-integrators.md)) |
| Regenerar token + `before` hook falla | Upstream **borra** provider antes del hook; OpenAuth **conserva** fila — **mejora OA** |
| `User.groups` en JSON | Solo OpenAuth (grupos org vía SCIM) |
| `email_verified` en create | OpenAuth `true`; upstream no explícito |
| PATCH transaccional | OpenAuth sí; upstream sin transacción |
| `default_scim` + hash default | Hashes solo en DB; `default_scim` plain en config |
| Feature `openauth/scim` | Documentado en §07 |
| Rate limit 429 | Solo en OpenAPI upstream; no en lógica del plugin |
