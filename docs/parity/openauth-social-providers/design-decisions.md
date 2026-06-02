# Design decisions and intentional divergences

Items marked **intentional** are acceptable for OpenAuth’s server-only, Rust-native goals unless product requires strict Better Auth compatibility.

## Server-only scope

| Upstream | OpenAuth decision |
| --- | --- |
| Client SDK `signIn.social`, `linkSocial`, cookie state | Not implemented in this crate; core handles HTTP |
| Browser redirect UX | Application responsibility |
| `disableRedirect: true` client flows | Core route options (if exposed), not providers |
| React / Vue helpers | Out of scope |

## Error model

| Topic | Upstream | OpenAuth | Why |
| --- | --- | --- | --- |
| Token exchange failure | Often `null` | `Result::Err(OAuthError)` | Idiomatic Rust; explicit errors |
| Userinfo failure | `null` | `Ok(None)` or `Err` depending on provider | Documented per provider |
| Missing PKCE / client id | `BetterAuthError` | `OAuthError::MissingOption` | Typed options |

**Partial parity impact:** Callers migrating from Better Auth TS null-checks must handle `Result` in Rust.

## Security hardening (intentional)

| Area | Behavior | Rating |
| --- | --- | --- |
| Outbound HTTP | `ProviderHttpClient` / `ValidationHttpClient` block private IPs | **Rust addition** |
| Facebook `verifyIdToken` | Upstream returns `true` for non-JWT (opaque) tokens | Rust returns **`false`** — **Divergent**; prevents treating access tokens as verified ID tokens |
| Microsoft multitenant | Upstream relaxed issuer for `common` / `organizations` / `consumers` | Rust **`accepts_multitenant_issuer`** rules — **stricter** |
| Twitch | No upstream `verifyIdToken` | Rust **JWKS verification** — **stricter** |
| GitLab | Same userinfo | Rust rejects **`state: locked`** accounts |

## Option hooks placement

| Upstream | OpenAuth |
| --- | --- |
| `mapProfileToUser`, `getUserInfo`, `verifyIdToken`, `refreshAccessToken` on shared `ProviderOptions` | Only on specific `*Options` structs (Twitter, Hugging Face, Salesforce, Vercel, PayPal, …) |

**Reason:** Rust favors compile-time struct fields over dynamic option bags; reduces clone-heavy callback wiring.

**Gap if you need BA compatibility:** Apps can wrap `Arc<dyn SocialOAuthProvider>` or extend `ProviderOptions` in `openauth-oauth`.

## Semantic differences

| Provider | Topic | Upstream | OpenAuth |
| --- | --- | --- | --- |
| GitHub | `mapProfileToUser` | Merges partial fields into user | **Replaces** full `OAuth2UserInfo` when set |
| Google | Custom hooks | `options.getUserInfo` etc. | Not exposed on `GoogleOptions` |
| PayPal | Default `verifyIdToken` | JWT decode + `sub` check | **`Ok(false)`** unless `verify_id_token` hook — **gap** |
| WeChat | `platformType` | Documented, unused in TS | Omitted (dead upstream field) |
| Discord / Roblox | Scope separator | `+` joined | **`scope_joiner: "+"`** (aligned 2026-06-01) |
| Railway | PKCE at authorize | Optional | Optional (aligned 2026-06-01) |
| PayPal | Default verify | `decodeJwt` + `sub` | Same decode semantics (aligned 2026-06-01) |

## Architecture (Rust-specific)

| Decision | Rationale |
| --- | --- |
| `src/` + `src/runtime/` split | Public types vs `SocialOAuthProvider` macro impl |
| `impl_social_oauth_provider!` macro | DRY trait wiring; consistent defaults (`unsupported_id_token`) |
| Separate crate from `openauth-oauth` | Reuse OAuth primitives for non-social flows |
| No Cargo features on this crate | Enabled via `openauth-core` feature `social-providers` |
| `PROVIDER_IDS` ordered like upstream | Mechanical parity check in CI |

## TypeScript-only upstream details

| Item | Handling |
| --- | --- |
| `clientId?: never` on TikTok | Rust uses `client_key` field explicitly |
| `Date` types on Microsoft profile claims | Rust uses `i64` / optional fields |
| Zod `SocialProviderListEnum` | `PROVIDER_IDS` + registration-time validation |
| Async `socialProviders` config factories | App-level `async` construction, not crate API |

## When to mark a difference as a bug vs decision

| Treat as **bug / backlog** | Treat as **decision** |
| --- | --- |
| PayPal default ID token verify | Facebook opaque token rejection |
| Missing hook if app relies on BA `options.mapProfileToUser` for Google | SSRF HTTP wrappers |
| Wrong endpoint or scope vs upstream 1.6.9 | `Result` instead of `null` |
| | Twitch JWKS verify |

Track backlog items in provider-catalog **Partial** rows and core/plugin parity docs.
