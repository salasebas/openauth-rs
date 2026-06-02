# Complex providers (deep parity notes)

Providers with OIDC, custom token flows, required PKCE, or significant hook surface. Upstream paths: `packages/core/src/social-providers/<file>.ts`.

---

## Google (`google`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | Auth `accounts.google.com/o/oauth2/v2/auth`; token `oauth2.googleapis.com/token`; JWKS `googleapis.com/oauth2/v3/certs` |
| **Default scopes** | `email`, `profile`, `openid` |
| **PKCE** | **Required** (both) |
| **Extra auth params** | `access_type`, `display`, `hd`, `include_granted_scopes`, `prompt` |
| **Userinfo** | From **ID token** only (no userinfo HTTP) |
| **verify_id_token** | JWKS; issuers `https://accounts.google.com` / `accounts.google.com`; aud = client id(s); ~1h max age; nonce |
| **Gaps** | No Rust `map_profile_to_user` / `get_user_info` / `verify_id_token` **overrides** on options (upstream `ProviderOptions` hooks) |
| **Rust extras** | `device_id` on code exchange; `verify_id_token_with_jwks_url` for tests |

---

## Apple (`apple`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | `appleid.apple.com` authorize + token; JWKS `/auth/keys` |
| **Default scopes** | `email`, `name` |
| **PKCE** | Neither side uses PKCE on authorize |
| **Response** | `response_mode=form_post`, `response_type=code id_token` |
| **Client secret** | JWT (ES256) generated server-side |
| **User name** | First sign-in via `provider_user` / `token.user` (Rust `AppleNonConformUser`) |
| **verify_id_token** | Audience: `audience` → `app_bundle_identifier` → `client_id` |
| **Gaps** | Upstream `mapProfileToUser` / custom `getUserInfo` not on Rust options |

---

## GitHub (`github`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | `github.com/login/oauth/*`; API `api.github.com/user` + `/user/emails` |
| **Default scopes** | `read:user`, `user:email` |
| **PKCE** | Optional |
| **map_profile_to_user** | **Semantic gap:** Rust **replaces** user; upstream **merges** partial map |
| **Token errors** | Upstream `null`; Rust `Err` |
| **verify_id_token** | No (both) |

---

## Microsoft Entra ID (`microsoft` / `microsoft_entra_id`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | `{authority}/{tenant}/oauth2/v2.0/*`; Graph photo `graph.microsoft.com/.../photos/{size}x{size}/$value` |
| **Default scopes** | `openid`, `profile`, `email`, `User.Read`, `offline_access` |
| **Options** | `tenant_id`, `authority`, `profile_photo_size`, `disable_profile_photo` |
| **PKCE** | Optional; upstream does not require `client_secret` for authorize URL; Rust only requires `client_id` for authorize |
| **get_user_info** | Decode ID token + optional photo as `data:image/jpeg;base64,...` |
| **verify_id_token** | **Stricter** multitenant issuer handling (`accepts_multitenant_issuer`, consumer tenant rules) |
| **Gaps** | No `map_profile_to_user` on Rust options |

---

## Facebook (`facebook`)

| Aspect | Detail |
| --- | --- |
| **Parity** | **Divergent** on verify |
| **API version** | v24.0 Graph URLs |
| **Default scopes** | `email`, `public_profile` |
| **PKCE** | No |
| **Options** | `fields`, `config_id` |
| **Limited Login** | JWKS `limited.facebook.com/.../jwks/` |
| **verify_id_token** | **JWT:** both verify RS256. **Opaque token:** upstream **`true`**; Rust **`false`** (intentional) |
| **Gaps** | No `map_profile_to_user` |

---

## Cognito (`cognito`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | `https://{domain}/oauth2/*`; JWKS `cognito-idp.{region}.amazonaws.com/{userPoolId}/.well-known/jwks.json` |
| **Default scopes** | `openid`, `profile`, `email` |
| **Scope encoding** | `%20` not `+` (both) |
| **PKCE** | Optional |
| **Options** | `domain`, `region`, `user_pool_id`, `require_client_secret`, `map_profile_to_user` |
| **Constructor** | `cognito()` returns `Result` (validates config) |
| **verify_id_token** | Issuer + audience + nonce + ~1h via `iat` |
| **Gaps** | No custom `get_user_info` / `verify_id_token` hooks on options |

---

## WeChat (`wechat`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Authorize** | `open.weixin.qq.com/connect/qrconnect` + `#wechat_redirect` |
| **Token / refresh** | **GET** with `appid` / `secret` (not `client_id`) |
| **Default scope** | `snsapi_login` |
| **PKCE** | No |
| **User id** | `unionid` \|\| `openid`; `email_verified: false` |
| **Errors** | Upstream throws; Rust `OAuthError::InvalidResponse` |
| **Gaps** | `platformType` unused upstream — omitted in Rust |

---

## Linear (`linear`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **User fetch** | GraphQL `viewer` POST `api.linear.app/graphql` |
| **Default scope** | `read` |
| **PKCE** | No (both) |
| **map_profile_to_user** | Yes |
| **Refresh** | Rust may send `client_key` in refresh body when set |

---

## Atlassian (`atlassian`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial |
| **Endpoints** | `auth.atlassian.com`; API `api.atlassian.com/me` |
| **Default scopes** | `read:jira-user`, `offline_access` |
| **PKCE** | **Required** |
| **Extra** | `audience=api.atlassian.com` |
| **map_profile_to_user** | Yes |
| **Tests** | **7** unit tests in `src/atlassian.rs` only (no `tests/atlassian.rs`) |

---

## Twitter / X (`twitter`)

| Aspect | Detail |
| --- | --- |
| **Parity** | Partial (strongest hook parity in crate) |
| **Endpoints** | `x.com/i/oauth2/authorize`; `api.x.com/2/oauth2/token`; users/me with `confirmed_email` |
| **Default scopes** | `users.read`, `tweet.read`, `offline.access`, `users.email` |
| **PKCE** | Optional |
| **Token auth** | HTTP Basic |
| **Hooks** | `get_user_info`, `map_profile_to_user`, `refresh_access_token` on `TwitterOptions` (matches upstream option callbacks) |
| **verify_id_token** | No |

---

## Cross-reference

See [provider-catalog.md](../provider-catalog.md) for the full 35-provider table and [testing.md](../testing.md) for per-file test counts.
