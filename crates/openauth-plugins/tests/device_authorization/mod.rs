#[test]
fn exposes_device_authorization_placeholder() {
    assert_eq!(
        openauth_plugins::device_authorization::UPSTREAM_PLUGIN_ID,
        "device-authorization"
    );
}
