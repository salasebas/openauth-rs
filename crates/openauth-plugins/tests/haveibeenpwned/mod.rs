#[test]
fn exposes_haveibeenpwned_placeholder() {
    assert_eq!(
        openauth_plugins::haveibeenpwned::UPSTREAM_PLUGIN_ID,
        "haveibeenpwned"
    );
}
