#[test]
fn exposes_additional_fields_placeholder() {
    assert_eq!(
        openauth_plugins::additional_fields::UPSTREAM_PLUGIN_ID,
        "additional-fields"
    );
}
