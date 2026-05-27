# OIDC Upstream Parity Audit Plan

## Summary

Audit target is the `openauth-oidc` crate plus the `openauth-sso` OIDC routes
that expose its server-side behavior. Upstream reference is Better Auth
`packages/sso`, especially OIDC discovery, registration, sign-in, callback,
provider sanitization, and tests.

## Upstream Files Inspected

- `upstream/better-auth/1.6.9/repository/packages/sso/src/oidc/discovery.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/oidc/types.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/oidc/errors.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/oidc/discovery.test.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/oidc.test.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/routes/sso.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/routes/schemas.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/routes/providers.ts`
- `upstream/better-auth/1.6.9/repository/packages/sso/src/types.ts`

## OpenAuth Files Inspected

- `crates/openauth-oidc/src/lib.rs`
- `crates/openauth-oidc/src/options.rs`
- `crates/openauth-oidc/src/discovery.rs`
- `crates/openauth-oidc/src/flow.rs`
- `crates/openauth-oidc/src/utils.rs`
- `crates/openauth-oidc/tests/flow.rs`
- `crates/openauth-sso/src/routes/registration.rs`
- `crates/openauth-sso/src/routes/sign_in.rs`
- `crates/openauth-sso/src/routes/oidc.rs`
- `crates/openauth-sso/src/routes/providers.rs`
- `crates/openauth-sso/src/routes/provider_update.rs`
- `crates/openauth-sso/src/options.rs`
- `crates/openauth-sso/src/store.rs`
- `crates/openauth-sso/src/openapi.rs`
- `crates/openauth-sso/src/schema.rs`
- `crates/openauth-sso/src/lib.rs`
- `crates/openauth-sso/tests/sso/endpoints/registration/**`
- `crates/openauth-sso/tests/sso/endpoints/sign_in/**`
- `crates/openauth-sso/tests/sso/endpoints/oidc_callback/**`

## Confirmed Matches

- Discovery URL computation preserves issuer paths and trims trailing slash.
- Discovery requires `issuer`, `authorization_endpoint`, `token_endpoint`, and
  `jwks_uri`.
- Discovery accepts optional UserInfo, scopes, token auth methods, revocation,
  end-session, introspection, and PKCE metadata.
- Token endpoint auth selection matches upstream: prefer
  `client_secret_basic`, then `client_secret_post`, default to basic.
- Existing config values override discovered endpoints.
- Discovered `scopes_supported` is metadata only; explicitly configured
  request scopes are preserved, and missing request scopes continue to use the
  SSO sign-in defaults.
- Registration supports `skipDiscovery`, manual endpoints, PKCE, mappings,
  scopes, and default override-user-info behavior.
- Sign-in sends `nonce`, OAuth state, login hint, default scopes, and supports
  shared/per-provider callback URLs.
- Callback supports UserInfo-first behavior, ID-token fallback, lowercase email
  normalization, explicit token auth modes, safe redirects, provider lookup from
  state/path, and organization provisioning hooks.
- Rust intentionally hardens several paths beyond upstream: explicit errors,
  redacted secret debug output, stricter safe redirect handling, optional strict
  manual endpoint origin validation, ID-token verification for fallback,
  multi-audience `azp` checks, and avoiding panics in production paths.

## Confirmed Differences

- Upstream `needsRuntimeDiscovery` returns true unless `authorizationEndpoint`,
  `tokenEndpoint`, and `jwksEndpoint` are all present. OpenAuth split sign-in
  and callback requirements and allowed callback without JWKS when UserInfo was
  configured.
- Because of that split, callback could avoid runtime discovery and complete
  with a partial manual config that upstream would attempt to hydrate first.
- OpenAuth also folded discovered `scopes_supported` into configured request
  scopes during registration and runtime hydration. Upstream exposes
  `scopesSupported` from discovery but persists only explicit
  `oidcConfig.scopes`, leaving sign-in to use default scopes when no scopes were
  configured.

## Proposed Fixes

- Update `crates/openauth-oidc/src/discovery.rs` so
  `OidcRuntimeRequirement::SignIn` and `OidcRuntimeRequirement::Callback` both
  require `authorization_endpoint`, `token_endpoint`, and `jwks_endpoint`.
- Keep `OidcRuntimeRequirement` for API compatibility, documenting that both
  current runtime modes share the upstream required endpoint set.
- Update callback tests to prove UserInfo plus missing JWKS triggers discovery,
  and that a missing discovery document redirects with the stable discovery
  error code.
- Preserve only explicit configured OIDC request scopes during registration and
  runtime discovery; do not convert discovered `scopes_supported` into
  configured `scopes`.

## Tests To Add Or Update

- Update `crates/openauth-oidc/src/discovery.rs` unit coverage so runtime
  discovery returns true for only `authorization_endpoint`, true for
  `token_endpoint` plus `user_info_endpoint` without JWKS, and false only when
  `authorization_endpoint`, `token_endpoint`, and `jwks_endpoint` are present.
- Add `openauth-sso` callback coverage for UserInfo plus missing JWKS:
  successful hydration when discovery supplies JWKS, and stable error redirect
  when discovery is unavailable.
- Adjust existing looser callback coverage to reflect upstream parity.
- Add regression coverage that discovered `scopes_supported` does not become
  stored or runtime request scopes, and that default SSO sign-in scopes are still
  used after runtime discovery.

## Intentionally Left Unchanged

- Keep Rust's stricter safe redirect handling and optional strict endpoint-origin
  validation as production-grade security hardening.
- Keep ID-token validation and `azp` checks in Rust even though upstream's
  UserInfo path is looser.
- Keep revocation, end-session, and introspection endpoint
  persistence/sanitization.
- Do not add dependencies.
- Do not change unrelated SAML, OAuth provider, adapter, or organization
  behavior.

## Risks

- Tightening runtime discovery can make partial manual OIDC configs fail earlier
  if they relied on UserInfo without JWKS and had no working discovery endpoint.
  This matches upstream behavior, but it is a compatibility change for existing
  OpenAuth users.
- Discovery network calls may happen in a few more cases. Existing timeout and
  error mapping limit blast radius.
