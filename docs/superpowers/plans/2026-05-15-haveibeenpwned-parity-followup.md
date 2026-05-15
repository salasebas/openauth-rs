# HaveIBeenPwned Parity Follow-Up Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining upstream/API parity gaps in the Have I Been Pwned plugin without adding speculative behavior.

**Architecture:** Keep the existing Rust route-level password validator pipeline. Add only a convenience constructor matching upstream's options-based creator shape, and move the sign-up validator after duplicate-email detection to match upstream's hash-time ordering more closely without introducing async password hashing.

**Tech Stack:** Rust workspace, `openauth-core`, `openauth-plugins`, `reqwest`, `sha1`, route-level async plugin validators.

---

## Findings From Re-Review

- [x] Public API gap: upstream exposes `haveIBeenPwned(options?)`; Rust now has `have_i_been_pwned_with_options(options)` so users can pass options without constructing a checker.
- [x] Sign-up precedence gap: upstream duplicate-email detection happens before password hashing and therefore before HIBP. Rust now checks duplicate email before `run_password_validators`, while keeping `EmailPasswordAuth::sign_up` as the race-safe guard.
- [x] Local `main` had advanced after the previous merge; it was merged before the follow-up implementation.
- [x] No gap found for default paths, runtime id, inventory id, SHA-1 split, k-anonymity privacy, headers, empty password skip, non-success HTTP message, transport failure message, custom compromised message, disabled option, or non-default path skipping.
- [x] No file-size issue found in HIBP implementation files: `checker.rs` 167 lines, `plugin.rs` 82 lines, test module 404 lines.

## Task 0: Merge Latest Local Main

**Files:**
- Conflict-dependent; preserve HIBP changes and all newer `main` plugin/core additions.

- [x] **Step 1: Confirm local main delta**

Run:

```bash
git status --short --branch
git log --oneline HEAD..main
```

Expected: only the plan file is untracked before implementation, and `HEAD..main` shows the newer local `main` commits.

- [x] **Step 2: Merge local main**

Run:

```bash
git merge main
```

Expected: either a clean merge or conflicts in shared core/plugin surfaces. If conflicts appear, keep both sides: all new `main` plugin work plus HIBP password validator API, route calls, dependencies, and tests.

- [x] **Step 3: Run focused smoke tests after merge**

Run:

```bash
cargo test -p openauth-core password_validator
cargo test -p openauth-plugins haveibeenpwned
```

Expected: both pass. If unrelated `main` plugin tests require local sockets, do not broaden to all plugin tests until socket permissions are available.

- [x] **Step 4: Commit merge if Git did not auto-commit**

Run:

```bash
git status --short --branch
git commit -m "Merge branch 'main' into feature/haveibeenpwned-plugin"
```

Expected: merge commit exists only if `git merge main` did not already create one.

## Task 1: Add Options Constructor

**Files:**
- Modify: `crates/openauth-plugins/src/haveibeenpwned/plugin.rs`
- Modify: `crates/openauth-plugins/src/haveibeenpwned/mod.rs`
- Modify: `crates/openauth-plugins/tests/haveibeenpwned/mod.rs`

- [x] **Step 1: Add the public constructor test**

Add this test near the existing plugin id test:

```rust
#[test]
fn options_constructor_preserves_upstream_options_shape() -> Result<(), Box<dyn std::error::Error>>
{
    let plugin = have_i_been_pwned_with_options(HaveIBeenPwnedOptions {
        enabled: false,
        paths: vec!["/change-password".to_owned()],
        custom_password_compromised_message: Some("Use another password.".to_owned()),
    });

    assert_eq!(plugin.id, "have-i-been-pwned");
    let Some(options) = plugin.options else {
        return Err("plugin options should be serialized".into());
    };
    assert_eq!(options["enabled"], false);
    assert_eq!(options["paths"], serde_json::json!(["/change-password"]));
    assert_eq!(
        options["customPasswordCompromisedMessage"],
        "Use another password."
    );
    Ok(())
}
```

Update the imports in the same file to include `have_i_been_pwned_with_options`.

- [x] **Step 2: Run the focused test and verify it fails**

Run:

```bash
cargo test -p openauth-plugins options_constructor_preserves_upstream_options_shape
```

Expected: compile failure because `have_i_been_pwned_with_options` is not exported yet.

- [x] **Step 3: Implement the constructor**

Add this in `crates/openauth-plugins/src/haveibeenpwned/plugin.rs` after `have_i_been_pwned()`:

```rust
pub fn have_i_been_pwned_with_options(options: HaveIBeenPwnedOptions) -> AuthPlugin {
    have_i_been_pwned_with_checker(options, Arc::new(ReqwestHaveIBeenPwnedChecker::new()))
}
```

