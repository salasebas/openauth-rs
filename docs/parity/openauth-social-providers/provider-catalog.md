# Provider catalog (35 built-ins)

Two independent ratings (do not conflate):

| Column | Meaning |
| --- | --- |
| **Wire** | Default-path endpoints, scopes, token exchange, profile → `OAuth2UserInfo` |
| **Hooks** | Injectable `ProviderOptions` callbacks — see [hooks-coverage.md](./hooks-coverage.md) |

| Wire | Hooks |
| --- | --- |
| **Full** — matches 1.6.9 default path | **Full** — typed override fields for all four (none have all four in Rust) |
| **Partial** — known wire gap | **Partial** — some typed overrides |
| **Gap** — wire mismatch | **None** — fixed methods only; upstream still has hook check sites |
| **Div** — intentional divergence | |

Confirmed wire issues: [confirmed-gaps.md](./confirmed-gaps.md).

## Master table

| ID | Upstream file | Wire | Hooks | PKCE | Built-in `verify_id_token` |
| --- | --- | --- | --- | --- | --- |
| `apple` | `apple.ts` | Full | None | No | Yes (JWKS) |
| `atlassian` | `atlassian.ts` | Full | Partial (map) | **Required** | No |
| `cognito` | `cognito.ts` | Full | Partial (map) | Optional | Yes (JWKS) |
| `discord` | `discord.ts` | Full (`+` scope) | None | Optional | No |
| `dropbox` | `dropbox.ts` | Full | None | Optional | No |
| `facebook` | `facebook.ts` | **Div** (opaque verify) | None | No | Yes (JWT only) |
| `figma` | `figma.ts` | Full | None | Required | No |
| `github` | `github.ts` | Full | Partial (map) | Optional | No |
| `gitlab` | `gitlab.ts` | Full (+ locked filter) | None | Optional | No |
| `google` | `google.ts` | Full | None | **Required** | Yes (JWKS) |
| `huggingface` | `huggingface.ts` | Full | Partial (get/map/refresh) | Optional | No |
| `kakao` | `kakao.ts` | Full | None | No | No |
| `kick` | `kick.ts` | Full | None | Optional | No |
| `line` | `line.ts` | Full | None | Optional | Yes (LINE verify) |
| `linear` | `linear.ts` | Full | Partial (map) | No | No |
| `linkedin` | `linkedin.ts` | Full | None | No | No |
| `microsoft` | `microsoft-entra-id.ts` | Full (stricter iss) | None | Optional | Yes (JWKS) |
| `naver` | `naver.ts` | Full | None | No | No |
| `notion` | `notion.ts` | Full | None | Required | No |
| `paybin` | `paybin.ts` | Full | None | Required | No (user from id token) |
| `paypal` | `paypal.ts` | Full | Partial (map, verify hook) | Required | Yes (decode `sub`) |
| `polar` | `polar.ts` | Full | Partial (map) | Optional | No |
| `railway` | `railway.ts` | Full | None | Optional | No |
| `reddit` | `reddit.ts` | Full | None | Optional | No |
| `roblox` | `roblox.ts` | Full (`+` scope) | None | Optional | No |
| `salesforce` | `salesforce.ts` | Full | Partial (get/map/refresh) | Required | No |
| `slack` | `slack.ts` | Full | None | Optional | No |
| `spotify` | `spotify.ts` | Full | None | Optional | No |
| `tiktok` | `tiktok.ts` | Full | None | `client_key` | No |
| `twitch` | `twitch.ts` | Full (+ Rust verify) | None | Optional | Yes (Rust-only JWKS) |
| `twitter` | `twitter.ts` | Full | Partial (get/map/refresh) | Optional | No |
| `vercel` | `vercel.ts` | Full | Partial (get/map) | Required | No |
| `vk` | `vk.ts` | Full | None | Optional | No |
| `wechat` | `wechat.ts` | Full | None | No | No |
| `zoom` | `zoom.ts` | Full | None | Configurable | No |

### Summary counts

| Wire | Count | Providers |
| --- | --- | --- |
| Full | 33 | Default path matches upstream |
| Div | 1 | facebook (opaque `verifyIdToken`) |
| Extra | 1 | twitch (JWKS verify not in upstream) |

| Hooks (typed Rust overrides) | Count |
| --- | --- |
| Partial | 10 | atlassian, cognito, github, huggingface, linear, paypal, polar, salesforce, twitter, vercel |
| None | 25 | all others |

## Upstream-only `ProviderOptions` hooks (most providers)

Better Auth attaches these to **every** provider via `ProviderOptions` in `oauth-provider.ts`:

| Hook | In Rust `ProviderOptions`? | OpenAuth pattern |
| --- | --- | --- |
| `mapProfileToUser` | No (global) | Per-provider struct field where needed (`GitHubOptions`, …) |
| `getUserInfo` | No | `TwitterOptions`, `HuggingFaceOptions`, `SalesforceOptions`, `VercelOptions`, … |
| `verifyIdToken` | No | Built-in per provider or `PayPalOptions.verify_id_token` |
| `refreshAccessToken` | No | Default trait method or per-provider override |
| `revokeToken` | Trait exists; most providers unsupported | Same default `Err` as upstream absence |

Closing full **option-hook parity** would mean extending `openauth-oauth::ProviderOptions` or a shared `SocialProviderHooks` trait — currently a deliberate scope boundary.

## ID token verification matrix

| Provider | Upstream `verifyIdToken` | Rust default |
| --- | --- | --- |
| Google | JWKS + aud/iss/nonce | Same intent |
| Apple | JWKS + audience rules | Same intent |
| Microsoft | JWKS; relaxed issuer for multi-tenant | **Stricter** `accepts_multitenant_issuer` |
| Cognito | JWKS | Same |
| Facebook | JWT verify; **opaque token → `true`** | JWT only; opaque → **`false`** |
| Twitch | Not defined | **JWKS verify added** |
| LINE | Verify endpoint | Same |
| PayPal | Decode JWT, check `sub` | **`false`** unless custom hook |
| Others | Usually absent | `unsupported_id_token` → `false` |

## Providers with custom refresh in Rust

All providers implement `refresh_access_token` on the trait where the upstream provider defines `refreshAccessToken`. Notable: Twitter/Hugging Face/Salesforce allow custom refresh callbacks on options structs.

## Registry parity test

`tests/module_structure.rs` asserts `PROVIDER_IDS` matches the upstream key list exactly (order included).
