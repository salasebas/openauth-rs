#[test]
fn exposes_captcha_placeholder() {
    assert_eq!(openauth_plugins::captcha::UPSTREAM_PLUGIN_ID, "captcha");
}
