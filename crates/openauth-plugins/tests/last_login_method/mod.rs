#[test]
fn exposes_last_login_method_placeholder() {
    assert_eq!(
        openauth_plugins::last_login_method::UPSTREAM_PLUGIN_ID,
        "last-login-method"
    );
}
