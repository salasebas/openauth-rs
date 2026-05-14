use std::sync::Arc;

use openauth_core::db::MemoryAdapter;
use openauth_plugins::email_otp::{EmailOtpOptions, OtpStorage, ResendStrategy};

use super::common::*;

#[tokio::test]
async fn encrypted_storage_is_not_plain_and_can_be_reused() {
    let adapter = Arc::new(MemoryAdapter::new());
    create_user(&adapter, "ada@example.com", false).await;
    let sender = CaptureSender::default();
    let router = router(
        adapter.clone(),
        sender.clone(),
        EmailOtpOptions {
            store_otp: OtpStorage::Encrypted,
            resend_strategy: ResendStrategy::Reuse,
            ..EmailOtpOptions::default()
        },
    )
    .unwrap();

    for _ in 0..2 {
        router
            .handle_async(
                json_request(
                    "/email-otp/send-verification-otp",
                    r#"{"email":"ada@example.com","type":"email-verification"}"#,
                    None,
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }
    let otp = sender.last_otp();
    let stored = verification_value(&adapter, "email-verification-otp-ada@example.com")
        .await
        .unwrap();
    let response = router
        .handle_async(
            json_request(
                "/email-otp/check-verification-otp",
                &format!(
                    r#"{{"email":"ada@example.com","type":"email-verification","otp":"{otp}"}}"#
                ),
                None,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(sender.count(), 2);
    assert!(!stored.starts_with(&otp));
    assert_eq!(response.status(), StatusCode::OK);
}
