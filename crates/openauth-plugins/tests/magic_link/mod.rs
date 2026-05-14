#[test]
fn exposes_magic_link_placeholder() {
    assert_eq!(
        openauth_plugins::magic_link::UPSTREAM_PLUGIN_ID,
        "magic-link"
    );
}
