use http::{header, Method, Request, Response, StatusCode};
use openauth_core::cookies::{get_cookies, set_session_cookie, SessionCookieOptions};
use openauth_core::plugin::PluginAfterHookAction;
use openauth_plugins::mcp::McpOptions;
use serde_json::json;

use super::support::*;

#[tokio::test]
async fn login_resume_after_hook_authorizes_and_expires_prompt_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let (context, plugin, adapter) = context_with_plugin(McpOptions {
        login_page: "/login".to_owned(),
        ..McpOptions::default()
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
    let prompt = serde_json::to_string(&json!({
        "client_id": "client_1",
        "redirect_uri": "https://client.example/callback",
        "response_type": "code",
        "prompt": "login consent",
        "scope": "openid"
    }))?;
    let request = Request::builder()
        .method(Method::POST)
        .uri("http://localhost:3000/api/auth/sign-in/email")
        .header(
            header::COOKIE,
            format!("oidc_login_prompt={}", signed_cookie_value(&prompt)?),
        )
        .body(Vec::new())?;
    let auth_cookies = get_cookies(&openauth_core::options::OpenAuthOptions {
        secret: Some("test-secret-123456789012345678901234".to_owned()),
        ..openauth_core::options::OpenAuthOptions::default()
    })?;
    let session_cookie = set_session_cookie(
        &auth_cookies,
        "test-secret-123456789012345678901234",
        "session_token_1",
        SessionCookieOptions::default(),
    )?
    .into_iter()
    .next()
    .ok_or("missing session cookie")?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            header::SET_COOKIE,
            format!(
                "{}={}; Path=/; HttpOnly",
                session_cookie.name, session_cookie.value
            ),
        )
        .body(Vec::new())?;
    let hook = &plugin.hooks.async_after[0].handler;
    let PluginAfterHookAction::Continue(response) = hook(&context, &request, response).await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(response.headers()[header::LOCATION]
        .to_str()?
        .starts_with("https://client.example/callback?code="));
    let set_cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>()
        .join(",");
    assert!(set_cookies.contains("oidc_login_prompt="));
    assert!(set_cookies.contains("Max-Age=0"));
    Ok(())
}
