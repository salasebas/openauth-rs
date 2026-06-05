# Social providers upstream parity

Full notes: [README.md § Upstream parity](./README.md#upstream-parity-better-auth-169).

Upstream: `@better-auth/core/social-providers` @ 1.6.9 (**35** providers).

**Wire:** 33/35 full; Facebook opaque-token verify and Twitch JWKS are stricter than
upstream. **Hooks:** only 10/35 expose typed Rust overrides vs global upstream
`ProviderOptions` callbacks. **Tests:** 310 crate tests vs 0 upstream unit tests in
the provider folder; E2E flows are owned by `openauth-core`.
