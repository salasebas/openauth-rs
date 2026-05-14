#[test]
fn exposes_custom_session_placeholder() {
    assert_eq!(
        openauth_plugins::custom_session::UPSTREAM_PLUGIN_ID,
        "custom-session"
    );
}
