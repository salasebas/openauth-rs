# SCIM Upstream Parity Audit

## Summary

This audit compares the server-side OpenAuth SCIM crate against Better Auth
SCIM 1.6.9 under `upstream/better-auth/1.6.9/repository/packages/scim`.

The OpenAuth implementation is already at parity for the upstream server-side
SCIM behavior that Better Auth implements, while intentionally providing a
broader SCIM 2.0 surface. No Rust source, tests, public exports, feature gates,
schema contributions, or dependencies need to change from this audit.

Estimated server-only parity with upstream Better Auth SCIM 1.6.9: **98%**.
The remaining 2% is not missing runtime behavior. It is made up of intentional
Rust/OpenAuth differences in public API ergonomics, management error envelopes,
and metadata capability reporting for features OpenAuth actually implements.

## Upstream Files Inspected

- `src/index.ts`
- `src/types.ts`
- `src/middlewares.ts`
- `src/routes.ts`
- `src/mappings.ts`
- `src/patch-operations.ts`
- `src/scim-error.ts`
- `src/scim-filters.ts`
- `src/scim-metadata.ts`
- `src/scim-resources.ts`
- `src/scim-tokens.ts`
- `src/user-schemas.ts`
- `src/scim.test.ts`
- `src/scim-users.test.ts`
- `src/scim-patch.test.ts`
- `src/scim.management.test.ts`

Client-only package inference in `src/client.ts` was intentionally excluded
from the parity target because this audit is server-side only.

## OpenAuth Files Inspected

- `crates/openauth-scim/src/lib.rs`
- `crates/openauth-scim/src/options.rs`
- `crates/openauth-scim/src/token.rs`
- `crates/openauth-scim/src/store.rs`
- `crates/openauth-scim/src/errors.rs`
- `crates/openauth-scim/src/mappings.rs`
- `crates/openauth-scim/src/metadata.rs`
- `crates/openauth-scim/src/schema.rs`
- `crates/openauth-scim/src/patch.rs`
- `crates/openauth-scim/src/filters.rs`
- `crates/openauth-scim/src/routes.rs`
- `crates/openauth-scim/src/routes/*`
- `crates/openauth-scim/tests/scim/*`
- `crates/openauth-scim/tests/scim/routes/*`
- `crates/openauth-scim/tests/support/scim_parity.md`

Historical planning reference:

- `docs/superpowers/plans/2026-05-12-scim-upstream-parity.md`

## Confirmed Matches

- Plugin identity and version surface preserve the upstream `scim` plugin id.
- Provider storage covers upstream `providerId`, `scimToken`, optional
  `organizationId`, and optional ownership `userId`.
- Token generation keeps the upstream bearer shape:
  `baseToken:providerId[:organizationId]`, base64url encoded.
- Provider ids containing `:` are rejected to protect the token delimiter
  format.
- Plain, hashed, encrypted, custom hash, and custom encryption token storage
  modes are implemented.
- Default SCIM providers are checked before persisted providers and use plain
  token comparison.
- SCIM bearer authentication accepts case-insensitive `Bearer` schemes,
  rejects missing or malformed tokens, and returns SCIM 401 errors on protocol
  routes.
- Management routes require authenticated OpenAuth sessions.
- Organization-scoped provider management requires organization membership and
  the configured role policy.
- Default required roles match upstream intent: `admin` and the organization
  creator role, defaulting to `owner`.
- Provider ownership prevents non-owners from managing personal providers when
  ownership is enabled, while preserving legacy personal providers without
  `userId`.
- Before and after token generation hooks run at the same lifecycle points as
  upstream.
- User create, list, get, PUT, PATCH, and DELETE preserve upstream provider and
  organization isolation behavior.
- Account id mapping uses `externalId` when present and falls back to
  `userName`.
- Primary email mapping prefers a primary email, then the first email, then
  `userName`.
- Name mapping prefers `name.formatted`, then name parts, then email.
- Duplicate provider account creation returns SCIM 409 with
  `scimType: "uniqueness"`.
- SCIM errors include the RFC 7644 error schema, string HTTP status, detail,
  and `scimType` where applicable.
- Filter handling preserves upstream `userName eq "..."` database filtering and
  returns `invalidFilter` for invalid filters.
- PatchOp handling preserves upstream replace/add behavior, dotted path
  normalization, default `replace`, no-op rejection, and SCIM 204 success.
- Metadata routes expose ServiceProviderConfig, Schemas, Schema by id,
  ResourceTypes, and ResourceType by id without requiring bearer auth.
- Adapter-backed tests cover memory, SQLx SQLite/Postgres/MySQL,
  tokio-postgres, and deadpool-postgres surfaces where configured.

## Confirmed Intentional Differences

- OpenAuth supports SCIM Groups; upstream 1.6.9 exposes only User resources.
- OpenAuth supports SCIM Bulk operations and advertises Bulk support; upstream
  reports `bulk.supported: false`.
