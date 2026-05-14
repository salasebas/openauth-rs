#[test]
fn exposes_username_placeholder() {
    assert_eq!(openauth_plugins::username::UPSTREAM_PLUGIN_ID, "username");
}
