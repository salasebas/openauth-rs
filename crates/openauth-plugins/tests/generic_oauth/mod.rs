#[test]
fn exposes_generic_oauth_placeholder() {
    assert_eq!(
        openauth_plugins::generic_oauth::UPSTREAM_PLUGIN_ID,
        "generic-oauth"
    );
}
