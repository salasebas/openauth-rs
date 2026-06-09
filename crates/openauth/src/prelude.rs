//! Convenient re-exports for application developers mounting OpenAuth.
//!
//! Library authors extending adapters, plugins, or endpoints should import from
//! the focused modules (`openauth::db`, `openauth::plugin`, `openauth::api`, …)
//! instead of this prelude.

pub use crate::auth::{OpenAuth, OpenAuthBuilder};
pub use crate::db::MemoryAdapter;
pub use crate::error::OpenAuthError;
pub use crate::oauth::oauth2::SocialOAuthProvider;
pub use crate::options::{
    AdvancedOptions, EmailPasswordOptions, OpenAuthOptions, RateLimitOptions, SessionOptions,
    TrustedOriginOptions, UserOptions,
};
pub use crate::plugin::AuthPlugin;
