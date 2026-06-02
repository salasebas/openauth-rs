# Confirmed parity gaps (source-verified)

Gaps below were verified by reading `packages/core/src/social-providers/*.ts`, `crates/openauth-social-providers/src/*.rs`, and `tests/*.rs` — not READMEs.

Use two ratings:

| Rating | Meaning |
| --- | --- |
| **Wire** | Authorization URL, token endpoint, default scopes, profile mapping for default path |
| **Hooks** | `getUserInfo` / `mapProfileToUser` / `refreshAccessToken` / `verifyIdToken` overrides on `ProviderOptions` |

---

## Wire-level gaps (behavior may differ in production)

### Resolved in crate (2026-06-01)

| Provider | Was | Fix |
| --- | --- | --- |
| **discord** | Space-separated scopes | `scope_joiner: "+"` on auth URL |
| **roblox** | Space-separated scopes | `scope_joiner: "+"` |
| **railway** | PKCE required at authorize | Optional (matches upstream `createAuthorizationURL`) |
| **paypal** | Default verify always `false` | Decode JWT payload; accept when `sub` present (like upstream `decodeJwt`) |

### Still open

| Provider | Gap | Upstream | Rust | Tests |
| --- | --- | --- | --- | --- |
| **facebook** | Opaque token verify | Non-JWT → **`true`** | Non-JWT → **`false`** | 🔒 intentional |
| **twitch** | ID token verify | No `verifyIdToken` | **JWKS `verify_id_token`** | Stricter than upstream |
| **wechat** | `platformType` option | Documented, unused in TS | Omitted | ➖ dead field |
| **gitlab** | Locked accounts | No filter | Rejects `state: locked` | Hardening |
| **github** | Token exchange errors | `null` | `Err` | Idiomatic Rust |
| **github** | `mapProfileToUser` | Merges partial fields | **Replaces** user when set | Semantic |

Discord and Roblox: Discord’s API accepts both `+` and space in practice for many apps, but the authorize URL string **differs** from Better Auth 1.6.9.

---

## Hook-level gap (all providers)

| Gap | Upstream | Rust |
| --- | --- | --- |
| Global `ProviderOptions` callbacks | All providers delegate to `options.*` when set | Only **9** providers expose typed overrides; **26** have no injectable hooks |

See [hooks-coverage.md](./hooks-coverage.md).

---

## PKCE requirement alignment

| Provider | Upstream requires `codeVerifier` at authorize | Rust requires |
| --- | --- | --- |
| google | Yes (`BetterAuthError`) | Yes (`MissingOption`) |
| atlassian | Yes | Yes |
| figma | Yes | Yes |
| paybin | Yes | Yes |
| salesforce | Yes | Yes |
| vercel | Yes | Yes |
| **railway** | **No** | **Yes** ← only mismatch |

---

## Providers with no dedicated integration test file

| Provider | Rust tests location | Count (approx) |
| --- | --- | --- |
| **atlassian** | `src/atlassian.rs` `#[cfg(test)]` | 7 unit tests |
| All others | `tests/<provider>.rs` or `tests/microsoft_entra_id.rs` | see [testing.md](./testing.md) |

---

## Upstream E2E not mirrored in this crate

From `packages/better-auth/src/social.test.ts` (40 `it` cases). These belong to **openauth-core**, not `openauth-social-providers`:

| Theme | Upstream test | Rust crate coverage |
| --- | --- | --- |
| Registry / sign-in redirect | `should be able to add/sign in…` | None (HTTP) |
| Async provider factory | `async social provider` | None |
| Callback URL when user exists | `Should use callback URL…` | None |
| **mapProfileToUser E2E** | `should be able to map profile to user` | Only per-provider unit tests (vercel, linear, salesforce, …) |
| Open redirect / callback attacks | `callback URL attacks` | None |
| Refresh after sign-in session | `should refresh the access token` | Per-provider refresh **request shape** only |
| Redirect URI infer/custom | `Redirect URI` describe | Partial via `redirect_uri` on auth URL tests |
| `disableImplicitSignUp` / `disableSignUp` | two describes | None in crate (core options) |
| `overrideUserInfoOnSignIn` | two `it` in signin | None |
| OAuth state `additionalData` | `should allow additional data` | None |
| State tamper fields | `should not allow overriding oauth code verifier…` | None |
| `updateAccountOnSignIn` | describe | None |
| Google multi-client + id-token aud | `Google Provider — multiple client IDs` | **Partial** — `tests/google.rs` multi `ClientId`, not reject-wrong-aud E2E |
| Apple name / idToken body | `Apple Provider` (4 tests) | **Partial** — `tests/apple.rs` mapping, not full POST body |
| Vercel E2E (7 tests) | full flow + mapProfile | **Partial** — auth URL, mapper, PKCE; no session |
| Microsoft id-token / JWKS / tenant | `Microsoft Provider` (5 tests) | **Strong** — `tests/microsoft_entra_id.rs` (14 tests) |
| Railway E2E (3 tests) | PKCE flow | **Partial** — contract tests + forced PKCE |

---

## Providers tested for custom hooks in Rust

| Provider | Test file | What's tested |
| --- | --- | --- |
| vercel | `tests/vercel.rs` | `vercel_custom_mapper_can_override_user_info_fields` |
| linear | `tests/linear.rs` | `linear_custom_mapper_can_override_user_info_fields` |
| salesforce | `tests/salesforce.rs` | custom `get_user_info`, `refresh_access_token`, partial mapper |
| huggingface | `tests/huggingface.rs` | custom get/map/refresh callbacks |
| twitter | `tests/twitter.rs` | custom get/map/refresh |
| paypal | `tests/paypal.rs` | custom `verify_id_token` hook |
| github | — | **No** test for `map_profile_to_user` override |

---

## Suggested backlog (wire parity)

1. **facebook** — product decision: keep strict opaque rejection or add compat flag `accept_opaque_id_token` (upstream-compatible).
2. **github** — optional: merge semantics for `map_profile_to_user` to match upstream spread.

## Suggested backlog (hook parity)

Only if apps rely on Better Auth-style `options.mapProfileToUser` for providers without Rust fields — extend `ProviderOptions` or add fields per provider (26 remaining).
