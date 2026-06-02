# Testing parity

## Counts (Better Auth 1.6.9 vs `openauth-social-providers`)

| Layer | Upstream | OpenAuth |
| --- | --- | --- |
| Tests under `packages/core/src/social-providers/` | **0** | — |
| Provider contract tests | — | **310** `#[test]` / `#[tokio::test]` (counted via source scan) |
| Core OAuth2 unit tests | `oauth2/*.test.ts` (**15** `it`) | `openauth-oauth` (separate crate) |
| Social sign-in E2E | `better-auth/src/social.test.ts` (**40** `it`) | `openauth-core` (not this crate) |
| Account linking + OAuth utils | `oauth2/link-account.test.ts` (**15** `it`) | `openauth-core` |
| generic-oauth plugin | `generic-oauth.test.ts` (large) | Out of scope |

OpenAuth invests heavily in **per-provider contract tests** because upstream has **no** unit tests beside the monolithic E2E file.

## `social.test.ts` describe blocks (upstream E2E)

| Describe | `it` count | Covered in this crate? |
| --- | --- | --- |
| Social Providers | 8 | Partial (provider logic only) |
| Redirect URI | 2 | Partial (`redirect_uri` in auth URL tests) |
| Disable implicit signup | 2 | No (core) |
| Disable signup | 1 | No (core) |
| signin (override user, state) | 4 | No (core) |
| updateAccountOnSignIn | 1 | No (core) |
| Google Provider — multiple client IDs | 3 | Partial (multi `ClientId` in `google.rs`; no reject-aud E2E) |
| Multi-client ID — other widened providers | (suite) | Partial (apple, cognito, facebook, microsoft tests) |
| Apple Provider | 4 | Partial (`tests/apple.rs`) |
| Vercel Provider | 7 | Partial (`tests/vercel.rs` — no full session) |
| Microsoft Provider | 5 | Strong (`tests/microsoft_entra_id.rs`) |
| Railway Provider | 3 | Partial (contract + **stricter** PKCE) |

