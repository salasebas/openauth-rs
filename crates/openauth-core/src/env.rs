//! Environment helpers for OpenAuth core.

pub mod logger;

/// Returns true when `NODE_ENV`-style environment state is production.
pub fn is_production() -> bool {
    std::env::var("NODE_ENV").is_ok_and(|value| value == "production")
}