- OpenAuth supports `.search` endpoints for Users, Groups, and all resources.
- OpenAuth supports pagination, projection, sorting, weak ETags, and
  `If-Match` precondition checks.
- OpenAuth persists SCIM user and group extension/profile attributes rather
  than limiting responses to the upstream core User projection.
- OpenAuth returns `application/scim+json` for SCIM protocol routes and errors,
  which is stricter than upstream's generic JSON response metadata.
- OpenAuth uses constant-time token comparison for plain, hashed, decrypted,
  custom, and default-provider token checks.
- OpenAuth rejects organization-scoped SCIM providers on protocol routes when
  the organization plugin is absent, rather than allowing ambiguous org scope.
- OpenAuth implements `/scim/v2/Me` as an explicit SCIM 501 because
  provider-scoped SCIM bearer tokens are not end-user aliases.
- OpenAuth deletes SCIM profile and team membership data when deleting a SCIM
  user, avoiding stale provisioning state.

These differences are intentionally left in place because they are either
production hardening, RFC-facing SCIM 2.0 functionality, or OpenAuth-specific
server architecture choices that preserve upstream behavior where upstream has
observable behavior.

## Risks

- The OpenAuth SCIM surface is broader than Better Auth 1.6.9, so future
  upstream changes may not map one-to-one to the Rust module.
- Email validation is intentionally local and simple. It is stricter than
  accepting arbitrary strings and is covered by current SCIM route tests, but it
  is not a full RFC 5322 parser.
- Management endpoints intentionally use regular OpenAuth JSON errors, while
  SCIM protocol endpoints return RFC 7644-compatible SCIM errors. This matches
  the crate README and current tests.
- SCIM Groups and Bulk depend on OpenAuth organization/team schema availability
  for organization-scoped provisioning.

## Proposed Fixes

None.

No meaningful upstream parity gap, missing validation, request or response
shape mismatch, security boundary issue, public API problem, feature gate issue,
or missing regression test was found that justifies changing Rust code in this
audit.

## Server-Only Parity Estimate

**Estimated parity: 98%.**

Supported upstream server-only behavior:

- SCIM plugin registration, schema contribution, and version identity.
- Token management endpoints and access control.
- Static and persisted SCIM provider authentication.
- Token storage modes and generated token wire format.
- User provisioning, listing, filtering, reading, replacing, patching, and
  deletion.
- Provider and organization isolation for SCIM user resources.
- SCIM metadata endpoints for ServiceProviderConfig, Schemas, and
  ResourceTypes.
- SCIM error response bodies for protocol routes.
- Upstream regression coverage from `scim.test.ts`, `scim-users.test.ts`,
  `scim-patch.test.ts`, and `scim.management.test.ts`.

No missing upstream server-side feature was found.

Remaining non-100% items:

- Upstream TypeScript exposes `scim(options?)`; Rust exposes
  `scim(ScimOptions)` with `ScimOptions::default()`, because Rust has no
  optional function arguments.
- Management endpoints use OpenAuth's normal JSON error envelope, currently
  including a stable `code` field in addition to `message`.
- `ServiceProviderConfig` advertises OpenAuth's implemented Bulk, sort, and
  ETag support, while upstream 1.6.9 reports those capabilities as unsupported.
- Upstream's TypeScript client plugin inference (`src/client.ts`) is not
  ported because this crate is server-side only.

These are not planned fixes unless the project chooses strict upstream wire
shape over existing OpenAuth conventions and implemented SCIM capabilities.

## Tests To Add Or Update

None.

Existing focused coverage already includes:

- Token format, decoding, hashing, storage modes, and default providers.
- Management session, ownership, organization role, hook, list, get, delete,
  and token rotation behavior.
- SCIM bearer authentication failures and org-scope hardening.
- Metadata route shapes and schema/resource type lookups.
- User create, list, get, PUT, PATCH, DELETE, filter, projection, pagination,
  ETag, extension attribute, email validation, and provider/org isolation.
- Group, Bulk, and search behavior for OpenAuth's broader SCIM 2.0 surface.
- Adapter migration and provider persistence contracts.

## Items Intentionally Left Unchanged

- `crates/openauth-scim/src/**`
- `crates/openauth-scim/tests/**`
- `crates/openauth-scim/README.md`
- Public re-exports and feature flags.
- Schema contributions for SCIM providers, user profiles, and group profiles.
- OpenAuth-only Groups, Bulk, search, pagination, projection, sorting, ETags,
  and extension/profile support.
- Dependency set.

## Verification

Baseline verification before saving this audit:

```bash
cargo nextest run -p openauth-scim
```

Result: `135` tests passed.

Required verification after saving this audit:

```bash
cargo fmt --all --check
cargo clippy -p openauth-scim --all-targets -- -D warnings
cargo nextest run -p openauth-scim
```
