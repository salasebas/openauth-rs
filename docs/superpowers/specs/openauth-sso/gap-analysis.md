# OpenAuth SSO Upstream Gap Analysis

This file tracks material differences between the current
`crates/openauth-sso` implementation and the upstream server-side Better Auth
SSO package at `reference/upstream-src/1.6.9/repository/packages/sso`.

The goal is behavioral parity where it makes sense for OpenAuth. This remains
an idiomatic Rust implementation, not a line-by-line TypeScript port.

## Sources Compared

- Upstream:
  - `src/routes/sso.ts`
  - `src/routes/providers.ts`
  - `src/routes/domain-verification.ts`
  - `src/routes/saml-pipeline.ts`
  - `src/routes/helpers.ts`
  - `src/routes/schemas.ts`
  - `src/oidc/discovery.ts`
  - `src/saml/assertions.ts`
  - `src/saml/algorithms.ts`
  - `src/saml/timestamp.ts`
  - `src/linking/org-assignment.ts`
  - upstream tests under `src/**/*.test.ts`
- OpenAuth:
  - `crates/openauth-sso/src/options.rs`
  - `crates/openauth-sso/src/routes/*`
  - `crates/openauth-sso/src/oidc/*`
  - `crates/openauth-sso/src/saml/*`
  - `crates/openauth-sso/src/linking.rs`
  - `crates/openauth-sso/src/store.rs`
  - `crates/openauth-sso/tests/sso/*`

## Status Summary

| Area | Status | Notes |
| --- | --- | --- |
| Plugin/schema | Implemented | Physical DB table/fields follow OpenAuth naming rules. `SsoOptions::model_name` is honored by schema contribution and runtime provider storage. |
| Provider CRUD | Implemented | User-owned paths, organization admin/owner access, registration membership validation, and update `organizationId` membership validation are covered. |
| Provider sanitization | Implemented | OIDC secrets and raw SAML private keys/certificates stay out of read responses; derived certificate metadata is returned when parseable. |
| Registration | Implemented | OIDC/SAML config validation, dynamic `providersLimit`, register-time domain token return, optional OIDC endpoint persistence, and metadata size checks are covered. |
| OIDC discovery | Implemented | Registration-time and runtime discovery use stable error codes, trusted-origin validation, user endpoint preservation, runtime hydration for partial `skipDiscovery` configs, aggregate incomplete-field reporting, optional OP endpoint normalization, and opt-in strict validation for manual `skipDiscovery` endpoints. |
| OIDC sign-in/callback | Implemented | `defaultSSO`, `organizationSlug`, runtime discovery, ID-token-only profile extraction, state/path provider mix-up rejection, strict trust semantics, new-user redirects, provisioning callbacks, production-shaped Okta/Azure/Google endpoint and claim fixture tests, and default Basic token auth are covered. |
| Domain verification | Implemented | Secondary storage, DNS TXT verification, custom prefixes, URL/bare domains, multi-domain behavior, and org access checks are covered. |
| SAML metadata | Implemented | Generated and passthrough metadata, SLO bindings, NameID format, signing flags, and upstream-compatible `format=json` tolerance are covered. |
| SAML sign-in | Implemented | Redirect AuthnRequest via `opensaml`; unsigned by default, signed when `authnRequestsSigned` and SP private key are configured (`saml-signed` / `openauth-sso` `saml` feature). |
| SAML ACS | Implemented | ACS uses `opensaml` flow for signed/encrypted responses after OpenAuth pre-checks; unsigned responses use the local parser. InResponseTo state, replay, algorithms, and wrapping checks remain in OpenAuth. |
| SAML IdP interoperability | Implemented | Production-shaped Okta/Azure/Google provider configs (`tests/fixtures/saml/idp/*-shaped.json`), signed XMLDSig responses via `opensaml`, and ACS mapping tests in `provider_fixtures.rs` (no live IdP network calls). |
| SAML signature validation | Implemented | XMLDSig verify for Response/Assertion/Logout POST and Redirect bindings via `opensaml` + `bergshamra` (`saml-signed`). Fail-closed when crypto feature is disabled. |
| SLO | Implemented | SP/IdP-initiated logout, Redirect/POST bindings, outbound LogoutRequest/Response via `opensaml`, and signature verification for signed messages. |
| Organization assignment | Implemented | SSO login organization assignment and verified-domain non-SSO assignment run after session creation. |
| OpenAPI | Implemented | SSO route metadata, hidden browser/IdP routes, unique operation IDs, and public response schemas are covered. |
| Modularization | Implemented | Endpoint families and large SAML tests have been split into focused modules. |

## Closed High-Risk Gaps

- SAML ACS no longer trusts `RelayState` as the only SP-initiated state handle.
  When absent, it parses Response or SubjectConfirmation `InResponseTo` and
  loads the stored AuthnRequest from state storage.
- SAML ACS rejects unknown, expired, corrupt, or provider-mismatched
  AuthnRequest state before user provisioning, account linking, session
  creation, or replay-key writes.
