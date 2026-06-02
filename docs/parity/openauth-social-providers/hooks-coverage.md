# ProviderOptions hooks coverage

Better Auth attaches four optional callbacks to **every** built-in provider via `ProviderOptions` (`packages/core/src/oauth2/oauth-provider.ts`):

| Hook | Purpose |
| --- | --- |
| `getUserInfo` | Replace entire userinfo fetch |
| `mapProfileToUser` | Merge or transform profile → `OAuth2UserInfo` fields |
| `refreshAccessToken` | Replace token refresh |
| `verifyIdToken` | Replace ID-token verification |

Upstream checks `if (options.getUserInfo)` (etc.) inside each provider implementation.

Rust `openauth_oauth::ProviderOptions` has **no** callback fields. Only some `*Options` structs expose typed `Option<Arc<dyn Fn…>>` overrides.

## Matrix (mechanical read of sources)

| Provider | Upstream `mapProfile` | Upstream `getUserInfo` | Upstream `refresh` hook | Upstream `verify` hook | Rust typed `map` | Rust typed `get` | Rust typed `refresh` | Rust typed `verify` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| apple | ✓ | ✓ | ✓ | ✓ | — | — | — | — |
| atlassian | ✓ | ✓ | ✓ | — | ✓ | — | — | — |
| cognito | ✓ | ✓ | ✓ | ✓ | ✓ | — | — | — |
| discord | ✓ | ✓ | ✓ | — | — | — | — | — |
| dropbox | ✓ | ✓ | ✓ | — | — | — | — | — |
| facebook | ✓ | ✓ | ✓ | ✓ | — | — | — | — |
| figma | ✓ | ✓ | ✓ | — | — | — | — | — |
| github | ✓ | ✓ | ✓ | — | ✓ | — | — | — |
| gitlab | ✓ | ✓ | ✓ | — | — | — | — | — |
| google | ✓ | ✓ | ✓ | ✓ | — | — | — | — |
| huggingface | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | — |
| kakao | ✓ | ✓ | ✓ | — | — | — | — | — |
| kick | ✓ | ✓ | ✓ | — | — | — | — | — |
| line | ✓ | ✓ | ✓ | ✓ | — | — | — | — |
| linear | ✓ | ✓ | ✓ | — | ✓ | — | — | — |
| linkedin | ✓ | ✓ | ✓ | — | — | — | — | — |
| microsoft | ✓ | ✓ | ✓ | ✓ | — | — | — | — |
| naver | ✓ | ✓ | ✓ | — | — | — | — | — |
| notion | ✓ | ✓ | ✓ | — | — | — | — | — |
| paybin | ✓ | ✓ | ✓ | — | — | — | — | — |
| paypal | ✓ | ✓ | ✓ | ✓ | ✓ | — | — | ✓ |
| polar | ✓ | ✓ | ✓ | — | ✓ | — | — | — |
| railway | ✓ | ✓ | ✓ | — | — | — | — | — |
| reddit | ✓ | ✓ | ✓ | — | — | — | — | — |
| roblox | ✓ | ✓ | ✓ | — | — | — | — | — |
| salesforce | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | — |
| slack | ✓ | ✓ | ✓ | — | — | — | — | — |
| spotify | ✓ | ✓ | ✓ | — | — | — | — | — |
| tiktok | **—** | ✓ | ✓ | — | — | — | — | — |
| twitch | ✓ | ✓ | ✓ | — | — | — | — | — |
| twitter | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | — |
| vercel | ✓ | ✓ | — | — | ✓ | ✓ | — | — |
| vk | ✓ | ✓ | ✓ | — | — | — | — | — |
| wechat | ✓ | ✓ | ✓ | — | — | — | — | — |
| zoom | ✓ | ✓ | ✓ | — | — | — | — | — |

**TikTok:** upstream maps profile inline in `getUserInfo`; it does **not** call `mapProfileToUser` (confirmed in `tiktok.ts`).

**Vercel:** neither upstream nor Rust implements `refreshAccessToken` on the provider object (aligned omission).

### Rust providers with typed hook fields (10)

| Provider | Fields on `*Options` |
| --- | --- |
| atlassian, cognito, github, linear, polar | `map_profile_to_user` |
| paypal | `map_profile_to_user`, `verify_id_token` |
| vercel | `map_profile_to_user`, `get_user_info` |
| huggingface, salesforce, twitter | map, get, refresh |

### Built-in verify (not hooks)

These implement fixed `verify_id_token` / `verifyIdToken` without requiring a Rust hook field:

| Provider | Method |
| --- | --- |
| google, apple, cognito, facebook, microsoft | JWKS (`jose` / `josekit`) |
| line | LINE `POST …/oauth2/v2.1/verify` (+ payload checks in Rust) |
| paypal | Default decode: upstream `sub` check; Rust **`Ok(false)`** unless hook |
| twitch | **Rust only:** JWKS verify (upstream has no `verifyIdToken`) |

## Semantic note: `mapProfileToUser`

| | Upstream | Rust (where field exists) |
| --- | --- | --- |
| Merge behavior | Spreads partial object onto built user | Often **replaces** full `OAuth2UserInfo` (GitHub) or typed patch (Twitter) |

## If full hook parity is required

1. Extend `openauth_oauth::ProviderOptions` with optional `Arc` callbacks, **or**
2. Add per-provider `Option` fields for the remaining 26 providers, **or**
3. Document that apps wrap `Arc<dyn SocialOAuthProvider>` for custom behavior.

Default **refresh** and **get_user_info** behavior is still implemented as fixed methods on each provider when hooks are unset upstream; Rust mirrors that in `src/<provider>.rs` + `runtime/`.
