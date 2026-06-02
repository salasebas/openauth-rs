# Integration: `openauth-core` social HTTP layer

This completes the parity picture started in the provider crate. Better Auth splits:

| Layer | Upstream | OpenAuth |
| --- | --- | --- |
| Provider logic | `@better-auth/core/social-providers` | `openauth-social-providers` |
| Routes + session + DB | `packages/better-auth/src/api/routes/` | `openauth-core` `api/routes/social/` |

Parity pin: Better Auth **1.6.9**.

## Route mapping

| Upstream endpoint | Handler file | OpenAuth |
| --- | --- | --- |
| `POST /sign-in/social` | `api/routes/sign-in.ts` → `signInSocial` | `api/routes/social/flow.rs` (sign-in entry) |
| `GET|POST /callback/:id` | `api/routes/callback.ts` → `callbackOAuth` | `flow.rs` → `callback_get` / callback POST |
| `POST /link-social` | `api/routes/account.ts` → `linkSocialAccount` | `api/routes/account.rs` + social support types |
| ID-token sign-in body | `signInSocial` (provider-specific) | `flow.rs` → `sign_in_with_id_token` |

## What `flow.rs` does (server)

1. Resolve `Arc<dyn SocialOAuthProvider>` from `AuthContext::social_providers`.
2. **Sign-in:** build authorization URL (state, PKCE, redirect) → redirect or JSON.
3. **Callback:** parse state, exchange code via `validate_authorization_code`, `get_user_info`.
4. **ID token path:** `verify_id_token` then `get_user_info` (Apple/Google/Microsoft, etc.).
5. Delegate user/account/session to `handle_oauth_user_info` (account linking rules).

Provider crate does **not** implement cookies, CSRF, or `additionalData` on OAuth state — those live in core.

## `social.test.ts` → ownership

| Test area | Owner crate |
| --- | --- |
| Provider URL scopes, token form, profile map | `openauth-social-providers` |
| Redirect after sign-in, session cookie, new user | `openauth-core` |
| `disableImplicitSignUp`, `disableSignUp`, `requestSignUp` | `openauth-core` |
| `overrideUserInfoOnSignIn` | `openauth-core` + provider options |
| OAuth state `additionalData` / tamper fields | `openauth-core` |
| Callback URL attack hardening | `openauth-core` |
| `updateAccountOnSignIn` | `openauth-core` |
| Google multi-`clientId` + wrong audience rejection (E2E) | **Partial** — provider tests multi id; full E2E in core TBD |
| Vercel/Railway full OAuth flow E2E | `openauth-core` TBD |
| Account linking (trusted providers, email match) | `openauth-core` vs `oauth2/link-account.test.ts` |

## `openauth-plugins` usage

`openauth-plugins` / Google One Tap calls `openauth_social_providers::google` directly for `verify_id_token` + `get_user_info`, bypassing the generic social route table for that path.

## Recommended next parity pass (if any)

Only if end-to-end Better Auth parity is required:

1. Port high-value `social.test.ts` scenarios into `openauth-core` integration tests.
2. Port `link-account.test.ts` scenarios for `trustedProviders` / link-social.
3. Do **not** duplicate E2E in `openauth-social-providers` — keep contract tests there.

After provider wire fixes (discord/roblox `+`, railway PKCE optional, paypal verify), **provider-layer wire parity is 33/35 full + 1 intentional div (facebook)**.
