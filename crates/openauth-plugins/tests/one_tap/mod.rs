#[test]
fn exposes_one_tap_placeholder() {
    assert_eq!(openauth_plugins::one_tap::UPSTREAM_PLUGIN_ID, "one-tap");
}
