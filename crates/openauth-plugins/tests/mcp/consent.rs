use http::{header, Method, Request, StatusCode};
use serde_json::json;

use super::support::*;

async fn consent_code() -> Result<
    (
        openauth_core::api::AuthRouter,
        std::sync::Arc<openauth_core::db::MemoryAdapter>,
        String,
        String,
    ),
    Box<dyn std::error::Error>,
> {
    let (auth, adapter) = seeded_router_with_options(openauth_plugins::mcp::McpOptions {
        login_page: "/login".to_owned(),
        consent_page: Some("/consent".to_owned()),
        ..openauth_plugins::mcp::McpOptions::default()
    })
    .await?;
    seed_client(
        &adapter,
        "client_1",
        "secret_1",
        "https://client.example/callback",
        "web",
    )
    .await?;
    let cookie = signed_session_cookie("session_token_1")?;
    let response = auth
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("http://localhost:3000/api/auth/mcp/authorize?client_id=client_1&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&response_type=code&scope=openid%20email&state=state_1&prompt=consent")
                .header(header::COOKIE, cookie.clone())
                .body(Vec::new())?,
        )
        .await?;
    let location = response.headers()[header::LOCATION].to_str()?;
    let code = url::Url::parse(&format!("http://localhost{location}"))?
        .query_pairs()
        .find_map(|(name, value)| (name == "consent_code").then(|| value.into_owned()))
        .ok_or("missing consent code")?;
    Ok((auth, adapter, cookie, code))
}

#[tokio::test]
async fn consent_accept_returns_json_redirect_and_rotates_code(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter, cookie, code) = consent_code().await?;
    let response = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/oauth2/consent")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(serde_json::to_vec(&json!({
                    "accept": true,
                    "consent_code": code
                }))?)?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let redirect = json_body(&response)?["redirectURI"]
        .as_str()
        .ok_or("missing redirectURI")?
        .to_owned();
    assert!(redirect.starts_with("https://client.example/callback?code="));
    assert!(redirect.contains("state=state_1"));
    let new_code = url::Url::parse(&redirect)?
        .query_pairs()
        .find_map(|(name, value)| (name == "code").then(|| value.into_owned()))
        .ok_or("missing code")?;
    assert_ne!(new_code, code);
    assert!(find_record(&adapter, "verification", "identifier", &code)
        .await?
        .is_none());
    assert!(
        find_record(&adapter, "verification", "identifier", &new_code)
            .await?
            .is_some()
    );
    assert!(find_record(&adapter, "oauthConsent", "userId", "user_1")
        .await?
        .is_some());
    Ok(())
}

#[tokio::test]
async fn consent_reject_deletes_code_and_returns_access_denied_redirect(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter, cookie, code) = consent_code().await?;
    let response = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/oauth2/consent")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(serde_json::to_vec(&json!({
                    "accept": false,
                    "consent_code": code
                }))?)?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(json_body(&response)?["redirectURI"]
        .as_str()
        .ok_or("missing redirectURI")?
        .contains("error=access_denied"));
    assert!(find_record(&adapter, "verification", "identifier", &code)
        .await?
        .is_none());
    Ok(())
}

#[tokio::test]
async fn consent_rejects_missing_mismatched_expired_and_not_required(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter, cookie, code) = consent_code().await?;

    let missing_session = auth
        .handle_async(json_request(
            Method::POST,
            "/api/auth/oauth2/consent",
            json!({ "accept": true, "consent_code": code }),
        )?)
        .await?;
    assert_eq!(missing_session.status(), StatusCode::UNAUTHORIZED);

    seed_session_for_user(&adapter, "session_other", "user_2").await?;
    let mismatched = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/oauth2/consent")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, signed_session_cookie("session_other")?)
                .body(serde_json::to_vec(&json!({
                    "accept": true,
                    "consent_code": code
                }))?)?,
        )
        .await?;
    assert_eq!(mismatched.status(), StatusCode::FORBIDDEN);

    let expired = seed_expired_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
    )
    .await?;
    let expired_response = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/oauth2/consent")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie.clone())
                .body(serde_json::to_vec(&json!({
                    "accept": true,
                    "consent_code": expired
                }))?)?,
        )
        .await?;
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);

    let not_required = seed_code(
        &adapter,
        "client_1",
        "user_1",
        "https://client.example/callback",
        "openid",
        None,
        None,
    )
    .await?;
    let not_required_response = auth
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("http://localhost:3000/api/auth/oauth2/consent")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(serde_json::to_vec(&json!({
                    "accept": true,
                    "consent_code": not_required
                }))?)?,
        )
        .await?;
    assert_eq!(not_required_response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}
