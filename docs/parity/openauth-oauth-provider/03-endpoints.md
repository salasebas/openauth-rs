# 03 — HTTP endpoints

Routes are relative to the OpenAuth `base_path` (typically `/api/auth`). Upstream uses the same prefix.

## Summary

| | Upstream | OpenAuth |
| --- | --- | --- |
| Distinct routes | 25 | 26 |
| Difference | — | `POST` and `GET` on `/oauth2/continue` |

## OAuth / OIDC protocol

| Method | Path | Upstream | OpenAuth | Auth | Notes |
| --- | --- | --- | --- | --- | --- |
| GET | `/.well-known/oauth-authorization-server` | Yes (`SERVER_ONLY`) | Yes | Public | OIDC document when `openid` ∈ scopes (Jun 2026 closeout) |
| GET | `/.well-known/openid-configuration` | Yes | Yes | Public | 404 if `openid` not in scopes |
| GET | `/oauth2/authorize` | Yes | Yes | Public + implicit session | `response_type=code`, PKCE, prompts, `request_uri` |
| POST | `/oauth2/consent` | Yes | Yes | Session | Accept / deny scopes |
| POST | `/oauth2/continue` | Yes | Yes | Session | Continue after select_account / create / post_login |
| GET | `/oauth2/continue` | No | **Yes** | Session | Rust extension (same handler, query vs body) |
| POST | `/oauth2/token` | Yes | Yes | Client | `application/x-www-form-urlencoded` |
| POST | `/oauth2/introspect` | Yes | Yes | Client | RFC 7662 |
| POST | `/oauth2/revoke` | Yes | Yes | Client | RFC 7009 |
| GET | `/oauth2/userinfo` | Yes | Yes | Bearer | Requires `openid` scope |
| GET | `/oauth2/end-session` | Yes | Yes | Public | RP-initiated logout |
| POST | `/oauth2/register` | Yes | Yes | Session or public if `allow_unauthenticated_client_registration` | DCR |

## OAuth client management

| Method | Path | Upstream | OpenAuth | Auth | Notes |
| --- | --- | --- | --- | --- | --- |
| POST | `/admin/oauth2/create-client` | Yes | Yes | Server / admin | Privileged fields (`skip_consent`, etc.) |
| POST | `/oauth2/create-client` | Yes | Yes | Session | Same logic, fewer fields |
| GET | `/oauth2/get-client` | Yes | Yes | Session + ownership | Does not return stored secret |
| GET | `/oauth2/public-client` | Yes | Yes | Session | Public UI metadata only |
| POST | `/oauth2/public-client-prelogin` | Yes | Yes | Signed `oauth_query` | Requires `allow_public_client_prelogin` |
| GET | `/oauth2/get-clients` | Yes | Yes | Session | By `user_id` or `reference_id` |
| PATCH | `/admin/oauth2/update-client` | Yes | Yes* | Admin | *OpenAuth uses **POST** (same semantics) |
| POST | `/oauth2/update-client` | Yes | Yes | Session | `token_endpoint_auth_method` immutable |
| POST | `/oauth2/client/rotate-secret` | Yes | Yes | Session | Rejects public clients |
| POST | `/oauth2/delete-client` | Yes | Yes | Session | Rejects `cached_trusted_clients` |

## Consent management

| Method | Path | Upstream | OpenAuth |
| --- | --- | --- | --- |
| GET | `/oauth2/get-consent` | Yes | Yes |
| GET | `/oauth2/get-consents` | Yes | Yes |
| POST | `/oauth2/update-consent` | Yes | Yes |
| POST | `/oauth2/delete-consent` | Yes | Yes |

## Upstream HTTP hooks without a dedicated Rust route

| Upstream behavior | OpenAuth implementation |
| --- | --- |
| `before`: read `body.oauth_query`, sign, attach to `/sign-in/social`, `/sign-in/oauth2` | **Not replicated in this crate**; prelogin validates `oauth_query` on `public-client-prelogin`. Apps can pass state into login explicitly. |
| `after`: parse `Set-Cookie`, resume `/oauth2/authorize` after login | **Partial:** `prompt=create` / `select_account` / `post_login` + `/oauth2/continue` with state in `verification` |
| `Sec-Fetch-Mode: navigate` → 302 vs JSON `{ url }` | `redirect_or_json_response` for CORS / `Accept: application/json` (Jun 2026) |

## Rate limiting (upstream defaults = OpenAuth)

| Route | Window | Max |
| --- | --- | --- |
| `/oauth2/token` | 60s | 20 |
| `/oauth2/authorize` | 60s | 30 |
| `/oauth2/introspect` | 60s | 100 |
| `/oauth2/revoke` | 60s | 30 |
| `/oauth2/register` | 60s | 5 |
| `/oauth2/userinfo` | 60s | 60 |

## Endpoints that exist on neither side (checklist reference)

| Capability | Upstream 1.6.9 | OpenAuth |
| --- | --- | --- |
| PAR storage endpoint | Only `request_uri` + resolver callback | Same |
| `/.well-known/oauth-protected-resource` HTTP on oauth-provider | Built in MCP challenge, not always AS route | Helper `mcp::protected_resource_metadata` |
| Device code, implicit, password grants | Rejected / not implemented | Rejected |

## Per-endpoint parity matrix

| Endpoint | Parity | Main remaining gap |
| --- | --- | --- |
| authorize | High | Automatic post-login cookie hooks |
| consent / continue | High | Extra `GET` continue in Rust |
| token | High | — |
| introspect / revoke | High | Fewer JWT-only tests than upstream |
| userinfo | High | No headers-only mode without `Request` |
| end-session | High | — |
| register + client CRUD | High | Admin update: POST vs PATCH |
| well-known | **High** | OIDC on oauth-authorization-server when `openid` (closed Jun 2026) |
| MCP HTTP | N/A in crate | See `openauth-plugins::mcp` |
