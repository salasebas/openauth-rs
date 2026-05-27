# Access, Additional Fields, Admin, Anonymous Upstream Parity Plan

## Summary

Audit target `acess` as upstream/OpenAuth `access`, plus `additional_fields`, `admin`, and `anonymous` in `openauth-plugins`. Make only the justified admin security/parity fixes identified by the audit.

## Files Inspected

Upstream:
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/plugins/access/*`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/plugins/additional-fields/*`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/plugins/admin/*`
- `upstream/better-auth/1.6.9/repository/packages/better-auth/src/plugins/anonymous/*`

OpenAuth:
- `crates/openauth-plugins/src/access/*`, `tests/access/mod.rs`
- `crates/openauth-plugins/src/additional_fields/mod.rs`, `tests/additional_fields/mod.rs`
- `crates/openauth-plugins/src/admin/*`, `tests/admin/*`
- `crates/openauth-plugins/src/anonymous/*`, `tests/anonymous/*`
- Supporting core additional-field/session output code in `crates/openauth-core/src/api/additional_fields.rs`, `crates/openauth-core/src/cookies/session.rs`, and related route tests.

## Confirmed Matches

- `access`: resource/action authorization, AND/OR connectors, per-resource OR, default admin access-control statements, and unknown-resource handling are covered and behaviorally aligned.
- `admin`: endpoint set, default roles, admin-user-id bypass, multi-role parsing, role allow-list validation, list/search/filter/sort shapes, ban/unban, remove-user self-protection, impersonate-admin checks, password length checks, and error-code exports are implemented.
- `anonymous`: anonymous sign-in, custom/async email and name generation, invalid email rejection, duplicate anonymous sign-in rejection, delete endpoint, link-account callback, cleanup safeguards, custom physical field name, and additional-field defaults are implemented.
- `additional_fields`: static user/session additional fields, input/returned/generated flags, `db_name`, default values, sign-up/update/get-session integration, and schema registration are implemented through OpenAuth core.

## Confirmed Differences And Proposed Fixes

- Fix admin impersonation restore invariant:
  - Upstream `stopImpersonating` verifies the stored admin session belongs to `session.impersonatedBy`.
  - OpenAuth currently restores any valid signed `admin_session` cookie without checking `admin_session.user_id == current_session.impersonated_by`.
  - Add this check in `crates/openauth-plugins/src/admin/handlers/sessions.rs`; return the existing internal admin-session error if it fails.
- Preserve `dont_remember` state through admin impersonation:
  - Upstream stores the original admin session token plus the `dont_remember` cookie value in `admin_session`.
  - OpenAuth writes the admin cookie with `None` and ignores the stored flag on restore.
  - Add a small helper in `crates/openauth-plugins/src/admin/cookies.rs` to read the signed `dont_remember` cookie from the request.
  - In `impersonate_user`, store that value in `set_admin_cookie`.
  - In `stop_impersonating`, use the parsed stored flag when restoring the admin session cookie via a new helper that accepts `SessionCookieOptions { dont_remember: stored_flag.is_some(), ..Default::default() }`.
- Do not change the stronger Rust-only admin create-user reserved-field validation:
  - Upstream spreads `data` after `role`, which can allow reserved-field overriding.
  - OpenAuth rejects reserved custom data fields; keep this as a deliberate server-side hardening.
- Do not add dynamic function defaults for `additional_fields` in this pass:
  - Upstream supports function-valued defaults.
  - OpenAuth supports static `DbValue` defaults and the behavior is owned by `openauth-core`, not just the target plugin.
  - Treat dynamic defaults as a separate cross-crate feature proposal.
- Do not change `access` empty-request behavior:
  - Upstream returns a generic failed authorization.
  - OpenAuth returns explicit `AccessError::EmptyRequest`, which is safer and already tested.

## Tests To Add Or Update

- Add admin regression tests in `crates/openauth-plugins/tests/admin/parity.rs`:
  - `stop_impersonating_rejects_admin_cookie_for_different_user`: create an impersonated session with `impersonated_by = admin_a`, provide a signed `admin_session` cookie for `admin_b`, call `/admin/stop-impersonating`, expect internal error and no restoration to `admin_b`.
  - `impersonation_preserves_dont_remember_cookie_state`: sign in or seed an admin session with a signed `dont_remember` cookie, impersonate, assert the `admin_session` cookie payload includes the stored marker, stop impersonating, assert restored session cookie is emitted with `dont_remember` semantics.
- Keep existing access/additional-fields/anonymous tests unchanged unless they fail after the admin cookie helper refactor.

## Verification

Run scoped verification for the modified surface:

```bash
cargo fmt --all --check
cargo clippy -p openauth-plugins --all-targets -- -D warnings
cargo nextest run -p openauth-plugins admin
```

If the cookie helper change requires touching `openauth-core` public APIs, broaden verification to the affected crate:

```bash
cargo clippy -p openauth-core --all-targets -- -D warnings
cargo nextest run -p openauth-core
```

## Risks And Assumptions

- Assumption: `acess` means the `access` plugin.
- Assumption: admin impersonation cookie restoration should preserve upstream's original-session ownership invariant and `dont_remember` behavior.
- Risk: exact `dont_remember` cookie semantics may differ slightly from Better Auth's boolean argument naming, so tests should assert OpenAuth's observable cookie/session behavior rather than TypeScript internals.
- Remaining follow-up: dynamic additional-field defaults need a separate `openauth-core` API design because current additional-field structs are cloneable value metadata, not async callback containers.
