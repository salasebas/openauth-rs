#[test]
fn exposes_jwt_placeholder() {
    assert_eq!(openauth_plugins::jwt::UPSTREAM_PLUGIN_ID, "jwt");
}
