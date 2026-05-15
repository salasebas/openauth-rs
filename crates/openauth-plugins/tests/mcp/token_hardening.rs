use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use http::{header, Method, Request, StatusCode};

use super::support::*;

#[tokio::test]
async fn token_supports_client_secret_basic_and_rejects_invalid_basic(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_client(
        &adapter,
        "client_1",
        "secret_1",
        "https://client.example/callback",
        "web",
    )
    .await?;
    let code = seed_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
        Some("verifier"),
        Some("plain"),
    )
    .await?;
    let valid = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/mcp/token")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(
                    header::AUTHORIZATION,
                    format!("Basic {}", STANDARD.encode("client_1:secret_1")),
                )
                .body(format!("grant_type=authorization_code&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier").into_bytes())?,
        )
        .await?;
    assert_eq!(valid.status(), StatusCode::OK);

    let code = seed_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
        Some("verifier"),
        Some("plain"),
    )
    .await?;
    let invalid = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/mcp/token")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(
                    header::AUTHORIZATION,
                    format!("Basic {}", STANDARD.encode("client_1:wrong")),
                )
                .body(format!("grant_type=authorization_code&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier").into_bytes())?,
        )
        .await?;
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn token_consumes_code_on_missing_client_and_expired_code(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_client(
        &adapter,
        "client_1",
        "secret_1",
        "https://client.example/callback",
        "web",
    )
    .await?;
    let code = seed_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
        Some("verifier"),
        Some("plain"),
    )
    .await?;
    let missing_client = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier"),
        )?)
        .await?;
    assert_eq!(missing_client.status(), StatusCode::BAD_REQUEST);
    assert!(find_record(&adapter, "verification", "identifier", &code)
        .await?
        .is_none());

    let expired = seed_expired_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
    )
    .await?;
    let expired_response = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=client_1&client_secret=secret_1&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={expired}"),
        )?)
        .await?;
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        find_record(&adapter, "verification", "identifier", &expired)
            .await?
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn public_client_requires_stored_pkce_challenge() -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_client(
        &adapter,
        "public_1",
        "unused",
        "https://client.example/callback",
        "public",
    )
    .await?;
    let code = seed_code(
        &adapter,
        "public_1",
        "user_1",
        "https://client.example/callback",
        "openid",
        None,
        None,
    )
    .await?;
    let response = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=public_1&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=anything"),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}
