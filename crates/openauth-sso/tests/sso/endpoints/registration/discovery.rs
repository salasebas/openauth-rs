use super::*;

#[tokio::test]
async fn register_discovers_oidc_endpoints_when_skip_discovery_is_false(
) -> Result<(), Box<dyn std::error::Error>> {
    let oidc = MockOidcServer::start().await?;
    let (adapter, router) = router_with_options_and_trusted_origins(
        SsoOptions::default(),
        vec![oidc.base_url.clone()],
    )?;
    let cookie = seed_session(&adapter).await?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/register",
            &format!(
                r#"{{
                "providerId":"okta",
                "issuer":"{}",
                "domain":"example.com",
                "oidcConfig":{{
                    "clientId":"client_123456",
                    "clientSecret":"super-secret",
                    "pkce":true
                }}
            }}"#,
                oidc.base_url
            ),
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response)?;
    assert_eq!(
        body["oidcConfig"]["authorizationEndpoint"],
        format!("{}/authorize", oidc.base_url)
    );
    assert_eq!(
        body["oidcConfig"]["tokenEndpoint"],
        format!("{}/token", oidc.base_url)
    );
    assert_eq!(
        body["oidcConfig"]["jwksEndpoint"],
        format!("{}/keys", oidc.base_url)
    );

    let records = adapter.records("ssoProvider").await;
    let Some(DbValue::String(config)) = records[0].get("oidcConfig") else {
        return Err("missing stored OIDC config".into());
    };
    assert!(config.contains(&format!(
        r#""authorizationEndpoint":"{}/authorize""#,
        oidc.base_url
    )));
    assert!(config.contains(r#""tokenEndpointAuthentication":"client_secret_basic""#));

    Ok(())
}

#[tokio::test]
async fn register_returns_stable_oidc_discovery_error_code(
) -> Result<(), Box<dyn std::error::Error>> {
    let oidc = MockOidcServer::start().await?;
    let (adapter, router) = router_with_options_and_trusted_origins(
        SsoOptions::default(),
        vec![oidc.base_url.clone()],
    )?;
    let cookie = seed_session(&adapter).await?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/register",
            &format!(
                r#"{{
                "providerId":"okta",
                "issuer":"{issuer}",
                "domain":"example.com",
                "oidcConfig":{{
                    "clientId":"client_123456",
                    "clientSecret":"super-secret",
                    "discoveryEndpoint":"{issuer}/missing-openid-configuration",
                    "skipDiscovery":false,
                    "pkce":true
                }}
            }}"#,
                issuer = oidc.base_url
            ),
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "discovery_not_found");

    Ok(())
}

#[tokio::test]
async fn register_rejects_untrusted_oidc_discovery_origin() -> Result<(), Box<dyn std::error::Error>>
{
    let oidc = MockOidcServer::start().await?;
    let (adapter, router) = router_with_options(SsoOptions::default())?;
    let cookie = seed_session(&adapter).await?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/register",
            &format!(
                r#"{{
                "providerId":"okta",
                "issuer":"{issuer}",
                "domain":"example.com",
                "oidcConfig":{{
                    "clientId":"client_123456",
                    "clientSecret":"super-secret",
                    "discoveryEndpoint":"{issuer}/.well-known/openid-configuration",
                    "skipDiscovery":false,
                    "pkce":true
                }}
            }}"#,
                issuer = oidc.base_url
            ),
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "discovery_untrusted_origin");

    Ok(())
}
