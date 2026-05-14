#[test]
fn exposes_open_api_placeholder() {
    assert_eq!(openauth_plugins::open_api::UPSTREAM_PLUGIN_ID, "open-api");
}
