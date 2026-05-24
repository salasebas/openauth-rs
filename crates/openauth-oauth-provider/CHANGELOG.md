# Changelog

All notable changes to `openauth-oauth-provider` are documented in this file.

## [0.0.6] - 2026-05-24

### Added

- Added OAuth provider endpoint modules for authorization, clients, consent,
  introspection, logout, metadata, token, and userinfo behavior.
- Added typed provider options and token claim/introspection/type modules.
- Added expanded authorization, client, consent, metadata, OIDC, and token
  coverage mapped against upstream behavior.

### Changed

- Split the provider endpoint and token implementations into focused modules.
- Aligned OAuth provider server behavior more closely with upstream Better Auth
  semantics while keeping Rust-owned APIs.

## [0.0.5] - 2026-05-19

### Added

- Published the beta OAuth provider release line.