- SAML ACS rejects assertion wrapping where `Assertion` or
  `EncryptedAssertion` is not a direct `Response` child, including single
  assertion wrappers under `Extensions`.
- SAML assertion replay keys now expire at `Assertion.NotOnOrAfter + clockSkew`
  when available. The 15 minute replay TTL is only a fallback for responses
  without a usable assertion expiration.
- `SamlOptions::request_ttl` now defaults to 5 minutes to match upstream's
  AuthnRequest/RelayState window.
- Provider update keeps `organizationId` support as an OpenAuth extension, but
  requires current-user membership in the target organization before persisting
  the change.
- OIDC callback defaults missing token endpoint authentication to
  `client_secret_basic`, matching Better Auth manual provider config behavior.
- OIDC callback rejects provider mix-up between the callback path provider and
  the provider captured in OAuth state.
- OIDC profiles now require a stable mapped ID, defaulting to `sub`, instead of
  silently falling back to email as the account ID.
- OIDC ID tokens require standard `exp`, `sub`, `aud`, and `iss` claims before
  profile extraction or session creation.
- OIDC email verification is only trusted when `SsoOptions::trust_email_verified`
  is enabled or a domain-verified provider match supplies a stronger trust
  signal.
- OIDC manual `skipDiscovery` endpoints can now be validated against OpenAuth
  trusted origins via `SsoOptions::strict_oidc_manual_endpoint_origins(true)`.
  The policy is opt-in because existing manual provider records may have been
  configured before this hardening existed.
- OIDC provider matrix tests cover production-shaped Okta, Azure/Entra ID, and
  Google issuer/authorization/token/userinfo/JWKS endpoint layouts without
  making external network calls.
- OIDC provider claim fixtures now cover Google hosted-domain/locale claims,
  Azure/Entra `oid`/`tid`/`preferred_username` mapping from UserInfo and ID
  token fallback, and Okta group/zoneinfo extra fields through the local mock
  OIDC server.
- OpenAPI uses distinct operation IDs for callback routes:
  `handleSSOCallbackShared` for `/sso/callback` and `handleSSOCallback` for
  `/sso/callback/:providerId`.
- OIDC discovery now normalizes, trusted-origin validates, persists, and
  sanitizes optional OP endpoints: `revocation_endpoint`,
  `end_session_endpoint`, and `introspection_endpoint`.
- Public SSO OpenAPI metadata now includes explicit success response schemas
  for provider CRUD, registration, sign-in, domain verification, SAML metadata,
  OIDC callback redirects, and SLO redirect/POST-form responses.
- Trust semantics, redirect safety, runtime OIDC discovery, partial
  `skipDiscovery` hydration, organization provider access, domain verification,
  SAML fail-closed signature/encryption paths, corrupt state rejection, direct
  assertion placement, and SLO cleanup are covered by Rust tests.

## Intentional Differences

- Better Auth's full custom SSO field-name mapping option is not exposed.
  OpenAuth supports a configurable logical provider model name and lets adapters
  own physical snake_case/plural storage.
- Client/browser SDK behavior is out of scope for this crate. Server endpoints
  are the public contract; future clients should be thin HTTP wrappers.
- SAML XML validation uses OpenAuth pre-checks (single assertion, wrapping placement,
  DOCTYPE rejection, algorithm policy) plus `opensaml` for crypto and extraction
  (samlify parity via the local `opensaml` crate, not embedded `samlify` npm).
- `saml-signed` enables `opensaml/crypto-bergshamra` for XMLDSig and XML-Enc.
  Builds without the feature reject signed/encrypted SAML paths fail-closed.

## Remaining Work

SAML cryptography and production-shaped IdP fixtures are implemented via `opensaml`
and `tests/fixtures/saml/idp/*-shaped.json` (see `provider_fixtures.rs`). Continue
hardening with:

- Optional full SAML XSD validation hook (upstream does not require this either).
- Live IdP sandbox smoke: `./scripts/saml-smoke.sh` (Phase 1 offline, always) and
  `SAML_SMOKE_LIVE=1 ./scripts/saml-smoke.sh` (Phase 2 preflight + browser checklist).
  See [SMOKE-SAML.md](../../../crates/openauth-sso/SMOKE-SAML.md). Not run in CI.

## CI / local test matrix

Run these from the workspace root:

| Command | Purpose |
| --- | --- |
| `cargo test -p openauth-sso --features saml,oidc -- saml` | Primary SAML regression (135 integration tests; always run in CI) |
| `cargo test -p openauth-sso --features oidc` | OIDC-only build; SAML routes not compiled |
| `cargo test -p openauth-saml --features saml-signed` | Unit + security tests for the SAML crate |
| `cargo test -p opensaml --features crypto-bergshamra` | Optional upstream crypto conformance for the pinned `opensaml` git rev |
| `./scripts/saml-smoke.sh` | Offline smoke (both crates); optional live preflight with `SAML_SMOKE_LIVE=1` |

