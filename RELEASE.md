# Release Process

This release process is for the independent, unofficial **RustAuth** Rust
workspace. It is inspired by Better Auth but is not a 1:1 port, not affiliated
with, maintained by, endorsed by, or sponsored by the Better Auth project or
its maintainers.

This repository does not use Better Auth’s Changesets setup; that flow is
built around pnpm and npm package publishing.

RustAuth uses **Cargo** and is released via **release-plz**, **crates.io
Trusted Publishing**, and **GitHub releases**.

The workspace currently uses one shared version in `[workspace.package]`.
Member crates keep `version.workspace = true`, and release-plz keeps them in a
single `rustauth-workspace` version group.

## Automated release flow

1. Merge normal changes to `main`.
2. The `Release-plz` workflow runs `release-plz release-pr` and opens or updates
   a **draft** release PR from a `release-plz-*` branch. The PR bumps the shared
   workspace version, updates path dependency pins, refreshes changelogs, and
   updates `Cargo.lock` when needed.
3. Review the release PR. Mark it ready for review, let CI pass, and merge it
   when you want to publish.
4. The next `Release-plz` workflow run executes `release-plz release` for the
   release PR merge commit. Because `release_always = false`, release-plz still
   verifies that publishing only happens for commits that came from the release
   PR. `workflow_dispatch` can be used as a manual fallback if the merge commit
   message was customized enough that the workflow condition skipped it.
5. The publish job uses the GitHub Environment named `release`. Configure that
   environment with required reviewers if you want a manual approval gate before
   crates are published.
6. release-plz publishes every unpublished crate version to crates.io. It creates
   the root `vX.Y.Z` Git tag and GitHub release from the aggregate `rustauth`
   changelog.

## Trusted Publishing setup

Configure a crates.io Trusted Publisher for each published crate:

- Repository: `salasebas/rustauth`
- Workflow filename: `release-plz.yml`
- Environment: `release`

Do not add `CARGO_REGISTRY_TOKEN` to the workflow when using Trusted Publishing.
The `release-plz-release` job has `id-token: write`, which release-plz uses to
request the crates.io publishing token.

Also configure GitHub repository Actions settings so workflows have read/write
permissions and can create pull requests; release-plz needs that to create and
update the release PR.

Trusted Publishing cannot publish a crate for the first time. All current
`rustauth*` crates are already published at `0.3.0`; if a new crate is added,
publish that crate manually once before relying on Trusted Publishing.

## Changelog policy

- The root `CHANGELOG.md` is the aggregate changelog for the umbrella
  `rustauth` package and includes commits from the public RustAuth crates.
- Specialized crates keep their own `crates/<crate>/CHANGELOG.md` files; release-plz
  updates those files when it finds package-specific changes.
- `crates/rustauth/CHANGELOG.md` is intentionally a pointer to the root
  changelog to avoid maintaining two generated changelogs for the umbrella
  crate.
- Commits should use conventional prefixes (`feat:`, `fix:`, `security:`,
  `perf:`, `refactor:`, etc.) so release-plz can classify changelog entries.

## Manual fallback

If release-plz is unavailable, use this manual process:

1. Bump the workspace version in the root `Cargo.toml` under
   `[workspace.package] version`. Member crates use `version.workspace = true`.
2. Align any **path dependency version pins** with the new release version (for
   example `rustauth-core = { path = "../rustauth-core", version = "…" }` in
   crates that depend on other workspace packages). Those semver constraints
   must match what you intend to publish on crates.io.
3. Refresh the lockfile: `cargo check` or `cargo build --workspace` so
   `Cargo.lock` reflects the bump (commit the lockfile change when it differs).
4. Run tests: `./scripts/ensure-test-services.sh postgres mysql redis valkey`,
   then `CARGO_INCREMENTAL=0 cargo nextest run --workspace --all-features`,
   then `CARGO_INCREMENTAL=0 cargo test --workspace --doc --all-features`.
5. Update the root `CHANGELOG.md` and each crate-level `CHANGELOG.md` with the
   release notes for the version being published. The `rustauth` umbrella crate
   uses the root changelog.
