#[test]
fn exposes_bearer_placeholder() {
    assert_eq!(openauth_plugins::bearer::UPSTREAM_PLUGIN_ID, "bearer");
}