Full mapping: [confirmed-gaps.md](./confirmed-gaps.md#upstream-e2e-not-mirrored-in-this-crate).

## Rust test layout

```
crates/openauth-social-providers/
├── tests/<provider>.rs     # integration tests (one binary per provider, except atlassian)
├── tests/module_structure.rs
└── src/<provider>.rs       # occasional unit tests (atlassian, google, …)
```

### Per-file integration test counts

| File | Tests | File | Tests |
| --- | --- | --- | --- |
| `module_structure.rs` | 5 | `microsoft_entra_id.rs` | 14 |
| `apple.rs` | 8 | `naver.rs` | 9 |
| `cognito.rs` | 9 | `notion.rs` | 6 |
| `discord.rs` | 6 | `paybin.rs` | 10 |
| `dropbox.rs` | 4 | `paypal.rs` | 10 |
| `facebook.rs` | 12 | `polar.rs` | 7 |
| `figma.rs` | 8 | `railway.rs` | 9 |
| `github.rs` | 4 | `reddit.rs` | 8 |
| `gitlab.rs` | 9 | `roblox.rs` | 8 |
| `google.rs` | 5 | `salesforce.rs` | 14 |
| `huggingface.rs` | 9 | `slack.rs` | 8 |
| `kakao.rs` | 7 | `spotify.rs` | 8 |
| `kick.rs` | 8 | `tiktok.rs` | 10 |
| `line.rs` | 10 | `twitch.rs` | 11 |
| `linear.rs` | 8 | `twitter.rs` | 12 |
| `linkedin.rs` | 6 | `vercel.rs` | 7 |
| | | `vk.rs` | 9 |
| | | `wechat.rs` | 8 |
| | | `zoom.rs` | 11 |

**No `tests/atlassian.rs`.** Atlassian coverage lives in **`src/atlassian.rs`** (**7** unit tests).

### What Rust tests typically assert

| Category | Examples |
| --- | --- |
| Metadata | Provider `id`, `name`, default scopes |
| Authorization URL | Query params: `client_id`, `scope`, `state`, PKCE `code_challenge` / `S256`, provider-specific (`access_type`, `claims`, `audience`, WeChat `#wechat_redirect`) |
| Token exchange | POST body fields, Basic auth header, GET token (WeChat), `device_id` |
| Profile mapping | Provider JSON → `OAuth2UserInfo` (email, `email_verified`, image) |
| ID token | Decode/verify paths, JWKS mocks, nonce, audience |
| SSRF / HTTP | `ProviderHttpClient::permissive` or validation client for mocked hosts |
| Registry | `PROVIDER_IDS` exact match; all types implement `SocialOAuthProvider` |

Style: **contract / upstream-parity**, not full HTTP server stacks.

## What upstream `social.test.ts` covers (not duplicated here)

Rough themes across **40** tests:

| Theme | Upstream | OpenAuth location |
| --- | --- | --- |
| Provider registration / `enabled: false` | Yes | Core builder tests |
| Full redirect → callback → session | Yes | Core social flow tests |
| OAuth state `additionalData` | Yes | Core state module |
| Callback URL / open redirect hardening | Yes | Core security tests |
| Google multi-`clientId` + id-token audience | Partial | `google.rs` + core |
| Apple id-token + `user` form_post payload | Partial | `apple.rs` + core |
| Vercel / Railway PKCE E2E | Partial | `vercel.rs`, `railway.rs` |
| Microsoft `mapProfileToUser` + JWKS id-token | Partial | `microsoft_entra_id.rs` |
| Token refresh after sign-in | Partial | Per-provider + core |
| `overrideUserInfoOnSignIn` | Yes | Core linking |
| Implicit sign-up / `requestSignUp` | Yes | Core |

## Coverage gaps (Rust crate)

| Gap | Severity | Notes |
| --- | --- | --- |
| No `tests/atlassian.rs` | Low | **7** unit tests in `src/atlassian.rs` |
| No HTTP E2E in this crate | By design | Belongs to `openauth-core` |
| PayPal default `verify_id_token` | Medium | Test **documents** gap: `paypal_verify_id_token_rejects_payload_only_tokens_by_default` |
| Discord/Roblox scope `+` vs space | Medium | Tests **assert upstream mismatch** (space) |
| Railway forced PKCE | Low | Test **asserts** stricter Rust behavior |
| Hook override tests | Low | Only vercel, linear, salesforce, huggingface, twitter, paypal — not github map hook |
| Google reject wrong aud | Medium | Upstream E2E; not in `tests/google.rs` |
| `revoke_token` | Low | Unimplemented upstream for most built-ins |
| Cross-provider linking | N/A | `link-account.test.ts` → core |

## What Rust tests assert (by theme)

| Theme | Example test names | Providers |
| --- | --- | --- |
| Metadata `id` / `name` | `*_provider_exposes_upstream_metadata` | Most |
| Auth URL query contract | `*_authorization_url_*` | Most |
| PKCE required | `*_requires_code_verifier`, `*_uses_pkce` | google, figma, paybin, salesforce, vercel, railway, … |
| Token POST shape / Basic auth | `*_token_requests_use_basic_auth` | railway, paypal, notion, reddit, twitter, … |
| Profile → `OAuth2UserInfo` | `*_profile_maps*`, `maps_*_profile` | Most |
| ID token verify | `*_verify_id_token_*` | google, apple, cognito, facebook, microsoft, line, twitch, paypal |
| SSRF / private IP | `*rejects_private*` | gitlab, cognito, zoom, … |
| Custom hooks | `*_custom_*_callback*`, `*_custom_mapper*` | salesforce, huggingface, twitter, vercel, linear, paypal |
| Registry | `PROVIDER_IDS`, `SocialOAuthProvider` | `module_structure.rs` |

## Commands

```bash
cargo nextest run -p openauth-social-providers
cargo clippy -p openauth-social-providers --all-targets -- -D warnings
```
