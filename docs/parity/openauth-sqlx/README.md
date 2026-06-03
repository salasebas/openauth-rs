# openauth-sqlx — upstream parity index

**OpenAuth crate:** `crates/openauth-sqlx` (`openauth-sqlx` on crates.io)  
**Primary upstream reference:** `@better-auth/kysely-adapter` v1.6.9  
**Secondary upstream reference:** `better-auth` package — `src/db/get-migration.ts`, CLI `migrate`  
**Shared contract:** `@better-auth/core` adapter factory ↔ `openauth-core::db`  
**Parity target version:** [VERSION.md](../../../reference/upstream-better-auth/VERSION.md)  
**Last reviewed:** 2026-06-01

## Summary

`openauth-sqlx` is the **SQLx execution layer** for OpenAuth’s `DbAdapter`. It is
**not** a port of Drizzle, Prisma, or Mongo adapters. Observable SQL behavior is
aligned with Better Auth’s **Kysely** path plus **Kysely-only runtime migrations**.

Estimated **server-only** parity with that path: **~95%** for CRUD/migrations
**behavior** exercised by our tests. **Test surface parity is lower**: upstream
runs ~7 shared adapter suites per dialect under `e2e/adapter/test/kysely-adapter/`
(see [testing.md](testing.md)); we use a smaller harness plus custom integration
tests.

Gaps are mostly intentional Rust/OpenAuth choices, missing MSSQL, upstream-only
drivers (D1, Bun, `node:sqlite`), or features we extend (safer delete, migration
warnings, missing-index repair).

## Documents in this folder

| File | Contents |
| --- | --- |
| [source-inventory.md](source-inventory.md) | Every source/test file and upstream path (code-verified) |
| [upstream-mapping.md](upstream-mapping.md) | How upstream packages split vs our crates |
| [functional-parity.md](functional-parity.md) | Feature-by-feature parity matrix |
| [testing.md](testing.md) | Test counts, e2e suites, cross-crate coverage |
| [design-decisions.md](design-decisions.md) | Intentional differences with rationale |
| [openauth-beyond-upstream.md](openauth-beyond-upstream.md) | Extensions past Better Auth Kysely |
| [parity-gaps.md](parity-gaps.md) | Non-obvious gaps (joins, DDL, factory) from code review |

## Related docs (outside this folder)

| Location | Role |
| --- | --- |
| `crates/openauth-sqlx/UPSTREAM_PARITY.md` | Short parity summary (kept in crate) |
| `crates/openauth-core/SQL_ADAPTER_PARITY.md` | Shared SQL rules (identifiers, LIKE) |
| `docs/superpowers/plans/2026-05-16-sqlx-plugin-aware-migrations-hardening.md` | Migration hardening implementation history |

## Out of scope for this crate

| Upstream surface | Why excluded |
| --- | --- |
| `@better-auth/drizzle-adapter` | Different ORM; users run Drizzle migrate, not OpenAuth SQL planner |
| `@better-auth/prisma-adapter` | Prisma schema/migrate toolchain |
| `@better-auth/mongo-adapter` | Non-SQL |
| `@better-auth/memory-adapter` | In-memory; OpenAuth has `openauth-core` memory adapter |
| Client-only Better Auth APIs | OpenAuth is server-only |
| `mssql`, Cloudflare D1, Bun/`node:sqlite` Kysely dialect shims | Not implemented in SQLx crate today |

## Alternate Postgres drivers (same contract, different crate)

| OpenAuth crate | Upstream analogue |
| --- | --- |
| `openauth-tokio-postgres` | Kysely Postgres behavior (no SQLx) |
| `openauth-deadpool-postgres` | Same, pool via deadpool |

These share migration planning patterns with `openauth-sqlx` Postgres but are
documented separately when parity folders exist for them.
