//! OpenAuth authentication toolkit.

pub mod auth;
pub mod prelude;

pub use auth::{OpenAuth, OpenAuthBuilder};
pub use openauth_core::{
    api, context, cookies, crypto, db, env, error, options, plugin, rate_limit, session, user,
    utils, verification,
};
pub use openauth_core::{oauth, social_providers};
#[cfg(feature = "deadpool-postgres")]
pub use openauth_deadpool_postgres as deadpool_postgres;
#[cfg(feature = "i18n")]
pub use openauth_i18n as i18n;
#[cfg(feature = "oidc")]
pub use openauth_oidc as oidc;
#[cfg(feature = "passkey")]
pub use openauth_passkey as passkey;
#[cfg(feature = "plugins")]
pub use openauth_plugins as plugins;
#[cfg(feature = "saml")]
pub use openauth_saml as saml;
#[cfg(feature = "scim")]
pub use openauth_scim as scim;
#[cfg(feature = "sqlx")]
pub use openauth_sqlx as sqlx;
#[cfg(feature = "sso")]
pub use openauth_sso as sso;
#[cfg(feature = "stripe")]
pub use openauth_stripe as stripe;
#[cfg(feature = "telemetry")]
pub use openauth_telemetry::{
    create_telemetry, get_telemetry_auth_config, CustomTrackFn, TelemetryContext, TelemetryEvent,
    TelemetryPublisher, TelemetryTestHooks,
};
#[cfg(feature = "tokio-postgres")]
pub use openauth_tokio_postgres as tokio_postgres;

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
