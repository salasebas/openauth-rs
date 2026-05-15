use std::sync::Arc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use http::{Method, StatusCode};
use openauth_plugins::mcp::McpOptions;
use serde_json::{json, Map, Value};

use super::support::*;

#[tokio::test]
async fn mcp_options_support_generators_and_additional_claims(
) -> Result<(), Box<dyn std::error::Error>> {
    let (auth, adapter) = seeded_router_with_options(McpOptions {
        login_page: "/login".to_owned(),
        client_id_generator: Some(Arc::new(|| "custom_client".to_owned())),
        client_secret_generator: Some(Arc::new(|| "custom_secret".to_owned())),
        additional_id_token_claims: Some(Arc::new(|_user, _scopes| {
            let mut claims = Map::new();
            claims.insert("tenant_id".to_owned(), json!("tenant_1"));
            Ok(claims)
        })),
        ..McpOptions::default()
    })
    .await?;

    let registered = auth
        .handle_async(json_request(
            Method::POST,
            "/api/auth/mcp/register",
            json!({
                "redirect_uris": ["https://client.example/callback"],
                "client_name": "Generated"
            }),
        )?)
        .await?;
    assert_eq!(registered.status(), StatusCode::CREATED);
    let registered = json_body(&registered)?;
    assert_eq!(registered["client_id"], "custom_client");
    assert_eq!(registered["client_secret"], "custom_secret");

    let code = seed_code(
        &adapter,
        "custom_client",
        "user_1",
        "https://client.example/callback",
        "openid profile",
        Some("verifier"),
        Some("plain"),
    )
    .await?;
    let token = auth
        .handle_async(form_request(
            Method::POST,
            "/api/auth/mcp/token",
            &format!("grant_type=authorization_code&client_id=custom_client&client_secret=custom_secret&redirect_uri=https%3A%2F%2Fclient.example%2Fcallback&code={code}&code_verifier=verifier"),
        )?)
        .await?;
    assert_eq!(token.status(), StatusCode::OK);
    let id_token = json_body(&token)?["id_token"]
        .as_str()
        .ok_or("missing id token")?
        .to_owned();
    let payload = id_token.split('.').nth(1).ok_or("missing jwt payload")?;
    let payload = URL_SAFE_NO_PAD.decode(payload)?;
    let payload: Value = serde_json::from_slice(&payload)?;
    assert_eq!(payload["tenant_id"], "tenant_1");
    assert_eq!(payload["name"], "Ada Lovelace");
    assert!(payload.get("email").is_none());
    Ok(())
}
