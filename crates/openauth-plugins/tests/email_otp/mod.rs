#[test]
fn exposes_email_otp_placeholder() {
    assert_eq!(openauth_plugins::email_otp::UPSTREAM_PLUGIN_ID, "email-otp");
}
