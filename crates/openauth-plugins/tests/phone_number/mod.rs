#[test]
fn exposes_phone_number_placeholder() {
    assert_eq!(
        openauth_plugins::phone_number::UPSTREAM_PLUGIN_ID,
        "phone-number"
    );
}
