//! Server-side passkey plugin for OpenAuth.
//!
//! The plugin is server-only. It exposes Better Auth-inspired HTTP endpoints
//! under `/passkey/*`, contributes a `passkeys` table to the OpenAuth schema,
//! and uses `webauthn-rs` for WebAuthn ceremony generation and verification.
//!
//! ```rust,no_run
//! use openauth_core::options::OpenAuthOptions;
//! use openauth_passkey::{passkey, PasskeyOptions};
//!
//! let options = OpenAuthOptions::new()
//!     .secret("secret-a-at-least-32-chars-long!!")
//!     .base_url("https://app.example.com")
//!     .plugin(passkey(PasskeyOptions::default()));
//! ```
//!
//! WebAuthn registration and authentication state is persisted server-side in
//! OpenAuth's `verification` storage and keyed by a signed short-lived cookie.
//! This is why the crate enables `webauthn-rs` state serialization: the state is
//! not trusted from the client and is deleted after successful verification.

mod challenge;
mod cookies;
mod errors;
mod openapi;
mod options;
mod response;
mod routes;
mod schema;
mod session;
mod store;
mod webauthn;

pub use errors::PASSKEY_ERROR_CODES;
pub use options::{
    AfterAuthenticationVerificationInput, AfterRegistrationVerificationInput,
    AuthenticatorAttachment, AuthenticatorSelection, PasskeyAdvancedOptions,
    PasskeyAuthenticationOptions, PasskeyOptions, PasskeyRegistrationOptions,
    PasskeyRegistrationUser, RegistrationWebAuthnOptions, ResidentKeyRequirement,
    ResolveRegistrationUserInput, UserVerificationRequirement,
};
pub use store::Passkey;
pub use webauthn::{
    PasskeyAuthenticationStart, PasskeyRegistrationStart, PasskeyWebAuthnBackend,
    VerifiedAuthentication, VerifiedPasskeyCredential, WebAuthnConfig,
};

use openauth_core::plugin::AuthPlugin;

pub const UPSTREAM_PLUGIN_ID: &str = "passkey";

/// Build the server-side passkey plugin.
pub fn passkey(options: PasskeyOptions) -> AuthPlugin {
    let options = std::sync::Arc::new(options);
    let mut plugin = AuthPlugin::new(UPSTREAM_PLUGIN_ID).with_version(env!("CARGO_PKG_VERSION"));
    for contribution in schema::contributions(&options.passkey_table) {
        plugin = plugin.with_schema(contribution);
    }
    for code in errors::plugin_error_codes() {
        plugin = plugin.with_error_code(code);
    }
    for endpoint in routes::endpoints(options) {
        plugin = plugin.with_endpoint(endpoint);
    }
    plugin
}