6. Publish crates to crates.io in **dependency order** (dependencies before
   dependents). Use `cargo publish -p <crate-name>` from the repository root
   for each package, and wait for each newly published version to be visible
   on crates.io before publishing crates that depend on it.
7. Tag the release commit (`git tag vX.Y.Z && git push origin vX.Y.Z`) and create
   a **GitHub release** with notes from `CHANGELOG.md`.

## Publish order

The current workspace packages must be published in this order:

1. `rustauth-oauth` — no RustAuth workspace dependencies.
2. `rustauth-oidc` — no RustAuth workspace dependencies.
3. `rustauth-social-providers` — depends on `rustauth-oauth`.
4. `rustauth-core` — depends on `rustauth-oauth` and
   `rustauth-social-providers`.
5. `rustauth-diesel` — depends on `rustauth-core`.
6. `rustauth-stripe` — depends on `rustauth-core`.
7. `rustauth-saml` — depends on `rustauth-core`.
8. `rustauth-i18n` — depends on `rustauth-core`.
9. `rustauth-sqlx` — depends on `rustauth-core`.
10. `rustauth-telemetry` — depends on `rustauth-core`.
11. `rustauth-tokio-postgres` — depends on `rustauth-core`.
12. `rustauth-deadpool-postgres` — depends on `rustauth-core` and
    `rustauth-tokio-postgres`.
13. `rustauth-redis` — depends on `rustauth-core`.
14. `rustauth-plugins` — depends on `rustauth-core`, `rustauth-oauth`, and
    `rustauth-social-providers`; publish verification also uses
    `rustauth-redis` and `rustauth-sqlx`.
15. `rustauth-passkey` — depends on `rustauth-core`; publish verification also
    uses `rustauth-sqlx`.
16. `rustauth-sso` — depends on `rustauth-core`, `rustauth-oauth`,
    `rustauth-oidc`, and `rustauth-saml`; publish verification also uses
    `rustauth-sqlx`.
17. `rustauth-scim` — depends on `rustauth-core`; publish verification also
    uses `rustauth-deadpool-postgres`, `rustauth-plugins`, `rustauth-sqlx`, and
    `rustauth-tokio-postgres`.
18. `rustauth-oauth-provider` — depends on `rustauth-core` and
    `rustauth-plugins`.
19. `rustauth` — depends on `rustauth-core`,
    `rustauth-deadpool-postgres`, `rustauth-diesel`, `rustauth-i18n`,
    `rustauth-oidc`, `rustauth-passkey`, `rustauth-plugins`, `rustauth-saml`,
    `rustauth-scim`, `rustauth-sqlx`, `rustauth-sso`, `rustauth-stripe`,
    `rustauth-telemetry`, and `rustauth-tokio-postgres`.
20. `rustauth-fred` — depends on `rustauth-core`, and its publish
    verification uses a dev-dependency on `rustauth`.
21. `rustauth-axum` — depends on `rustauth`.
22. `rustauth-actix-web` — depends on `rustauth`.
23. `rustauth-cli` — depends on `rustauth`, `rustauth-core`,
    `rustauth-plugins`, `rustauth-sqlx`, and optionally `rustauth-diesel`
    (via the `diesel` feature).

## Crate names

Rust crate names match the `name` field in each `crates/*/Cargo.toml`. The
workspace currently includes:

- `rustauth` — main umbrella crate (re-exports / integration surface)
- `rustauth-actix-web`
- `rustauth-axum`
- `rustauth-cli`
- `rustauth-core`
- `rustauth-deadpool-postgres`
- `rustauth-diesel`
- `rustauth-fred`
- `rustauth-i18n`
- `rustauth-oidc`
- `rustauth-oauth`
- `rustauth-oauth-provider`
- `rustauth-passkey`
- `rustauth-plugins`
- `rustauth-redis`
- `rustauth-saml`
- `rustauth-scim`
- `rustauth-social-providers`
- `rustauth-sqlx`
- `rustauth-sso`
- `rustauth-stripe`
- `rustauth-telemetry`
- `rustauth-tokio-postgres`

Published versions on crates.io are whatever you ship from this repository;
they are **not** the official Better Auth npm packages.
