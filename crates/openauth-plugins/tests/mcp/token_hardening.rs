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
    assert_oauth_error(&missing_client, StatusCode::UNAUTHORIZED, "invalid_client")?;
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
    assert_oauth_error(&expired_response, StatusCode::UNAUTHORIZED, "invalid_grant")?;
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
    assert_oauth_error(&response, StatusCode::BAD_REQUEST, "invalid_request")?;
    Ok(())
}

#[tokio::test]
async fn token_returns_precise_oauth_errors_for_code_exchange_failures(
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

    let invalid_code = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=authorization_code&client_id=client_1&client_secret=secret_1&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code=missing",
        )?)
        .await?;
    assert_oauth_error(&invalid_code, StatusCode::UNAUTHORIZED, "invalid_grant")?;

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
    let invalid_secret = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=client_1&client_secret=wrong&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier"),
        )?)
        .await?;
    assert_oauth_error(&invalid_secret, StatusCode::UNAUTHORIZED, "invalid_client")?;

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
    let invalid_client = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=unknown&client_secret=secret_1&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier"),
        )?)
        .await?;
    assert_oauth_error(&invalid_client, StatusCode::UNAUTHORIZED, "invalid_client")?;

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
    let invalid_redirect = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=client_1&client_secret=secret_1&redirect_uri=https%3A%2F%2Fevil.example%2Fcallback&code={code}&code_verifier=verifier"),
        )?)
        .await?;
    assert_oauth_error(
        &invalid_redirect,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    )?;
    Ok(())
}

#[tokio::test]
async fn refresh_token_confidential_client_rejects_missing_and_wrong_secret(
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
    seed_access_token(
        &adapter,
        "access_1",
        "refresh_1",
        "client_1",
        "user_1",
        "openid",
    )
    .await?;

    let missing_secret = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=client_1&refresh_token=refresh_1",
        )?)
        .await?;
    assert_oauth_error(&missing_secret, StatusCode::UNAUTHORIZED, "invalid_client")?;

    let wrong_secret = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=client_1&client_secret=wrong&refresh_token=refresh_1",
        )?)
        .await?;
    assert_oauth_error(&wrong_secret, StatusCode::UNAUTHORIZED, "invalid_client")?;
    Ok(())
}

#[tokio::test]
async fn refresh_token_confidential_client_accepts_post_and_basic_secret(
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
    seed_access_token(
        &adapter,
        "access_1",
        "refresh_1",
        "client_1",
        "user_1",
        "openid",
    )
    .await?;
    seed_access_token(
        &adapter,
        "access_2",
        "refresh_2",
        "client_1",
        "user_1",
        "openid",
    )
    .await?;

    let post = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=client_1&client_secret=secret_1&refresh_token=refresh_1",
        )?)
        .await?;
    assert_eq!(post.status(), StatusCode::OK);
    assert!(json_body(&post)?["access_token"].as_str().is_some());

    let basic = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/mcp/token")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(
                    header::AUTHORIZATION,
                    format!("Basic {}", STANDARD.encode("client_1:secret_1")),
                )
                .body(b"grant_type=refresh_token&refresh_token=refresh_2".to_vec())?,
        )
        .await?;
    assert_eq!(basic.status(), StatusCode::OK);
    assert!(json_body(&basic)?["access_token"].as_str().is_some());
    Ok(())
}

#[tokio::test]
async fn refresh_token_rotation_invalidates_presented_token(
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
    seed_access_token(
        &adapter,
        "access_1",
        "refresh_1",
        "client_1",
        "user_1",
        "openid",
    )
    .await?;

    let first = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=client_1&client_secret=secret_1&refresh_token=refresh_1",
        )?)
        .await?;
    assert_eq!(first.status(), StatusCode::OK);
    let refresh_2 = json_body(&first)?["refresh_token"]
        .as_str()
        .ok_or("missing refresh token")?
        .to_owned();

    let replay = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=client_1&client_secret=secret_1&refresh_token=refresh_1",
        )?)
        .await?;
    assert_oauth_error(&replay, StatusCode::UNAUTHORIZED, "invalid_grant")?;

    let second = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!(
                "grant_type=refresh_token&client_id=client_1&client_secret=secret_1&refresh_token={refresh_2}"
            ),
        )?)
        .await?;
    assert_eq!(second.status(), StatusCode::OK);
    assert!(json_body(&second)?["access_token"].as_str().is_some());
    Ok(())
}

#[tokio::test]
async fn refresh_token_concurrent_rotation_allows_single_success(
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
    seed_access_token(
        &adapter,
        "access_1",
        "refresh_1",
        "client_1",
        "user_1",
        "openid",
    )
    .await?;

    let auth = std::sync::Arc::new(auth);
    let request_body = "grant_type=refresh_token&client_id=client_1&client_secret=secret_1&refresh_token=refresh_1";
    let auth_left = auth.clone();
    let auth_right = auth.clone();
    let (left, right) = tokio::join!(
        auth_left.handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            request_body,
        )?),
        auth_right.handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            request_body,
        )?)
    );

    let successes = [left?, right?]
        .iter()
        .filter(|response| response.status() == StatusCode::OK)
        .count();
    assert_eq!(successes, 1);
    Ok(())
}

#[tokio::test]
async fn refresh_token_public_client_succeeds_without_secret(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_client(
        &adapter,
        "public_1",
        "unused",
        "https://client.example/callback",
        "public",
    )
    .await?;
    seed_access_token(
        &adapter,
        "access_1",
        "refresh_1",
        "public_1",
        "user_1",
        "openid",
    )
    .await?;

    let response = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            "grant_type=refresh_token&client_id=public_1&refresh_token=refresh_1",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(json_body(&response)?["access_token"].as_str().is_some());
    Ok(())
}

fn assert_oauth_error(
    response: &http::Response<Vec<u8>>,
    status: StatusCode,
    code: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(response.status(), status);
    assert_eq!(json_body(response)?["error"], code);
    Ok(())
}
