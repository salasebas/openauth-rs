#[test]
fn exposes_oidc_provider_placeholder() {
    assert_eq!(
        openauth_plugins::oidc_provider::UPSTREAM_PLUGIN_ID,
        "oidc-provider"
    );
}