Update `have_i_been_pwned()` to call it:

```rust
pub fn have_i_been_pwned() -> AuthPlugin {
    have_i_been_pwned_with_options(HaveIBeenPwnedOptions::default())
}
```

Update `crates/openauth-plugins/src/haveibeenpwned/mod.rs` exports:

```rust
pub use plugin::{
    have_i_been_pwned, have_i_been_pwned_with_checker, have_i_been_pwned_with_options,
    RUNTIME_PLUGIN_ID, UPSTREAM_PLUGIN_ID,
};
```

- [x] **Step 4: Run the focused test and verify it passes**

Run:

```bash
cargo test -p openauth-plugins options_constructor_preserves_upstream_options_shape
```

Expected: one matching test passes.

## Task 2: Match Sign-Up Duplicate Email Precedence

**Files:**
- Modify: `crates/openauth-core/src/api/routes/sign_up.rs`
- Modify: `crates/openauth-core/tests/api/routes/password_validators.rs`

- [x] **Step 1: Add a core route regression test**

Add this test after `password_validator_rejects_sign_up_before_user_creation`:

```rust
#[tokio::test]
async fn sign_up_duplicate_email_is_rejected_before_password_validator(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let initial_router = router(adapter.clone())?;
    let first = initial_router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;
    assert_eq!(first.status(), StatusCode::OK);

    let rejecting_router = router_with_options(
        adapter,
        OpenAuthOptions {
            plugins: vec![rejecting_password_plugin("/sign-up/email")],
            ..OpenAuthOptions::default()
        },
    )?;
    let duplicate = rejecting_router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);
    let body: ApiErrorResponse = serde_json::from_slice(duplicate.body())?;
    assert_eq!(body.code, "USER_ALREADY_EXISTS");
    Ok(())
}
```

- [x] **Step 2: Run the targeted test and verify it fails**

Run:

```bash
cargo test -p openauth-core sign_up_duplicate_email_is_rejected_before_password_validator
```

Expected: failure with `PASSWORD_COMPROMISED` until the route checks duplicate email before validators.

- [x] **Step 3: Move duplicate-email detection before password validators**

In `crates/openauth-core/src/api/routes/sign_up.rs`, add an email existence check after username duplicate checking and before `run_password_validators`:

```rust
if DbUserStore::new(adapter.as_ref())
    .find_user_by_email(&input.email)
    .await?
    .is_some()
{
    return auth_flow_error_response(
        crate::auth::email_password::AuthFlowError::new(
            crate::auth::email_password::AuthFlowErrorCode::UserAlreadyExists,
        ),
    );
}
```

Keep the existing duplicate check inside `EmailPasswordAuth::sign_up` as a race-safe final guard.

- [x] **Step 4: Run the targeted tests**

Run:

```bash
cargo test -p openauth-core sign_up_duplicate_email_is_rejected_before_password_validator
cargo test -p openauth-core password_validator
```

Expected: all matching tests pass.

## Task 3: Verification And Commits

**Files:**
- All modified files from Task 1 and Task 2.

- [x] **Step 1: Format and run focused plugin tests**

Run:

```bash
cargo fmt
cargo test -p openauth-plugins haveibeenpwned
```

Expected: HIBP tests pass.

- [x] **Step 2: Run core and public API checks**

Run:

```bash
cargo test -p openauth-core
cargo test -p openauth
cargo fmt --check
cargo clippy --all-targets --all-features
```

Expected: all pass.

- [x] **Step 3: Run full plugin tests when local socket permissions are available**

Run:

```bash
cargo test -p openauth-plugins
```

Expected: all pass outside the restricted sandbox. In the current sandbox, unrelated captcha tests can fail with `Operation not permitted` because they open local sockets.

- [x] **Step 4: Commit the follow-up**

Run:

```bash
git add crates/openauth-core/src/api/routes/sign_up.rs \
  crates/openauth-core/tests/api/routes/password_validators.rs \
  crates/openauth-plugins/src/haveibeenpwned/plugin.rs \
  crates/openauth-plugins/src/haveibeenpwned/mod.rs \
  crates/openauth-plugins/tests/haveibeenpwned/mod.rs \
  docs/superpowers/plans/2026-05-15-haveibeenpwned-parity-followup.md
git commit -m "feat(plugins): tighten haveibeenpwned parity"
```

- [x] **Step 5: Confirm local main is integrated**

Run:

```bash
git log --oneline HEAD..main
```

Expected: empty output. If it is not empty, repeat Task 0 before opening a PR.
