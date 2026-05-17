# Passkey Server Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an idiomatic server-only OpenAuth passkey plugin backed by `webauthn-rs`, with Better Auth-compatible behavior, Rust-shaped APIs, schema contribution, endpoint coverage, and focused tests.

**Architecture:** The new `openauth-passkey` crate owns passkey options, schema, WebAuthn backend integration, challenge persistence, endpoint handlers, response/cookie helpers, and storage mapping. The top-level `openauth` crate exposes it only behind an optional `passkey` feature so unrelated users do not pay the dependency cost. Challenge state stays server-side in `verification` and is referenced through a signed short-lived cookie.

**Tech Stack:** Rust, OpenAuth plugin/router/schema APIs, `webauthn-rs`, `uuid`, `serde_json`, OpenAuth `DbAdapter`, SQLite-backed migration tests through `openauth-sqlx`.

---

## File Structure

- `Cargo.toml`: workspace member/dependency registration for `openauth-passkey`, `webauthn-rs`, and `uuid`.
- `Cargo.lock`: resolved dependency graph for the new crate.
- `README.md`: public usage notes for passkeys.
- `crates/openauth/Cargo.toml`: optional `passkey` feature and dependency.
- `crates/openauth/src/lib.rs`: optional re-export as `openauth::passkey`.
- `crates/openauth/tests/public_api.rs`: public API feature re-export test.
- `crates/openauth-passkey/Cargo.toml`: crate metadata, dependencies, dev-dependencies.
- `crates/openauth-passkey/README.md`: crate-level usage and persistence notes.
- `crates/openauth-passkey/src/lib.rs`: plugin entrypoint and public exports.
- `crates/openauth-passkey/src/options.rs`: public builder-style options and callbacks.
- `crates/openauth-passkey/src/errors.rs`: plugin error code registration.
- `crates/openauth-passkey/src/schema.rs`: logical `passkey` model mapped to physical `passkeys`.
- `crates/openauth-passkey/src/store.rs`: adapter-backed passkey persistence.
- `crates/openauth-passkey/src/webauthn.rs`: real and trait-based WebAuthn backend integration.
- `crates/openauth-passkey/src/challenge.rs`: server-side challenge state persistence.
- `crates/openauth-passkey/src/cookies.rs`: challenge cookie signing and extraction.
- `crates/openauth-passkey/src/response.rs`: JSON response and cookie serialization helpers.
- `crates/openauth-passkey/src/openapi.rs`: request body and OpenAPI schema helpers.
- `crates/openauth-passkey/src/session.rs`: session lookup and passkey session creation.
- `crates/openauth-passkey/src/routes.rs`: passkey HTTP endpoint registration and route handlers.
- `crates/openauth-passkey/tests/passkey.rs`: integration test module registry.
- `crates/openauth-passkey/tests/passkey/schema.rs`: schema contribution tests.
- `crates/openauth-passkey/tests/passkey/sqlite.rs`: SQLite migration/schema tests.
- `crates/openauth-passkey/tests/passkey/openapi.rs`: endpoint metadata and body schema tests.
- `crates/openauth-passkey/tests/passkey/support.rs`: shared test router, seeds, fake backend, and request helpers.
- `crates/openauth-passkey/tests/passkey/register.rs`: registration option and verification behavior tests.
- `crates/openauth-passkey/tests/passkey/authenticate.rs`: authentication option and verification behavior tests.
- `crates/openauth-passkey/tests/passkey/management.rs`: list/update/delete and ownership tests.

---

## Checklist

### Task 1: Create Passkey Crate And Re-export

**Files:**
- Create: `crates/openauth-passkey/Cargo.toml`
- Create: `crates/openauth-passkey/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `crates/openauth/Cargo.toml`
- Modify: `crates/openauth/src/lib.rs`
- Test: `crates/openauth/tests/public_api.rs`

- [x] **Step 1: Add crate to the workspace**

Add `crates/openauth-passkey` to `[workspace].members` and add a workspace dependency:

```toml
openauth-passkey = { path = "crates/openauth-passkey", version = "0.0.3" }
uuid = { version = "1", features = ["serde", "v4", "v5"] }
webauthn-rs = { version = "0.5", features = ["conditional-ui", "danger-allow-state-serialisation", "danger-credential-internals"] }
```

- [x] **Step 2: Add the optional top-level feature**

In `crates/openauth/Cargo.toml`:

```toml
openauth-passkey = { workspace = true, optional = true }

