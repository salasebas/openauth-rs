#[test]
fn exposes_organization_placeholder() {
    assert_eq!(
        openauth_plugins::organization::UPSTREAM_PLUGIN_ID,
        "organization"
    );
}
