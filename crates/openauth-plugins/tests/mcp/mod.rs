#[test]
fn exposes_mcp_placeholder() {
    assert_eq!(openauth_plugins::mcp::UPSTREAM_PLUGIN_ID, "mcp");
}
