# OpenAuth SSO Upstream Gap Analysis

This file tracks material differences between the current
`crates/openauth-sso` implementation and the upstream server-side Better Auth
SSO package at `upstream/better-auth/1.6.9/repository/packages/sso`.

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
| SAML sign-in | Implemented | Unsigned Redirect AuthnRequest works by default; signed Redirect AuthnRequest is available behind `saml-signed`. |
| SAML ACS | Partial | ACS parses response XML for `InResponseTo` state when `RelayState` is absent, validates provider match, timestamps, algorithms, direct assertion placement, and replay until assertion expiration. XMLDSig validation and encrypted assertion decryption remain fail-closed. |
| SAML signature validation | Not production-ready | Signature parsing and policy gates exist, but XMLDSig verification and outbound signing are not implemented in the default backend. Signed paths return stable fail-closed errors. |
| SLO | Partial | Local logout, sign-out cleanup hook, SP/IdP initiated parsing, Redirect/POST bindings, and state-preservation failure cases are covered. Signed SLO messages still require XMLDSig backend work. |
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
- SAML XML validation uses OpenAuth's parser boundary, local-name traversal,
  DOCTYPE rejection, algorithm inspection, and XMLDSig feature boundary rather
  than embedding `samlify`. Upstream configures `samlify` with
  `fast-xml-parser` well-formed XML validation, not full SAML XSD validation.
- `saml-signed` keeps native XML tooling optional. Default builds reject signed
  SAML paths that require unavailable signature validation instead of silently
  accepting them.

## Remaining Work

The main remaining parity gap is SAML cryptography. Do not mark SAML
production-ready until these are implemented with an auditable backend and
negative tests:

- XMLDSig verification for SAML POST and Redirect bindings across responses,
  assertions, LogoutRequest, and LogoutResponse.
- Outbound AuthnRequest and SLO signing when policy requires it.
- EncryptedAssertion decryption, or a documented permanent non-parity decision.
- Real IdP interoperability matrix for Okta, Azure/Entra ID, Google, and at
  least one strict SAML implementation.

Other intentional differences remain:

- Better Auth client/browser helpers are out of scope for the server-first Rust
  crate.
- Full SAML XSD validation is not embedded because upstream does not perform
  full XSD validation either. OpenAuth uses a fail-closed parser, local-name
  traversal, DOCTYPE rejection, algorithm inspection, and XMLDSig signature
  boundary instead.

Continue adding upstream parity cases when new Better Auth SSO behavior lands
under `upstream/better-auth`.

## Recommended Review Order

1. Keep the upstream checklist and this gap file updated when SSO behavior
   changes.
2. Prefer security-sensitive test additions first: SAML state, signatures,
   replay, redirects, token handling, and organization access.
3. Expand OpenAPI response metadata after the endpoint contracts settle.
