use http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};

use super::support::*;

#[tokio::test]
async fn userinfo_returns_scope_gated_claims_and_jwks_is_empty(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_access_token(
        &adapter,
        "access_profile",
        "refresh_profile",
        "client_1",
        "user_1",
        "openid profile",
    )
    .await?;

    let userinfo = auth
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("http://localhost:3000/api/auth/mcp/userinfo")
                .header(header::AUTHORIZATION, "Bearer access_profile")
                .body(Vec::new())?,
        )
        .await?;
    assert_eq!(userinfo.status(), StatusCode::OK);
    let body = json_body(&userinfo)?;
    assert_eq!(body["sub"], "user_1");
    assert_eq!(body["name"], "Ada Lovelace");
    assert_eq!(body["given_name"], "Ada");
    assert!(body.get("email").is_none());

    let jwks = auth
        .handle_async(request(Method::GET, "/api/auth/mcp/jwks", "")?)
        .await?;
    assert_eq!(jwks.status(), StatusCode::OK);
    assert_eq!(json_body(&jwks)?["keys"], json!([]));
    assert_eq!(jwks.headers()[header::CACHE_CONTROL], "no-store");
    Ok(())
}

#[tokio::test]
async fn userinfo_rejects_missing_invalid_and_expired_bearer(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router().await?;
    seed_access_token(
        &adapter,
        "access_expired",
        "refresh_expired",
        "client_1",
        "user_1",
        "openid email",
    )
    .await?;
    expire_access_token(&adapter, "access_expired").await?;

    let missing = auth
        .handle_async(request(Method::GET, "/api/auth/mcp/userinfo", "")?)
        .await?;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(&missing)?["error"], "invalid_request");

    let invalid = auth
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("http://localhost:3000/api/auth/mcp/userinfo")
                .header(header::AUTHORIZATION, "Bearer missing")
                .body(Vec::new())?,
        )
        .await?;
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(&invalid)?["error"], "invalid_token");

    let expired = auth
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("http://localhost:3000/api/auth/mcp/userinfo")
                .header(header::AUTHORIZATION, "Bearer access_expired")
                .body(Vec::new())?,
        )
        .await?;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json_body(&expired)?["error"], "invalid_token");

    let session = auth
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("http://localhost:3000/api/auth/mcp/get-session")
                .header(header::AUTHORIZATION, "Bearer access_expired")
                .body(Vec::new())?,
        )
        .await?;
    assert_eq!(session.status(), StatusCode::OK);
    assert_eq!(json_body(&session)?, Value::Null);
    Ok(())
}
