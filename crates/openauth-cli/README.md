# openauth-cli

Command-line tools for OpenAuth-RS.

## Status

This package is in experimental beta. Commands, flags, generated output, and
workspace detection may change before stable release.

## What It Provides

`openauth-cli` provides local tooling for project setup, diagnostics, secret
generation, schema printing, migration planning, and official plugin changes.
The binary is exposed as `openauth` and also supports cargo-style aliases.

## Example

```sh
openauth secret --bytes 32
openauth doctor --production
openauth schema print --dialect sqlite
openauth plugins list
```

The CLI is designed to inspect the current workspace and generate OpenAuth-RS
configuration or database output without hiding the Rust code it affects.

## Links

- [Root README](../../README.md)
- [Repository](https://github.com/sebasxsala/openauth-rs)
