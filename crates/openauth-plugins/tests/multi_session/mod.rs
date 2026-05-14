#[test]
fn exposes_multi_session_placeholder() {
    assert_eq!(
        openauth_plugins::multi_session::UPSTREAM_PLUGIN_ID,
        "multi-session"
    );
}
