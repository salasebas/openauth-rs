#[test]
fn exposes_admin_placeholder() {
    assert_eq!(openauth_plugins::admin::UPSTREAM_PLUGIN_ID, "admin");
}
