use super::*;

#[tokio::test]
async fn reset_password_route_updates_password_and_consumes_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_account(credential_account_record(
            "user_1",
            &hash_password("secret123")?,
            now,
        ))
        .await?;
    let router = router(adapter.clone())?;

    let request_response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/request-password-reset",
            r#"{"email":"ada@example.com","redirectTo":"/reset"}"#,
            None,
        )?)
        .await?;
    assert_eq!(request_response.status(), StatusCode::OK);
    let identifier = adapter
        .verifications
        .lock()
        .await
        .keys()
        .next()
        .cloned()
        .ok_or("missing verification")?;
    let token = identifier
        .strip_prefix("reset-password:")
        .ok_or("bad identifier")?
        .to_owned();

    let reset_response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/reset-password",
            &format!(r#"{{"newPassword":"new-secret123","token":"{token}"}}"#),
            None,
        )?)
        .await?;

    assert_eq!(reset_response.status(), StatusCode::OK);
    assert!(adapter.verifications.lock().await.is_empty());
    let accounts = adapter.accounts.lock().await;
    let account = accounts.get("account_1").ok_or("missing account")?;
    let hash = string_field(account, "password")?;
    assert!(openauth_core::crypto::password::verify_password(
        hash,
        "new-secret123"
    )?);

    let reused_response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/reset-password",
            &format!(r#"{{"newPassword":"another-secret123","token":"{token}"}}"#),
            None,
        )?)
        .await?;
    assert_eq!(reused_response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
