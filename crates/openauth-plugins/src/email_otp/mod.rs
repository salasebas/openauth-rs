//! Email OTP plugin.

mod endpoints;
mod helpers;
mod otp;
mod registry;
mod response;
mod schema;
mod types;

pub use types::{
    ChangeEmailOptions, EmailOtpGenerator, EmailOtpOptions, EmailOtpPayload, EmailOtpType,
    OtpStorage, ResendStrategy, SendEmailOtp,
};

use openauth_core::options::RateLimitRule;
use openauth_core::plugin::{AuthPlugin, PluginRateLimitRule};

pub const UPSTREAM_PLUGIN_ID: &str = "email-otp";

/// Build the Email OTP plugin.
pub fn email_otp(options: EmailOtpOptions) -> AuthPlugin {
    let rate_limit = options
        .rate_limit
        .clone()
        .unwrap_or(RateLimitRule { window: 60, max: 3 });
    registry::paths().iter().fold(
        registry::register(
            AuthPlugin::new(UPSTREAM_PLUGIN_ID).with_version(crate::VERSION),
            options,
        ),
        |plugin: AuthPlugin, path| {
            plugin.with_rate_limit(PluginRateLimitRule::new(*path, rate_limit.clone()))
        },
    )
}
