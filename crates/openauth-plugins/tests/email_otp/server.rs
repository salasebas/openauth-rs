use std::sync::Arc;

use openauth_core::db::MemoryAdapter;
use openauth_plugins::email_otp::EmailOtpOptions;

use super::common::*;

#[tokio::test]
async fn server_create_and_get_otp_returns_recoverable_plain_value() {
    let adapter = Arc::new(MemoryAdapter::new());
    let sender = CaptureSender::default();
    let router = router(adapter.clone(), sender, EmailOtpOptions::default()).unwrap();

    let create = router
        .handle_async(
            json_request(
                "/email-otp/create-verification-otp",
                r#"{"email":"ada@example.com","type":"email-verification"}"#,
                None,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let create_body: Value = serde_json::from_slice(create.body()).unwrap();
    let otp = create_body["otp"].as_str().unwrap();
    assert!(
        verification_value(&adapter, "email-verification-otp-ada@example.com")
            .await
            .is_some()
    );

    let get = router
        .handle_async(
            get_json_request(
                "/email-otp/get-verification-otp",
                r#"{"email":"ada@example.com","type":"email-verification"}"#,
                None,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let get_body: Value = serde_json::from_slice(get.body()).unwrap();

    assert_eq!(create.status(), StatusCode::OK);
    assert_eq!(get_body["otp"], otp);
}
