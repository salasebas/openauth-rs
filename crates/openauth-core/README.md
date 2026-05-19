# openauth-core

Core types and primitives for OpenAuth-RS.

## Status

This package is in experimental beta. Contracts for adapters, plugins, options,
and HTTP routing may change before stable release.

## What It Provides

`openauth-core` contains the shared server contracts used by the rest of the
workspace: API requests and responses, auth context, cookies, crypto helpers,
database adapter traits, schemas, errors, options, plugins, sessions, users,
verification storage, and rate limiting.

## Example

```rust
use openauth_core::db::{auth_schema, AuthSchemaOptions};

let schema = auth_schema(AuthSchemaOptions::default());
let user_table = schema.table_name("user")?;
```

Application code usually depends on `openauth`; adapter, plugin, and
integration crates use `openauth-core` for stable internal contracts.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
