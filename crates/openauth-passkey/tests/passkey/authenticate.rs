use http::{Method, StatusCode};
use openauth_passkey::{PasskeyAuthenticationOptions, PasskeyOptions};
use serde_json::{json, Value};

use crate::support::{
    cookie_header_from_response, empty_request, join_cookies, json_request, seed_passkey,
    seed_user_two, seeded_router, set_cookie_values, sign_in_cookie,
};

#[tokio::test]
async fn generate_authenticate_options_without_session_returns_discoverable_options(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-authenticate-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["challenge"], "authentication-challenge");
    assert_eq!(body["rpId"], "localhost");
    assert!(body.get("allowCredentials").is_none());
    Ok(())
}

#[tokio::test]
async fn generate_authenticate_options_with_session_includes_user_credentials(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;
    seed_passkey(
        adapter.as_ref(),
        "passkey_1",
        "user_1",
        "Laptop",
        "credential-id",
    )
    .await?;
    let session_cookie = sign_in_cookie(&router).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-authenticate-options",
            Some(&session_cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["allowCredentials"][0]["id"], "credential-id");
    Ok(())
}

#[tokio::test]
async fn generate_authenticate_options_includes_user_verification_and_extensions(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().authentication(
        PasskeyAuthenticationOptions::new()
            .extensions(json!({ "appid": "https://legacy.example.com" })),
    );
    let (_adapter, router, _backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-authenticate-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["userVerification"], "preferred");
    assert_eq!(body["extensions"]["appid"], "https://legacy.example.com");
    Ok(())
}

#[tokio::test]
async fn verify_authentication_creates_session_and_returns_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;
    seed_passkey(
        adapter.as_ref(),
        "passkey_1",
        "user_1",
        "Laptop",
        "credential-id",
    )
    .await?;
    let options_response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-authenticate-options",
            None,
        )?)
        .await?;
    let passkey_cookie = cookie_header_from_response(&options_response);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/verify-authentication",
            r#"{"response":{"id":"credential-id"}}"#,
            Some(&passkey_cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["user"]["id"], "user_1");
    assert!(body["session"]["token"]
        .as_str()
        .is_some_and(|token| !token.is_empty()));
    assert!(set_cookie_values(&response)
        .iter()
        .any(|cookie| cookie.contains("session_token")));
    Ok(())
}

#[tokio::test]
async fn verify_authentication_rejects_credential_outside_session_challenge(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;
    seed_user_two(adapter.as_ref()).await?;
    seed_passkey(
        adapter.as_ref(),
        "passkey_2",
        "user_2",
        "Other Laptop",
        "credential-user-2",
    )
    .await?;
    let session_cookie = sign_in_cookie(&router).await?;
    let options_response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-authenticate-options",
            Some(&session_cookie),
        )?)
        .await?;
    let passkey_cookie = cookie_header_from_response(&options_response);
    let cookie = join_cookies(&[session_cookie.as_str(), passkey_cookie.as_str()]);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/verify-authentication",
            r#"{"response":{"id":"credential-user-2"}}"#,
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}
