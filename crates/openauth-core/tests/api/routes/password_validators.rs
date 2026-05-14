use super::*;

#[tokio::test]
async fn password_validator_rejects_sign_up_before_user_creation(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let router = router_with_options(
        adapter.clone(),
        OpenAuthOptions {
            plugins: vec![rejecting_password_plugin("/sign-up/email")],
            ..OpenAuthOptions::default()
        },
    )?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: ApiErrorResponse = serde_json::from_slice(response.body())?;
    assert_eq!(body.code, "PASSWORD_COMPROMISED");
    assert_eq!(body.message, "compromised");
    assert_eq!(adapter.len("user").await, 0);
    assert_eq!(adapter.len("session").await, 0);
    Ok(())
}

#[tokio::test]
async fn password_validator_rejects_change_password_before_credential_update(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    let original_hash = hash_password("secret123")?;
    adapter
        .insert_account(credential_account_record("user_1", &original_hash, now))
        .await?;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    let router = router_with_options(
        adapter.clone(),
        OpenAuthOptions {
            plugins: vec![rejecting_password_plugin("/change-password")],
            ..OpenAuthOptions::default()
        },
    )?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/change-password",
            r#"{"currentPassword":"secret123","newPassword":"new-secret123"}"#,
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let account = record_by_string(&adapter, "account", "id", "account_1")
        .await?
        .ok_or("missing account")?;
    assert_eq!(string_field(&account, "password")?, original_hash);
    Ok(())
}

#[tokio::test]
async fn password_validator_rejects_reset_password_before_token_consumption(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    let original_hash = hash_password("secret123")?;
    adapter
        .insert_account(credential_account_record("user_1", &original_hash, now))
        .await?;
    let router = router_with_options(
        adapter.clone(),
        OpenAuthOptions {
            plugins: vec![rejecting_password_plugin("/reset-password")],
            ..OpenAuthOptions::default()
        },
    )?;

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
        .records("verification")
        .await
        .into_iter()
        .find_map(|record| string_field(&record, "identifier").ok().map(str::to_owned))
        .ok_or("missing verification")?;
    let token = identifier
        .strip_prefix("reset-password:")
        .ok_or("bad identifier")?
        .to_owned();

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/reset-password",
            &format!(r#"{{"newPassword":"new-secret123","token":"{token}"}}"#),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!adapter.is_empty("verification").await);
    let account = record_by_string(&adapter, "account", "id", "account_1")
        .await?
        .ok_or("missing account")?;
    assert_eq!(string_field(&account, "password")?, original_hash);
    Ok(())
}

#[tokio::test]
async fn password_validator_skips_unmatched_paths() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let router = router_with_options(
        adapter.clone(),
        OpenAuthOptions {
            plugins: vec![rejecting_password_plugin("/change-password")],
            ..OpenAuthOptions::default()
        },
    )?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(adapter.len("user").await, 1);
    Ok(())
}