`opensaml` is pinned in the workspace `Cargo.toml` by git rev (no local path
dependency). Refresh upstream parity sources with
`./scripts/fetch-upstream-better-auth.sh` before auditing
`reference/upstream-src/1.6.9/repository/packages/sso/src/saml.test.ts`.

## Upstream `saml.test.ts` parity (Better Auth 1.6.9)

Audited against `reference/upstream-src/1.6.9/repository/packages/sso/src/saml.test.ts`
(~108 `it` blocks). Status key: **Covered** = equivalent Rust test or enforced
behavior; **Partial** = subset or different surface; **Pending** = no equivalent yet.

| Upstream `describe` / theme | Status | Rust notes |
| --- | --- | --- |
| `defaultSSO` array fallback | Covered | `metadata_acs/default_sso.rs`: match by `providerId`, 404 for unknown id, precedence over DB; array order/fallback not duplicated |
| Signed AuthnRequests | Covered | `crypto.rs`, `dual_signed.rs` (redirect sig, private key required) |
| Unsigned AuthnRequests | Covered | `sign_in/saml.rs` omits `Signature`/`SigAlg` when `authnRequestsSigned` is false |
| `idpMetadata` without metadata XML | Covered | Registration + sign-in fallback tests |
| Core SAML SSO (register, metadata, sign-in, limits, linking) | Covered | `registration/*` incl. `saml_limits.rs` (limit + duplicate `providerId`); `metadata_acs/*`, `sign_in/*` |
| Production IdP fixtures (Okta/Azure/Google SAML) | Covered | `fixtures/saml/idp/*-shaped.json`, `provider_fixtures.rs`, `crypto.rs` |
| Custom fields / `safeJsonParse` | N/A | Server-only Rust crate; JSON parsing differs |
| Provider config parsing | Covered | Registration persistence / sanitization tests |
| IdP-initiated flow (GET after POST) | Covered | `flows.rs` POST ACS session + GET `/sso/saml2/callback`; `idp_initiated.rs` unsolicited ACS + `allow_idp_initiated` policy |
| Timestamp validation | Covered | Expired assertion e2e in `crypto.rs`; 1 ms boundaries in `openauth-saml` via `validate_saml_timestamp_at` |
| ACS origin bypass | Covered | ACS/SLO bypass origin; non-SSO routes remain protected |
| Response security (forged/tampered) | Covered | `crypto.rs` wrong cert, tampered sig; marker tests in helpers |
| Size limit constants | Covered | `DEFAULT_MAX_SAML_RESPONSE_SIZE` / `DEFAULT_MAX_SAML_METADATA_SIZE` in `options.rs`; `saml/constants.rs` |
| Assertion replay | Covered | `metadata_acs/state.rs` (ACS + callback POST replay), replay keys |
| Single assertion / XSW | Covered | `security.rs`, `crypto.rs` XSW HTTP test |
| Email lowercase normalization | Covered | ACS provisioning tests |
| Single Logout (SLO) | Covered | `slo/*`, `crypto.rs` signed POST + redirect; `logout_response.rs` `want_logout_response_signed` |
| `provisionUser` / `provisionUserOnEveryLogin` | Covered | `metadata_acs/provisioning.rs`; implicit link deny: `linking.rs` |
| InResponseTo opt-out | Covered | `idp_initiated.rs` `enable_in_response_to_validation: false` |
| Account linking trust | Covered | `linking.rs` denies implicit link for unverified/untrusted provider; allows when `account_linking.trusted_providers` includes SAML `providerId` |
| SAML SSO Hardening (ACS URL, provider lookup, registration validation) | Covered | `metadata_acs/*`, registration validation |

Remaining upstream-only parity (not security gaps):

| Item | Notes |
| --- | --- |
| `defaultSSO` array order/fallback | Sign-in from `default_sso` without DB record covered; strict array-index fallback not duplicated |
| Full IdP browser POST→GET mock | HTTP ACS + callback covered; upstream mock-IdP browser chain not replayed |
| RelayState cookie missing (cross-site POST) | OpenAuth uses session + GET callback; upstream cookie fallbacks not mirrored |
| Redirect loop (`callbackUrl` → callback route) | Not tested; misconfiguration edge |
| Better Auth client / `safeJsonParse` / callback naming | Out of scope for the server-first Rust crate |

These are documented differences vs Better Auth 1.6.9 `saml.test.ts`, not missing server implementation.

## Intentional differences (continued)

- Better Auth client/browser helpers are out of scope for the server-first Rust
  crate.
- Full SAML XSD validation is not embedded because upstream does not perform
  full XSD validation either. OpenAuth uses a fail-closed parser, local-name
  traversal, DOCTYPE rejection, algorithm inspection, and XMLDSig signature
  boundary instead.

Continue adding upstream parity cases when new Better Auth SSO behavior lands
under `reference/upstream-src/1.6.9/repository/`.

## Recommended Review Order

1. Keep the upstream checklist and this gap file updated when SSO behavior
   changes.
2. Prefer security-sensitive test additions first: SAML state, signatures,
   replay, redirects, token handling, and organization access.
3. Expand OpenAPI response metadata after the endpoint contracts settle.
