# Organization Plugin Parity Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining server-side parity and robustness gaps in `openauth-plugins::organization`.

**Architecture:** Keep `organization/mod.rs` as the public surface and preserve small files by splitting session, validation, additional field, and store helpers before source files exceed 500 lines. Implement behavior first with targeted integration tests, then run the full workspace checks.

**Tech Stack:** Rust, OpenAuth plugin contracts, `MemoryAdapter`, `serde`, `serde_json`, `time`, `http`, `indexmap`.

---

## Checklist

- [x] Register organization session fields through plugin init so `/get-session` returns `activeOrganizationId` and `activeTeamId`.
- [x] Refresh session cookies from endpoints that mutate active organization/team.
- [x] Add organization-scoped schema customizations and additional fields.
  - Progress: custom table names, field names, and additional-field schema metadata are implemented for organization/member/invitation/team/team_member/organization_role. Runtime input/response handling now covers organization, member, invitation, team, team_member, and organization_role with `returned: false` filtering.
- [x] Complete mutating hooks for org/member/invitation/team lifecycle.
  - Progress: added organization update/delete hooks, member add/remove/update-role hooks, invitation create/accept/reject/cancel hooks, and team create/update/delete/add-member/remove-member hooks. `before_*` hooks can mutate organization update data, member role data, invitation data, and team data where supported.
- [x] Harden dynamic access control validation and cross-org checks.
- [x] Harden team behavior around invitations, active team fallback, and cleanup.
- [x] Harden core route validation and duplicate slug handling.
- [x] Add OpenAPI operation ids/body schemas for organization endpoints.
- [x] Add tests for session fields, cookies, DAC, teams, hooks, and route gaps.
  - Progress: added targeted session, team-active, set-active-by-slug, DAC invalid resource, max-role, cross-org role id, custom schema metadata, additional-field runtime, endpoint registry/OpenAPI smoke coverage, and mutating hook coverage for organization/member/invitation/team flows.
- [x] Keep every organization source file under 500 lines.
- [x] Run `cargo test -p openauth-plugins`.
- [x] Run `cargo test -p openauth`.
- [x] Run `cargo clippy --all-targets --all-features`.