[features]
passkey = ["dep:openauth-passkey"]
```

In `crates/openauth/src/lib.rs`:

```rust
#[cfg(feature = "passkey")]
pub use openauth_passkey as passkey;
```

- [x] **Step 3: Add public API coverage**

```rust
#[cfg(feature = "passkey")]
#[test]
fn passkey_feature_reexports_passkey_crate() {
    let plugin = openauth::passkey::passkey(openauth::passkey::PasskeyOptions::default());

    assert_eq!(plugin.id, "passkey");
}
```

- [x] **Step 4: Verify**

Run:

```bash
cargo test -p openauth --features passkey
```

Expected: pass.

### Task 2: Implement Schema And Store

**Files:**
- Create: `crates/openauth-passkey/src/schema.rs`
- Create: `crates/openauth-passkey/src/store.rs`
- Test: `crates/openauth-passkey/tests/passkey/schema.rs`
- Test: `crates/openauth-passkey/tests/passkey/sqlite.rs`

- [x] **Step 1: Add schema contribution**

The plugin contributes logical model `passkey`, physical table `passkeys`, and fields:

```text
id, name, public_key, user_id, credential_id, counter, device_type,
backed_up, transports, created_at, aaguid, webauthn_credential
```

`user_id` references `users.id` with cascade delete. `user_id` and `credential_id` are indexed. `webauthn_credential` is hidden and JSON typed.

- [x] **Step 2: Add adapter-backed store**

`PasskeyStore` implements `list_by_user`, `find_by_id`, `find_by_credential_id`, `create`, `update_name_for_user`, `update_after_authentication`, and `delete_for_user`.

- [x] **Step 3: Verify schema and SQLite migration**

Run:

```bash
cargo test -p openauth-passkey passkey_plugin_registers_snake_case_plural_schema
cargo test -p openauth-passkey sqlite_schema_migration_creates_passkeys_table_and_columns
```

Expected: both pass and SQLite columns stay snake_case.

### Task 3: Implement WebAuthn And Challenge Flow

**Files:**
- Create: `crates/openauth-passkey/src/webauthn.rs`
- Create: `crates/openauth-passkey/src/challenge.rs`
- Create: `crates/openauth-passkey/src/cookies.rs`
- Create: `crates/openauth-passkey/src/session.rs`
- Test: `crates/openauth-passkey/tests/passkey/register.rs`
- Test: `crates/openauth-passkey/tests/passkey/authenticate.rs`

- [x] **Step 1: Add backend trait and real backend**

Expose `PasskeyWebAuthnBackend` for tests and use `RealPasskeyWebAuthnBackend` by default. The real backend uses `webauthn-rs` registration/authentication state and serializes it only into server-side verification storage.

- [x] **Step 2: Persist challenge state**

Use `verification.identifier` for a random challenge token and `verification.value` for serialized challenge state:

```rust
ChallengeValue {
    kind: ChallengeKind::Registration,
    state,
    user: Some(user),
    context,
}
```

The signed cookie name defaults to `better-auth-passkey` and max age is `300` seconds.

- [x] **Step 3: Verify real option shape**

Run:

```bash
cargo test -p openauth-passkey real_webauthn_backend_generates_registration_option_shape
```

Expected: pass with non-empty WebAuthn challenge and `rp.id`.

### Task 4: Implement HTTP Endpoints

**Files:**
- Create: `crates/openauth-passkey/src/routes.rs`
- Create: `crates/openauth-passkey/src/openapi.rs`
- Create: `crates/openauth-passkey/src/response.rs`
- Test: `crates/openauth-passkey/tests/passkey/openapi.rs`
- Test: `crates/openauth-passkey/tests/passkey/register.rs`
- Test: `crates/openauth-passkey/tests/passkey/authenticate.rs`
- Test: `crates/openauth-passkey/tests/passkey/management.rs`

- [x] **Step 1: Add endpoint registry**

Register exactly:

```text
GET  /passkey/generate-register-options
GET  /passkey/generate-authenticate-options
POST /passkey/verify-registration
POST /passkey/verify-authentication
GET  /passkey/list-user-passkeys
POST /passkey/delete-passkey
POST /passkey/update-passkey
```

- [x] **Step 2: Add registration behavior**

Default registration requires a session. Pre-auth registration requires `require_session(false)` and a valid `resolve_user`. `after_verification` may override the target user only when no authenticated session is being mismatched.

- [x] **Step 3: Add authentication behavior**

Successful authentication updates counter/state, creates an OpenAuth session, sets session cookies, returns `{ session, user }`, and deletes the challenge.

- [x] **Step 4: Add ownership protections**

Update/delete require ownership. Authentication generated under an existing session rejects a credential for a different user before creating a session.

- [x] **Step 5: Verify**

Run:

```bash
cargo test -p openauth-passkey
```

Expected: all passkey endpoint tests pass.

### Task 5: Split Large Passkey Tests

**Files:**
- Modify: `crates/openauth-passkey/tests/passkey.rs`
- Create: `crates/openauth-passkey/tests/passkey/support.rs`
- Create: `crates/openauth-passkey/tests/passkey/register.rs`
- Create: `crates/openauth-passkey/tests/passkey/authenticate.rs`
- Create: `crates/openauth-passkey/tests/passkey/management.rs`
- Delete: `crates/openauth-passkey/tests/passkey/options.rs`

- [x] **Step 1: Move shared helpers into support**

Move router creation, request helpers, cookie helpers, user/passkey seeding, and `FakeWebAuthnBackend` into `support.rs`. Export only the helpers used by tests:

```rust
pub async fn seeded_router(options: PasskeyOptions) -> Result<(Arc<MemoryAdapter>, AuthRouter, Arc<FakeWebAuthnBackend>), Box<dyn std::error::Error>>;
pub fn empty_request(method: Method, path: &str, cookie: Option<&str>) -> Result<Request<Vec<u8>>, http::Error>;
pub fn json_request(method: Method, path: &str, body: &str, cookie: Option<&str>) -> Result<Request<Vec<u8>>, http::Error>;
pub fn set_cookie_values(response: &http::Response<Vec<u8>>) -> Vec<String>;
pub fn cookie_header_from_response(response: &http::Response<Vec<u8>>) -> String;
pub fn join_cookies(values: &[&str]) -> String;
pub async fn sign_in_cookie(router: &AuthRouter) -> Result<String, Box<dyn std::error::Error>>;
pub async fn session_cookie_for(adapter: &MemoryAdapter, user_id: &str, token: &str) -> Result<String, Box<dyn std::error::Error>>;
pub async fn seed_user_two(adapter: &MemoryAdapter) -> Result<(), Box<dyn std::error::Error>>;
pub async fn seed_passkey(adapter: &MemoryAdapter, id: &str, user_id: &str, name: &str, credential_id: &str) -> Result<(), Box<dyn std::error::Error>>;
```

- [x] **Step 2: Split registration tests**

Move these tests to `register.rs`:

```text
generate_register_options_requires_session_by_default
generate_register_options_uses_resolve_user_without_session
generate_register_options_requires_resolve_user_in_preauth_mode
generate_register_options_rejects_invalid_resolved_user
real_webauthn_backend_generates_registration_option_shape
verify_registration_creates_passkey_and_deletes_challenge
after_registration_verification_can_override_preauth_user
after_registration_verification_cannot_override_session_user
```

- [x] **Step 3: Split authentication tests**

Move these tests to `authenticate.rs`:

```text
generate_authenticate_options_without_session_returns_discoverable_options
generate_authenticate_options_with_session_includes_user_credentials
verify_authentication_creates_session_and_returns_user
verify_authentication_rejects_credential_outside_session_challenge
```

- [x] **Step 4: Split management tests**

Move this test to `management.rs`:

```text
update_and_delete_require_passkey_ownership
```

- [x] **Step 5: Update module registry**

`tests/passkey.rs` should include:

```rust
#[path = "passkey/authenticate.rs"]
mod authenticate;
#[path = "passkey/management.rs"]
mod management;
#[path = "passkey/openapi.rs"]
mod openapi;
#[path = "passkey/register.rs"]
mod register;
#[path = "passkey/schema.rs"]
mod schema;
#[path = "passkey/sqlite.rs"]
mod sqlite;
#[path = "passkey/support.rs"]
mod support;
```

- [x] **Step 6: Verify**

Run:

```bash
cargo test -p openauth-passkey
cargo clippy -p openauth-passkey --all-targets -- -D warnings
```

Expected: all pass, and no single passkey test file remains around 800 lines.

### Task 6: Final Verification And Docs Check

**Files:**
- Modify if needed: `README.md`
- Modify if needed: `crates/openauth-passkey/README.md`
- Modify if needed: `docs/superpowers/plans/2026-05-17-passkey-server-plugin.md`

- [x] **Step 1: Run final verification**

Run:

```bash
cargo fmt --all --check
cargo test -p openauth-passkey
cargo clippy -p openauth-passkey --all-targets -- -D warnings
cargo test -p openauth --features passkey
cargo clippy -p openauth --features passkey --all-targets -- -D warnings
cargo test -p openauth-core schema
```

Expected: all pass.

- [x] **Step 2: Mark this plan complete**

Update every completed checkbox in this file from `- [ ]` to `- [x]`.

### Task 7: Upstream Server Parity Follow-ups

**Files:**
- Modify: `crates/openauth-passkey/src/options.rs`
- Modify: `crates/openauth-passkey/src/routes.rs`
- Modify: `crates/openauth-passkey/src/webauthn.rs`
- Modify: `crates/openauth-passkey/tests/passkey/register.rs`
- Modify: `crates/openauth-passkey/tests/passkey/authenticate.rs`
- Modify: `crates/openauth-passkey/tests/passkey/support.rs`
- Modify: `crates/openauth-passkey/README.md`
- Modify: `README.md`

- [x] **Step 1: Record applicable upstream differences**

Server-side Better Auth passkey behavior that still applies after excluding browser/client SDK code:

```text
1. GET /passkey/generate-register-options query `name` changes the WebAuthn userName sent to the browser.
2. Registration options include authenticatorSelection defaults:
   residentKey = "preferred", userVerification = "preferred".
