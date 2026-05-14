#[test]
fn exposes_anonymous_placeholder() {
    assert_eq!(openauth_plugins::anonymous::UPSTREAM_PLUGIN_ID, "anonymous");
}
