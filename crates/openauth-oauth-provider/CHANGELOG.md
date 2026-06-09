# Changelog

All notable changes to `openauth-oauth-provider` are documented in this file.

## Unreleased

## [0.1.0] - 2026-06-08




### Added

- `McpOptions` on `OAuthProviderOptions` for MCP protected-resource metadata
  (`GET /.well-known/oauth-protected-resource`) and authorization-server metadata
  overrides.
- `mcp-client` feature with `McpAuthClient` resource-server helpers (token
  verification via `/oauth2/introspect`).
- Integration tests in `tests/oauth_provider/mcp_metadata.rs` and `mcp_client.rs`.

### Changed

- MCP is a profile on the existing OAuth 2.1/OIDC provider (single `/oauth2/*`
  surface). The removed `openauth-plugins::mcp` duplicate `/mcp/*` routes are not
  reintroduced here.

### Removed

- N/A at crate level (MCP plugin removal is documented in `openauth-plugins`).

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