3. Registration query `authenticatorAttachment` overrides authenticatorSelection.authenticatorAttachment.
4. Global PasskeyOptions supports authenticatorSelection defaults.
5. Registration and authentication extensions are included in generated WebAuthn option JSON.
6. Authentication options include userVerification = "preferred".
7. Verification requires an origin in Better Auth. OpenAuth keeps base_url/localhost fallback for server ergonomics; document the difference because `webauthn-rs` still validates against configured origins.
```

Non-applicable items:

```text
1. passkeyClient/browser SDK behavior remains out of server-only scope.
2. TypeScript schema override syntax is not ported 1:1; OpenAuth uses Rust plugin schema contributions and database overrides.
3. Raw SimpleWebAuthn verification parameters are not ported; OpenAuth keeps `webauthn-rs` as the verifier.
```

- [x] **Step 2: Add failing tests for register option parity**

Add tests proving `name`, `authenticatorAttachment`, `authenticatorSelection`, and registration `extensions` appear in generated registration options.

- [x] **Step 3: Add failing tests for authentication option parity**

Add tests proving authentication `extensions` appear in generated authentication options and `userVerification` is `preferred`.

- [x] **Step 4: Implement minimal option plumbing**

Add Rust-shaped option types for authenticator selection and pass them through the backend trait. Merge global defaults, query override, and extensions into generated JSON options.

- [x] **Step 5: Document the verifier boundary**

Document that option-shape parity is supported while cryptographic verification remains delegated to `webauthn-rs`.

- [x] **Step 6: Verify**

Run:

```bash
cargo fmt --all --check
cargo test -p openauth-passkey
cargo clippy -p openauth-passkey --all-targets -- -D warnings
cargo test -p openauth --features passkey
```

Expected: all pass.

---

## Self-Review

- [x] **Spec coverage:** The plan covers crate creation, optional re-export, `webauthn-rs`, schema/table naming, all requested endpoints, pre-auth registration, `after_verification`, session creation, challenge persistence, management endpoints, ownership checks, docs, and tests.
- [x] **Placeholder scan:** No `TBD`, `TODO`, or unspecified test/implementation placeholders remain.
- [x] **Type consistency:** Public names match the implementation: `PasskeyOptions`, `passkey(options)`, `PasskeyWebAuthnBackend`, `PasskeyRegistrationUser`, `VerifiedPasskeyCredential`, and `VerifiedAuthentication`.
