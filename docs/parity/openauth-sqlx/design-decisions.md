# Design decisions: openauth-sqlx vs Better Auth Kysely

Intentional differences grouped by **reason**. Each item is server-side only;
client SDK behavior is out of scope.

## Safety & security

| Topic | Better Auth (Kysely path) | OpenAuth | Rationale |
| --- | --- | --- | --- |
| `delete` semantics | Deletes **all** rows matching WHERE | Deletes **one** row; `delete_many` for bulk | Prevent accidental mass delete from generic call sites |
| LIKE / ILIKE patterns | User input can act as SQL wildcards | Escape `%`, `_`, `\` + explicit `ESCAPE` | Untrusted filter values must not broaden queries ([SQL_ADAPTER_PARITY.md](../../../crates/openauth-core/SQL_ADAPTER_PARITY.md)) |
| Migration type mismatch | Logs warning; migration may still run | Warning in plan; `run_migrations` / `create_schema` **fail** if warnings | Additive-only policy; avoid silent schema drift |
| Migration FK mismatch | Less explicit | SQLite plans can warn on FK mismatch | Surfaces unsafe plugin/schema changes early |

## Rust / OpenAuth architecture

| Topic | Better Auth | OpenAuth | Rationale |
| --- | --- | --- | --- |
| SQL planning location | Split: factory in core, SQL in Kysely adapter | **`openauth-core`** plans; sqlx **executes** | Reuse across SQLx, tokio-postgres, deadpool-postgres |
| Default `findMany` limit | Factory default **100** | **No limit** | Typed query contract; callers set limits on user-facing lists |
| Default values / `onUpdate` in adapter | Factory applies | Service layer sets fields | Explicit lifecycle in Rust handlers |
| Rate limit consume | Read on request, write on response | **Single transaction** consume | `RateLimitStore` trait requires atomic decision + persist |
| Error type | JS exceptions / `BetterAuthError` | `OpenAuthError::Adapter` | Idiomatic Rust |

## SQLx / Postgres idioms

| Topic | Better Auth Kysely | OpenAuth SQLx | Rationale |
| --- | --- | --- | --- |
| Postgres array columns | JSONB in migrations; `supportsArrays: false` | Native arrays; `supports_arrays: true` | SQLx + Postgres native types are first-class |
| MySQL timestamps | `timestamp(3)` | `DATETIME(6)` | Avoid TZ/range surprises; microsecond precision |
| Count query | `COUNT(id)` | `COUNT(*)` | Same results for these schemas; no dependency on id column name |

## Scope / platform (not “missing parity” by accident)

| Topic | Upstream | OpenAuth | Category |
| --- | --- | --- | --- |
| Drizzle / Prisma adapters | First-class packages | **No crate** | **Design** — server users pick one SQL stack |
| `auth migrate` only for Kysely | yes | OpenAuth CLI targets SQLx URL config | **Design** — align CLI with Rust adapters |
| MSSQL | Supported | Not implemented | **Gap** — low priority unless requested |
| D1 / Bun / node:sqlite Kysely dialects | yes | Standard SQLx SQLite only | **N/A** — different deployment targets |
| MongoDB | `mongo-adapter` | separate storage story | **N/A** |
| Client hooks / browser session | yes | server-only | **N/A** |

## Behavioral parity we explicitly keep

These match upstream Kysely behavior and are regression-tested:

- Parameter binding (no string-concatenated user values in SQL)
- Logical → physical table/column resolution
- Join null / empty array semantics and join limits
- Transaction rollback on failed callback
- Additive migrations: create table → add column → create index order
- MySQL “insert then SELECT” when RETURNING unavailable
- Plugin-aware schema extensions (tables, columns, indexes, FKs)
- SQL rate limit table with physical names

## Capability flags vs Kysely (important for factory consumers)

Better Auth’s Kysely adapter **under-reports** SQLite abilities (`supportsBooleans`,
`supportsDates`, `supportsJSON`, `supportsArrays`) because values are often stored as
text/integer. OpenAuth SQLx adapters **over-report** relative to Kysely flags while still
using similar storage on SQLite/MySQL for arrays.

Do not expect `adapter.capabilities()` to match Kysely’s `adapterId: "kysely"` metadata
byte-for-byte. Compare **observable SQL** via tests instead.

## Future work (not intentional forever)

| Item | Notes |
| --- | --- |
| MSSQL dialect | Would need `openauth-sqlx` feature + core `SqlDialect` |
| Align Postgres/MySQL tests with SQLite-only safety cases | Done for FK, rate limit, hooked tx, multi-join; SQLite-only pool FK pragma remains sqlite-only — see [testing.md](testing.md) |
| Configurable default `FindMany` limit | Belongs in **core** API, not sqlx-only |
| Port more `@better-auth/test-utils` organization cases | If organization plugin parity expands |
| Run upstream-equivalent e2e suite against SQLx | Large effort; today only `run_adapter_contract` |
| Postgres/MySQL: FK warning, hooked adapter, rate-limit deny tests | Parity with SQLite integration depth |
| `schemaRefTestSuite` for `openauth-sqlx` Postgres | Today: `openauth-tokio-postgres` + `public_api` |

## Quick reference: crate-local summary

The short form of this document remains in
[`crates/openauth-sqlx/UPSTREAM_PARITY.md`](../../../crates/openauth-sqlx/UPSTREAM_PARITY.md).
This folder is the **expanded** parity record for reviews and iteration.
