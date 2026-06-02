# Audit checklist — `openauth-social-providers` vs Better Auth 1.6.9

Use when bumping the parity pin or adding a provider.

**Legend:** ✅ done / aligned · ⚠️ partial · ❌ gap · ➖ N/A · 🔒 intentional

| ID | Wire | Hooks | Integration test file | Notes |
| --- | --- | --- | --- | --- |
| apple | ✅ | ⚠️ | `tests/apple.rs` | form_post; JWKS verify |
| atlassian | ✅ | ⚠️ map | `src/atlassian.rs` (7 unit) | PKCE required |
| cognito | ✅ | ⚠️ map | `tests/cognito.rs` | JWKS; `%20` scopes |
| discord | ✅ | ⚠️ | `tests/discord.rs` | `scope_joiner` `+` |
| dropbox | ✅ | ⚠️ | `tests/dropbox.rs` | |
| facebook | 🔒 opaque verify | ⚠️ | `tests/facebook.rs` | |
| figma | ✅ | ⚠️ | `tests/figma.rs` | PKCE required |
| github | ✅ | ⚠️ map | `tests/github.rs` | map replaces user |
| gitlab | ✅ | ⚠️ | `tests/gitlab.rs` | locked accounts |
| google | ✅ | ⚠️ | `tests/google.rs` | PKCE; multi client id |
| huggingface | ✅ | ⚠️ get/map/ref | `tests/huggingface.rs` | |
| kakao | ✅ | ⚠️ | `tests/kakao.rs` | |
| kick | ✅ | ⚠️ | `tests/kick.rs` | |
| line | ✅ | ⚠️ | `tests/line.rs` | LINE verify |
| linear | ✅ | ⚠️ map | `tests/linear.rs` | GraphQL |
| linkedin | ✅ | ⚠️ | `tests/linkedin.rs` | |
| microsoft | ✅ | ⚠️ | `tests/microsoft_entra_id.rs` | strict multitenant iss |
| naver | ✅ | ⚠️ | `tests/naver.rs` | resultcode |
| notion | ✅ | ⚠️ | `tests/notion.rs` | Basic token |
| paybin | ✅ | ⚠️ | `tests/paybin.rs` | PKCE; id token user |
| paypal | ✅ | ⚠️ map/verify | `tests/paypal.rs` | decode `sub` default |
| polar | ✅ | ⚠️ map | `tests/polar.rs` | |
| railway | ✅ | ⚠️ | `tests/railway.rs` | PKCE optional |
| reddit | ✅ | ⚠️ | `tests/reddit.rs` | |
| roblox | ✅ | ⚠️ | `tests/roblox.rs` | `scope_joiner` `+` |
| salesforce | ✅ | ⚠️ get/map/ref | `tests/salesforce.rs` | |
| slack | ✅ | ⚠️ | `tests/slack.rs` | space scopes (upstream) |
| spotify | ✅ | ⚠️ | `tests/spotify.rs` | |
| tiktok | ✅ | ⚠️ | `tests/tiktok.rs` | no upstream mapProfile |
| twitch | ✅ | ⚠️ | `tests/twitch.rs` | Rust JWKS verify extra |
| twitter | ✅ | ⚠️ get/map/ref | `tests/twitter.rs` | |
| vercel | ✅ | ⚠️ get/map | `tests/vercel.rs` | no refresh |
| vk | ✅ | ⚠️ | `tests/vk.rs` | device_id |
| wechat | ✅ | ⚠️ | `tests/wechat.rs` | GET token |
| zoom | ✅ | ⚠️ | `tests/zoom.rs` | pkce flag |

**Registry:** ✅ `PROVIDER_IDS` + `module_structure.rs`

**Hooks column:** ⚠️ = upstream supports `ProviderOptions` callbacks; Rust has fixed methods unless typed `*Options` fields exist (10 providers). See [hooks-coverage.md](./hooks-coverage.md).

**Out of crate:** E2E → [integration-openauth-core.md](./integration-openauth-core.md).
