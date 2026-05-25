# Server-Side Telemetry Parity

This crate tracks Better Auth 1.6.9 server-side telemetry behavior while keeping OpenAuth Rust-native, explicit, and opt-in by deployment.

## Current Parity

Estimated server-only parity: **93%**.

Implemented:

- `create_telemetry` enablement matches upstream: option-side enablement or env-side enablement, suppressed in test environments unless explicitly skipped.
- `OPENAUTH_TELEMETRY`, `OPENAUTH_TELEMETRY_DEBUG`, and `OPENAUTH_TELEMETRY_ENDPOINT` provide the OpenAuth-prefixed equivalent of upstream `BETTER_AUTH_*` variables.
- No endpoint and no custom sink creates a hard no-op publisher.
- Init events contain config, runtime, database, framework, environment, system info, and package manager payloads.
- Explicit `publish` calls preserve event type and payload while adding the anonymous project id.
- Auth config telemetry includes the OpenAuth-modeled email/password, verification, password reset, session, account, rate limit, secondary storage, plugin, additional field, and OAuth social provider metadata.
- Provider secrets, client secrets, and callback internals are never serialized.
- Root `openauth` re-exports telemetry helpers only when the root `telemetry` feature is enabled.
- The OpenAuth CLI emits Better Auth-style `cli_generate` and `cli_migrate` events for successful, no-op, aborted, dry-run, unsupported-adapter, and unsupported-database paths when telemetry is enabled.

## Intentional Rust Differences

- Environment variables use the `OPENAUTH_*` prefix instead of upstream `BETTER_AUTH_*`.
- Publishing is opt-in to a configured endpoint or explicit `custom_track`; OpenAuth does not ship a maintainer-owned default collector.
- Runtime detection reports Rust/Cargo hosts instead of Node/Bun/Deno package metadata.
- Package and dependency versions come from Cargo manifests where available; JavaScript `package.json` and `node_modules` scanning are upstream-only.
- OpenAuth initialization is synchronous today, so the root `AuthContext` does not auto-create an async telemetry publisher. Applications can call `create_telemetry` directly, and the CLI wires it where async command execution already exists.
- CLI `dry_run` telemetry is OpenAuth-specific because the Rust CLI exposes a dry-run migration mode that upstream does not.
- Social provider telemetry reports stable trait option metadata only; it does not invoke providers or introspect callback bodies.

## Remaining Non-Parity

- Automatic server context wiring equivalent to upstream `ctx.publishTelemetry` is intentionally not implemented until OpenAuth exposes an async initialization boundary or a core-level telemetry sink contract.
- Node-only system fields such as exact CPU model, CPU speed, and total memory are left `null` without adding a platform system-information dependency.
- Framework detection is conservative and Cargo-based; it does not inspect web framework runtime state.
- Some Better Auth config branches stay `null` or `false` when OpenAuth does not yet model the corresponding option.

These gaps are either TypeScript/Node-specific or depend on larger OpenAuth API boundaries rather than missing telemetry package logic.
