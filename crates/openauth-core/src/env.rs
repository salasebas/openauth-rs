//! Environment helpers for OpenAuth core.

pub mod logger;

/// Returns true when OpenAuth is running in a production environment.
pub fn is_production() -> bool {
    std::env::var("RUST_ENV").is_ok_and(|value| value == "production")
}
