#[test]
fn exposes_siwe_placeholder() {
    assert_eq!(openauth_plugins::siwe::UPSTREAM_PLUGIN_ID, "siwe");
}
