# Changelog

All notable changes to `openauth-sso` are documented in this file.

## [0.0.6] - 2026-05-24

### Added

- Added integration with the split `openauth-oidc` and `openauth-saml` crates.
- Added OIDC registration, discovery, callback, provider update, and sign-in
  coverage.
- Added SAML metadata/ACS state and security coverage.
- Added provider fixtures and additional endpoint error coverage.

### Changed

- Closed OIDC and SAML behavior gaps against the upstream reference.
- Updated provider registration, sign-in, callback, store, secret, OpenAPI, and
  schema handling.

## [0.0.5] - 2026-05-19

### Added

- Published the beta SSO release line.

