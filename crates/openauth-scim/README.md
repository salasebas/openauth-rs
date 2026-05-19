# openauth-scim

SCIM support surface for OpenAuth-RS.

## Status

This package is in experimental beta and currently exposes the package surface
while SCIM behavior is being built out. Public APIs may change before stable
release.

## What It Provides

`openauth-scim` is reserved for System for Cross-domain Identity Management
support in OpenAuth-RS. The intended scope is server-side SCIM contracts,
schemas, endpoints, validation, and provisioning behavior.

## Example

```rust
let scim_crate_version = openauth_scim::VERSION;
```

Use this crate as the home for SCIM work rather than placing SCIM behavior in
the core crate.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
