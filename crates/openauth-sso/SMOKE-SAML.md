# openauth-sso — SAML smoke checklist

Manual and semi-automated validation for SAML SP flows (signed assertions, encrypted
assertions, SLO). **Not run in CI** — use `scripts/saml-smoke.sh` locally.

**Related:** [openauth-sso upstream parity](./README.md#upstream-parity-better-auth-169) ·
`scripts/saml-smoke.sh` · `.env.saml-smoke.example`

---

## 1. What runs where

| Layer | Command | Network | CI |
| --- | --- | --- | --- |
| **Offline smoke** (Phase 1) | `./scripts/saml-smoke.sh` | No | No — too slow; use targeted `cargo test` in CI |
| **CI regression** | `cargo test -p openauth-sso --features saml,oidc -- saml` | No | Yes |
| **Live sandbox** (Phase 2) | `SAML_SMOKE_LIVE=1 ./scripts/saml-smoke.sh` | Yes (your server + IdP admin) | No |

Offline smoke uses **opensaml-generated** signed/encrypted XML and production-shaped
fixtures (`tests/fixtures/saml/idp/*-shaped.json`). That is intentional: reproducible
without Okta/Azure/Google credentials.

Live smoke validates your **real tenant** wiring (metadata upload, browser SSO).

---

## 2. Phase 1 — offline (default)

From repo root:

```bash
chmod +x scripts/saml-smoke.sh
./scripts/saml-smoke.sh
```

This runs:

1. `cargo test -p openauth-sso --features saml,oidc -- saml` (135 tests)
2. `cargo test -p openauth-saml --features saml-signed`

No environment variables required.

---

## 3. Phase 2 — live IdP sandbox (optional)

### Prerequisites

| Requirement | Notes |
| --- | --- |
| Running OpenAuth server | With SSO plugin + `saml` feature |
| SAML provider registered | `providerId` matches `SAML_SMOKE_PROVIDER_ID` |
| IdP SAML app (test/sandbox) | Okta dev org, Azure enterprise app, or Google Workspace SAML |
| IdP signing certificate | PEM from IdP admin UI |

Copy `crates/openauth-sso/.env.saml-smoke.example` → repo root `.env` and fill the
live block.

```bash
export SAML_SMOKE_LIVE=1
./scripts/saml-smoke.sh
```

Phase 2 checks:

- Required env vars present
- `GET {OPENAUTH_BASE_URL}/sso/saml2/sp/metadata/{providerId}` returns valid XML
- IdP cert parses with `openssl x509`
- Prints browser checklist for manual sign-in

### Manual sign-in verification

1. Register provider (if not already) with `entryPoint`, `cert`, and `idpMetadata.entityId`
   from your sandbox.
2. Open sign-in: `POST /sign-in/sso` with `{"providerId":"…","providerType":"saml"}`.
3. Complete IdP login in browser.
4. Confirm ACS redirect does **not** contain `/login-error`.
5. Confirm user/account rows created with expected email and mapped `account_id`.

Optional SLO: enable `enableSingleLogout` in `SamlOptions`, trigger logout, verify
session cleared.

---

## 4. opensaml dependency (external crate)

This repo does **not** vendor-edit `opensaml` sources. The workspace pins a git rev:

```toml
opensaml = { git = "https://github.com/sebasxsala/opensaml-rs", rev = "d65e77da..." }
```

Changes made **upstream in opensaml-rs** (not in `openauth-saml`):

- `create_logout_request_with_id` / `create_logout_response_with_id` — preserve caller
  LogoutRequest/Response IDs (required for SP-initiated SLO state).

All SAML integration logic lives in `openauth-saml` and `openauth-sso`.

---

## 5. Upstream parity

See [README — Upstream parity](./README.md#upstream-parity-better-auth-169).
Most `saml.test.ts` themes are **Covered** in Rust. Remaining differences are upstream-only
(browser mock IdP chain, `defaultSSO` array ordering, Better Auth client callback names).

**Remaining Work** (live tenant smoke) is optional manual validation — not upstream parity.
