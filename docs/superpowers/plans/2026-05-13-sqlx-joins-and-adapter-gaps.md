# SQLx Joins and Adapter Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to continue this plan task-by-task.

**Goal:** Add Better Auth-style experimental joins support to OpenAuth core and SQLx adapters.

**Architecture:** Core owns the join contract, nested `DbValue` shape, runtime join fallback, and `experimental.joins` option. Concrete SQLx adapters remain outside core and expose join-capable behavior through `DbAdapter`.

**Tech Stack:** Rust, Tokio, SQLx, OpenAuth adapter contract, SQLite/Postgres/MySQL tests.

---

### Task 1: Core Join Contract

- [x] Add `ExperimentalOptions { joins: bool }` to `OpenAuthOptions`.
- [x] Add `supports_joins` and `.with_joins()` to `AdapterCapabilities`.
- [x] Add `DbValue::Record` and `DbValue::RecordArray`.
- [x] Add serde round-trip tests for nested values.

### Task 2: Runtime Join Fallback

- [x] Add `JoinAdapter` wrapper in core.
- [x] Resolve one-to-many forward joins and one-to-one reverse joins from `DbSchema`.
- [x] Keep joins off the inner adapter when native joins are disabled or unsupported.
- [x] Pass joins through when `experimental.joins` is enabled and the inner adapter reports `supports_joins`.
- [x] Preserve selected base fields while still fetching internal join keys.

### Task 3: Adapter and Store Integration

- [x] Wrap adapters passed through `open_auth_with_adapter` with `JoinAdapter`.
- [x] Make `MemoryAdapter` resolve direct join queries for tests and local development.
- [x] Update `DbUserStore::find_user_by_email_with_accounts` to request joined accounts and fall back for legacy mocks.

### Task 4: SQLx Adapter Join Behavior

- [x] Expose `.with_joins()` from SQLite, Postgres, and MySQL adapter capabilities.
- [x] Replace join rejection tests with nested join behavior tests.
- [x] Support forward, reverse, missing-row, selected-base-field, and join-limit scenarios.
- [x] Optional optimization: replace SQLx adapter-internal fallback with single-query `LEFT JOIN` execution per dialect for `find_one` and single-join `find_many`.
- [x] Keep batched fallback for multi-join `find_many` to avoid row explosion on list endpoints.

### Task 5: Validation

- [x] `cargo test -p openauth-core --test db`
- [x] `cargo test -p openauth-core --test options`
- [x] `cargo test -p openauth-sqlx --features sqlite --test sqlite_adapter`
- [x] `cargo test -p openauth-sqlx --features postgres --test postgres_adapter postgres_adapter_supports_forward_reverse_and_limited_joins`
- [x] `cargo test -p openauth-sqlx --features mysql --test mysql_adapter --no-run`
- [x] `cargo test -p openauth-sqlx --features mysql --test mysql_adapter`
- [x] `cargo test -p openauth-sqlx --features sqlite,postgres,mysql`
- [x] `cargo clippy -p openauth-core --all-targets -- -D warnings`
- [x] `cargo clippy -p openauth-sqlx --all-targets --features sqlite,postgres,mysql -- -D warnings`

### Notes

- SQLx join behavior now uses native `LEFT JOIN` for `find_one` and for `find_many` with one relation. `find_many` with multiple relations intentionally uses the batched fallback because it behaves more like an include/preload and avoids multiplying list rows across several one-to-many relations.
- MySQL runtime validation passed after local TCP approval.
