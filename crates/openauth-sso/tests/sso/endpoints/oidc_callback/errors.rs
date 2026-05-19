use super::*;

#[tokio::test]
async fn oidc_callback_redirects_provider_error_to_state_error_url(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router) = router_with_options(SsoOptions::default())?;
    let cookie = seed_session(&adapter).await?;
    register_oidc_provider(&router, &cookie).await?;

    let sign_in = router
        .handle_async(json_request(
            Method::POST,
            "/sign-in/sso",
            r#"{"providerId":"okta","callbackURL":"/dashboard","errorCallbackURL":"/login-error"}"#,
            None,
        )?)
        .await?;
    let state = authorization_state(sign_in)?;

    let callback = router
        .handle_async(json_request(
            Method::GET,
            &format!("/sso/callback/okta?state={state}&error=access_denied"),
            "",
            None,
        )?)
        .await?;

    assert_eq!(callback.status(), StatusCode::FOUND);
    assert_eq!(
        callback.headers().get(http::header::LOCATION),
        Some(&http::HeaderValue::from_static(
            "/login-error?error=access_denied"
        ))
    );

    Ok(())
}

#[tokio::test]
async fn oidc_callback_redirects_no_code_to_state_error_url(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router) = router_with_options(SsoOptions::default())?;
    let cookie = seed_session(&adapter).await?;
    register_oidc_provider(&router, &cookie).await?;

    let sign_in = router
        .handle_async(json_request(
            Method::POST,
            "/sign-in/sso",
            r#"{"providerId":"okta","callbackURL":"/dashboard","errorCallbackURL":"/login-error"}"#,
            None,
        )?)
        .await?;
    let state = authorization_state(sign_in)?;

    let callback = router
        .handle_async(json_request(
            Method::GET,
            &format!("/sso/callback/okta?state={state}"),
            "",
            None,
        )?)
        .await?;

    assert_eq!(callback.status(), StatusCode::FOUND);
    assert_eq!(
        callback.headers().get(http::header::LOCATION),
        Some(&http::HeaderValue::from_static(
            "/login-error?error=no_code"
        ))
    );

    Ok(())
}

#[tokio::test]
async fn oidc_callback_exchanges_code_creates_session_and_redirects(
) -> Result<(), Box<dyn std::error::Error>> {
    let oidc = MockOidcServer::start().await?;
    let (adapter, router) = router_with_options(SsoOptions::default())?;
    let cookie = seed_session(&adapter).await?;
    register_oidc_provider_with_endpoints(&router, &cookie, &oidc.base_url).await?;

    let sign_in = router
        .handle_async(json_request(
            Method::POST,
            "/sign-in/sso",
            r#"{"providerId":"okta","callbackURL":"/dashboard","errorCallbackURL":"/login-error"}"#,
            None,
        )?)
        .await?;
    let state = authorization_state(sign_in)?;

    let callback = router
        .handle_async(json_request(
            Method::GET,
            &format!("/sso/callback/okta?state={state}&code=auth-code"),
            "",
            None,
        )?)
        .await?;

    assert_eq!(callback.status(), StatusCode::FOUND);
    assert_eq!(
        callback.headers().get(header::LOCATION),
        Some(&http::HeaderValue::from_static("/dashboard"))
    );
    assert!(callback.headers().get(header::SET_COOKIE).is_some());

    let users = adapter.records("user").await;
    assert!(users.iter().any(|record| {
        record.get("email") == Some(&DbValue::String("sso-user@example.com".to_owned()))
    }));
    let accounts = adapter.records("account").await;
    assert!(accounts.iter().any(|record| {
        record.get("provider_id") == Some(&DbValue::String("okta".to_owned()))
            && record.get("account_id") == Some(&DbValue::String("subject_123".to_owned()))
    }));

    Ok(())
}
