use openauth_core::options::{EmailPasswordOptions, OpenAuthOptions};
use openauth_core::test_utils::apply_fast_password_defaults;

#[allow(dead_code, unused_imports)]
pub use openauth_core::test_utils::{fast_hash_password, real_password_options};

/// Apply development defaults for integration tests unless production mode is
/// explicitly requested.
#[allow(dead_code)]
pub fn with_test_defaults(mut options: OpenAuthOptions) -> OpenAuthOptions {
    if !options.production {
        options.development = true;
    }
    if !options.email_password.enabled {
        options.email_password = EmailPasswordOptions::new().enabled(true);
    }
    apply_fast_password_defaults(options)
}
