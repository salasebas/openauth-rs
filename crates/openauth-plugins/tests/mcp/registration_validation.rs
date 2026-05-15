use http::{Method, StatusCode};
use serde_json::json;

use super::support::*;

#[tokio::test]
async fn registration_validates_redirect_uri_shape_and_scheme(
) -> Result<(), Box<dyn std::error::Error>> {
    let auth = router().await?;
    for redirect_uri in [
        "",
        "not-a-url",
        "https://client.example/callback#fragment",
        "http://client.example/callback",
    ] {
        let response = auth
            .handle_async(json_request(
                Method::POST,
                "/api/auth/mcp/register",
                json!({ "redirect_uris": [redirect_uri] }),
            )?)
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(&response)?["error"], "invalid_redirect_uri");
    }

    let localhost = auth
        .handle_async(json_request(
            Method::POST,
            "/api/auth/mcp/register",
            json!({ "redirect_uris": ["http://localhost:3000/callback"] }),
        )?)
        .await?;
    assert_eq!(localhost.status(), StatusCode::CREATED);
    Ok(())
}
