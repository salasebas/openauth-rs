#[test]
fn exposes_access_placeholder() {
    assert_eq!(openauth_plugins::access::UPSTREAM_PLUGIN_ID, "access");
}
