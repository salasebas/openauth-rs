#[test]
fn exposes_oauth_proxy_placeholder() {
    assert_eq!(
        openauth_plugins::oauth_proxy::UPSTREAM_PLUGIN_ID,
        "oauth-proxy"
    );
}
