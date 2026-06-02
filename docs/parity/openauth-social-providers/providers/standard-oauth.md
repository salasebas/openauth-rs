# Standard and grouped providers

Remaining **25** built-ins after [complex.md](./complex.md). Unless noted, **wire** parity is **Full** on endpoints, defaults, token exchange, and profile → `OAuth2UserInfo`. **Hooks** parity is **None** for most rows (see [hooks-coverage.md](../hooks-coverage.md)).

## Group A — Standard OAuth2 code + Bearer userinfo

| ID | Notes |
| --- | --- |
| `discord` | Manual-style params; `DiscordPrompt`, `permissions`; default avatar image URL logic |
| `dropbox` | `DropboxAccessType` → `token_access_type`; POST userinfo; **`DropboxProvider::new` only** (no `dropbox()` fn) |
| `huggingface` | Hooks on `HuggingFaceOptions`: `get_user_info`, `map_profile_to_user`, `refresh_access_token` |
| `kick` | PKCE; userinfo wraps `data[0]` array |
| `linkedin` | `login_hint`; no PKCE |
| `kakao` | Maps `kakao_account`; no PKCE |
| `naver` | Checks `resultcode == "00"` |
| `spotify` | PKCE; token POST with client auth; first image URL |
| `polar` | `prompt`; optional `map_profile_to_user` |

## Group B — PKCE + Basic auth at token endpoint

| ID | Notes |
| --- | --- |
| `railway` | PKCE required; OIDC-style userinfo |
| `figma` | PKCE + secret required |
| `notion` | `owner=user`; `Notion-Version` header; no default scope when disabled |

## Group C — OIDC / ID-token centric

| ID | Notes |
| --- | --- |
| `line` | LINE `/verify` for id token; fallback userinfo |
| `twitch` | `claims` on authorize; user from ID token; **Rust adds JWKS verify** (upstream has none) |
| `paybin` | Configurable `issuer`; PKCE required; user from ID token payload |
| `paypal` | **Partial:** sandbox/live `environment`; custom token Basic POST; default **`verify_id_token` gap** (see [design-decisions.md](../design-decisions.md)) |

## Group D — Instance / environment config

| ID | Notes |
| --- | --- |
| `gitlab` | `issuer` for self-hosted; rejects `state: locked` |
| `salesforce` | `environment`, `login_url`; PKCE required; get/map/refresh hooks |
| `zoom` | `pkce: bool` (default true); rich `ZoomProfile`; token POST auth |

## Group E — Special token exchange shape

| ID | Notes |
| --- | --- |
| `reddit` | `duration=permanent`; Basic auth; custom User-Agent; icon URL strips `?` |
| `tiktok` | `client_key` + `client_secret`; comma-separated scopes |
| `vk` | `device_id` on code exchange; email required or no user |
| `roblox` | `RobloxPrompt`; token POST body auth |
| `slack` | OpenID scopes; space-separated scope in authorize URL |
| `vercel` | PKCE required; `get_user_info` / `map_profile_to_user` hooks; no refresh in upstream or Rust |

## Quick reference table

| ID | Wire | PKCE | Special |
| --- | --- | --- | --- |
| `discord` | Full | Opt | permissions, prompt; `+` scope |
| `dropbox` | Full | Opt | access type |
| `figma` | Full | Req | basic token auth |
| `gitlab` | Full | Opt | issuer, locked accounts |
| `huggingface` | Full | Opt | option hooks |
| `kakao` | Full | No | kakao_account |
| `kick` | Full | Opt | array userinfo |
| `line` | Full | Opt | verify endpoint |
| `linkedin` | Full | No | login_hint |
| `naver` | Full | No | resultcode |
| `notion` | Full | Req | owner=user |
| `paybin` | Full | Req | custom issuer |
| `paypal` | Full | Req | env; decode `sub` verify |
| `polar` | Full | Opt | map hook |
| `railway` | Full | Opt | basic token |
| `reddit` | Full | Opt | duration, basic |
| `roblox` | Full | Opt | prompt enum; `+` scope |
| `salesforce` | Full | Req | env, hooks |
| `slack` | Full | Opt | OpenID |
| `spotify` | Full | Opt | PKCE |
| `tiktok` | Full | client_key | — |
| `twitch` | Full | Opt | JWKS verify (Rust+) |
| `vercel` | Full | Req | hooks |
| `vk` | Full | Opt | device_id |
| `zoom` | Full | toggle | POST token auth |

## Tests

Per-provider integration test file `tests/<id>.rs` except `microsoft` → `microsoft_entra_id.rs`. Counts in [testing.md](../testing.md).
