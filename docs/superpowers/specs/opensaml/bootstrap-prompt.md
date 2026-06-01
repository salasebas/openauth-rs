# Bootstrap prompt — `opensaml` + `samlify` (re-export)

Copy everything inside the fenced block below into a new agent session.

---

````
# Objetivo

Bootstrap de dos crates Rust en el workspace OpenAuth (`openauth-rs`):

1. **`opensaml`** — librería SAML 2.0 **Service Provider** (capa protocolo + bindings), equivalente a la parte alta de [samlify](https://github.com/tngan/samlify). La criptografía XML (XML-DSig, XML-Enc, C14N) va en **bergshamra**, no aquí.

2. **`samlify`** — crate delgada que **solo re-exporta** `opensaml` (API familiar tipo npm `samlify`, sin lógica propia).

`openauth-saml` y `openauth-sso` no se reemplazan en este PR; solo scaffolding + utilidades. Cero regresiones en tests existentes.

# Arquitectura

```
bergshamra          → XML-DSig, XML-Enc, C14N, XPath, KeysManager (pure Rust)
opensaml            → SP SAML: metadata, AuthnRequest, parse login/logout, bindings
samlify             → pub use opensaml::*; (alias de marca/API)
openauth-saml       → integración OpenAuth (fase posterior consume opensaml)
openauth-sso        → HTTP, sesión, replay (como Better Auth sobre samlify)
```

Referencias behavior (no commitear clones):

- Better Auth SSO: `reference/upstream-src/1.6.9/repository/packages/sso/`
- samlify (lectura / porting): `reference/upstream-samlify/2.10.2/repository/` vía script

# Alcance M0 (este PR)

## Hacer

### 1. Crate `opensaml`

- Ruta: `crates/opensaml/`
- Registrar en workspace root `Cargo.toml` → `members`
- Añadir `opensaml = { path = "crates/opensaml", version = "0.0.6" }` en `[workspace.dependencies]` (misma versión workspace)
- `#![forbid(unsafe_code)]`
- **Sin** dependencia de `openauth-core` (standalone)

**Dependencias (workspace donde exista):**

- `base64`, `flate2`, `quick-xml`, `thiserror`, `url`
- `bergshamra = "0.4"` solo con feature `crypto-bergshamra` (optional, default off en M0)

**Features `opensaml`:**

```toml
[features]
default = []
crypto-bergshamra = ["dep:bergshamra"]
```

**Módulo `opensaml::binding`** (mover/adaptar desde `openauth-saml`, copiar sin borrar origen aún):

| Función / tipo | Descripción |
|----------------|-------------|
| `xml_escape(&str) -> String` | `& " ' < >` para XML |
| `html_escape(&str) -> String` | Para `action` / `value` en form POST |
| `deflate_raw_encode` / `deflate_raw_decode` | Wrapper `flate2`, raw deflate SAML Redirect |
| `base64_encode` / `base64_decode` | Normalizar whitespace SAML |
| `saml_post_binding_form(action, param_name, b64_value, relay_state?) -> String` | HTML auto-submit |
| `redirect_binding_query(saml_param, b64_value, relay_state?) -> String` | Query sin firma; documentar extensión SigAlg/Signature |

**Esqueletos (stubs documentados, sin paridad samlify aún):**

- `error` — `OpenSamlError`
- `sp` — `ServiceProvider` (entity_id, acs_url, optional keys/certs)
- `idp` — `IdentityProvider` (entity_id, sso_url, signing_cert)
- `metadata` — `generate_sp_metadata` stub
- `authn` — `create_login_request_redirect` stub
- `response` — `parse_login_response_post` stub (bergshamra cuando feature on)
- `logout` — stubs redirect/post
- `crypto` — trait `XmlSecurityBackend`; `BergshamraBackend` behind feature

**`lib.rs` re-exports:**

```rust
pub mod binding;
pub mod crypto;
pub mod error;
pub mod sp;
pub mod idp;
pub mod metadata;
pub mod authn;
pub mod response;
pub mod logout;

pub use error::OpenSamlError;
pub use sp::ServiceProvider;
pub use idp::IdentityProvider;
```

### 2. Crate `samlify` (solo re-export)

- Ruta: `crates/samlify/`
- Registrar en workspace `members`
- `description = "Re-exports opensaml with a samlify-familiar crate name."`
- Dependencia única: `opensaml = { workspace = true }`
- **Sin** código de negocio; solo:

```rust
//! Thin re-export crate. All SAML logic lives in `opensaml`.

#![forbid(unsafe_code)]

pub use opensaml::*;
```

- README: aclarar que no es el npm `samlify`; es el alias Rust de `opensaml` para OpenAuth.

### 3. Referencia upstream samlify (gitignored)

- `scripts/fetch-upstream-samlify.sh` — clona tag `v2.10.2` (mismo pin que Better Auth usa `~2.10.2`)
- Destino: `reference/upstream-samlify/2.10.2/repository/`
- `reference/upstream-samlify/VERSION.md` con tag, commit, URL repo
- Asegurar `.gitignore` cubre `reference/upstream-samlify/**/repository/` (como better-auth)
- No commitear el clone

### 4. Documentación

- `crates/opensaml/README.md` — experimental, SP-only v1, tabla samlify(npm) vs opensaml vs bergshamra, features, roadmap
- `crates/samlify/README.md` — una página: "use `opensaml` directly or `samlify` for the re-export name"

### 5. Tests M0 (`opensaml` only)

- `tests/binding.rs`:
  - deflate roundtrip
  - `xml_escape` golden
  - `saml_post_binding_form` escapa `<` en values, incluye `action` y hidden input
- `cargo fmt --all`
- `cargo clippy -p opensaml -p samlify --all-targets -- -D warnings`
- `cargo nextest run -p opensaml -p samlify`

### 6. `openauth-saml` (solo comentarios TODO)

En archivos que dupliquen binding, añadir:

```rust
// TODO(opensaml): delegate to opensaml::binding (M1 migration).
```

No cambiar comportamiento ni tests de `openauth-saml` / `openauth-sso` en M0.

## No hacer en M0

- XMLDSig verify/sign completo
- Integrar `openauth-sso` con `opensaml` / `samlify`
- Publicar en crates.io
- Borrar código en `openauth-saml`
- Implementar IdP completo (solo SP-first en roadmap)

# Estructura de archivos

```
crates/opensaml/
  Cargo.toml
  README.md
  src/lib.rs
  src/error.rs
  src/binding/mod.rs
  src/binding/deflate.rs
  src/binding/encoding.rs
  src/binding/escape.rs
  src/binding/post_form.rs
  src/binding/redirect.rs
  src/sp.rs
  src/idp.rs
  src/metadata.rs
  src/authn.rs
  src/response.rs
  src/logout.rs
  src/crypto/mod.rs
  src/crypto/backend.rs
  src/crypto/bergshamra.rs
  tests/binding.rs

crates/samlify/
  Cargo.toml
  README.md
  src/lib.rs

reference/upstream-samlify/
  VERSION.md

scripts/fetch-upstream-samlify.sh
```

# Cargo.toml orientativo

## `crates/opensaml/Cargo.toml`

```toml
[package]
name = "opensaml"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
readme = "README.md"
description = "SAML 2.0 Service Provider library (bergshamra for XML crypto)."
keywords = ["saml", "sso", "xml", "authentication"]
categories = ["authentication", "web-programming"]

[lib]
name = "opensaml"
path = "src/lib.rs"

[features]
default = []
crypto-bergshamra = ["dep:bergshamra"]

[dependencies]
base64.workspace = true
flate2.workspace = true
quick-xml.workspace = true
thiserror.workspace = true
url.workspace = true
bergshamra = { version = "0.4", optional = true }

[dev-dependencies]

[lints]
workspace = true
```

## `crates/samlify/Cargo.toml`

```toml
[package]
name = "samlify"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
readme = "README.md"
description = "Re-exports opensaml (samlify-shaped crate name for Rust)."
keywords = ["saml", "sso", "authentication"]
categories = ["authentication", "web-programming"]

[lib]
name = "samlify"
path = "src/lib.rs"

[dependencies]
opensaml.workspace = true

[lints]
workspace = true
```

## Workspace root (añadir)

```toml
# members:
"crates/opensaml",
"crates/samlify",

# [workspace.dependencies]:
opensaml = { path = "crates/opensaml", version = "0.0.6" }
samlify = { path = "crates/samlify", version = "0.0.6" }
```

# Script `scripts/fetch-upstream-samlify.sh`

Comportamiento análogo a `scripts/fetch-upstream-better-auth.sh`:

- Pin versión `2.10.2`, tag `v2.10.2`
- Clone shallow en `reference/upstream-samlify/2.10.2/repository`
- Idempotente: si existe, no re-clonar salvo que el usuario borre el directorio
- Imprimir ruta final

# Roadmap (README solamente, no implementar)

| Milestone | Contenido |
|-----------|-----------|
| M1 | `parse_login_response_post` + bergshamra verify (`trusted_keys_only`, `strict_verification`) |
| M2 | Redirect binding: query sig verify/sign |
| M3 | SLO parse/create |
| M4 | `EncryptedAssertion` decrypt (bergshamra-enc) |
| M5 | `openauth-saml` depende de `opensaml`; tests portados desde `packages/sso/src/saml.test.ts` |

# Criterios de aceptación

- [ ] `cargo nextest run -p opensaml -p samlify` pasa
- [ ] `cargo clippy -p opensaml -p samlify --all-targets -- -D warnings` pasa
- [ ] `samlify` crate contiene únicamente `pub use opensaml::*` (+ docs)
- [ ] `./scripts/fetch-upstream-samlify.sh` documentado en README opensaml
- [ ] Tests `openauth-saml` / `openauth-sso` sin cambios de comportamiento
- [ ] Seguir `AGENTS.md`: verificación scoped, no commitear upstream clone

# Convenciones

- Código y docs en **inglés**
- Comentarios mínimos
- Preferir copiar lógica probada desde `openauth-saml` hacia `opensaml::binding` en M0
- bergshamra: configuración SAML recomendada documentada en README (trusted_keys_only, strict_verification) para M1
````

---

## Uso

1. Abre este archivo en el editor.
2. Copia **solo** el contenido entre las líneas que empiezan con ```` ` (cuatro backticks) — desde `# Objetivo` hasta el final del bloque.
3. Pégalo en un chat nuevo como instrucción única.

## Nombres

| Nombre | Rol |
|--------|-----|
| `opensaml` | Crate con toda la lógica |
| `samlify` | `pub use opensaml::*` |
| `bergshamra` | XML crypto (feature `crypto-bergshamra`) |
