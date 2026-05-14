#[test]
fn exposes_one_time_token_placeholder() {
    assert_eq!(
        openauth_plugins::one_time_token::UPSTREAM_PLUGIN_ID,
        "one-time-token"
    );
}
