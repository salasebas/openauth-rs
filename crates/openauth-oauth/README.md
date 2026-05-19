# openauth-oauth

OAuth client primitives for OpenAuth-RS.

## Status

This package is in experimental beta. Request builders, provider contracts, and
token validation helpers may change before stable release.

## What It Provides

`openauth-oauth` contains OAuth 2.0/OIDC client-side server primitives used by
social providers and OpenAuth core: authorization URL creation, authorization
code exchange requests, refresh requests, token parsing, JWKS helpers, PKCE,
and provider contracts.

## Example

```rust
use openauth_oauth::oauth2::generate_code_challenge;

let challenge = generate_code_challenge("a-long-random-code-verifier")?;
```

Most applications will consume this indirectly through `openauth` or
`openauth-social-providers`; provider authors can use it directly.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
