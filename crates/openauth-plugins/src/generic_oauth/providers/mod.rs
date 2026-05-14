//! Generic OAuth provider helper placeholders.

pub mod auth0;
pub mod gumroad;
pub mod hubspot;
pub mod keycloak;
pub mod line;
pub mod microsoft_entra_id;
pub mod okta;
pub mod patreon;
pub mod slack;

pub const PROVIDER_IDS: &[&str] = &[
    "auth0",
    "gumroad",
    "hubspot",
    "keycloak",
    "line",
    "microsoft-entra-id",
    "okta",
    "patreon",
    "slack",
];
