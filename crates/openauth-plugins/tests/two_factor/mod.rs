#[test]
fn exposes_two_factor_placeholder() {
    assert_eq!(
        openauth_plugins::two_factor::UPSTREAM_PLUGIN_ID,
        "two-factor"
    );
}
