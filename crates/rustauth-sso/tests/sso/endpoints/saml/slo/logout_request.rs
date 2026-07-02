use super::*;

#[tokio::test]
async fn saml_slo_logout_request_allows_unsigned_opt_out_and_deletes_matching_saml_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    options.saml.want_logout_request_signed = false;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let relay_state = saml_sign_in_relay_state(&router).await?;
    let saml_response = valid_saml_response(&relay_state, "assertion-idp-slo")?;
    let acs = post_saml_acs(&router, &saml_response, &relay_state).await?;
    assert_eq!(acs.status(), StatusCode::FOUND);
    assert_eq!(adapter.records("session").await.len(), 2);
    let logout_request = logout_request_xml("idp-logout-1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLRequest":{},"RelayState":"/logged-out"}}"#,
                serde_json::to_string(&logout_request)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("missing Location")?;
    let url = url::Url::parse(location)?;
    let query = url
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        query.get("RelayState").map(|value| value.as_ref()),
        Some("/logged-out")
    );
    let saml_response = query.get("SAMLResponse").ok_or("missing SAMLResponse")?;
    let xml = inflate_redirect_binding(saml_response)?;
    assert!(xml.contains("<samlp:LogoutResponse"));
    assert!(xml.contains(r#"InResponseTo="idp-logout-1""#));
    assert!(adapter.records("verification").await.iter().all(|record| {
        !record.get("identifier").is_some_and(|value| {
            matches!(
                value,
                DbValue::String(identifier)
                    if identifier.starts_with("saml-session:saml-okta:")
                        || identifier.starts_with("saml-session-by-id:")
            )
        })
    }));
    assert_eq!(adapter.records("session").await.len(), 1);

    Ok(())
}

#[tokio::test]
async fn saml_slo_logout_request_requires_signature_by_default(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let logout_request = logout_request_xml("unsigned-logout")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLRequest":{},"RelayState":"/logged-out"}}"#,
                serde_json::to_string(&logout_request)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(response)?["code"],
        "SAML_LOGOUT_REQUEST_SIGNATURE_REQUIRED"
    );

    Ok(())
}

#[tokio::test]
async fn saml_slo_redirect_logout_request_requires_signature_by_default(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let logout_request = logout_request_xml("unsigned-redirect-logout")?;
    let logout_request_xml = String::from_utf8(
        base64::engine::general_purpose::STANDARD.decode(logout_request.as_bytes())?,
    )?;
    let saml_request = deflate_redirect_binding(&logout_request_xml)?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("SAMLRequest", &saml_request)
        .append_pair("RelayState", "/logged-out")
        .finish();

    let response = router
        .handle_async(json_request(
            Method::GET,
            &format!("/sso/saml2/sp/slo/saml-okta?{query}"),
            "",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(response)?["code"],
        "SAML_LOGOUT_REQUEST_SIGNATURE_REQUIRED"
    );

    Ok(())
}

#[tokio::test]
async fn saml_slo_redirect_logout_request_rejects_inflated_message_over_configured_size_limit(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    options.saml.max_logout_message_size = 512;
    let max_logout_message_size = options.saml.max_logout_message_size;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let filler = "a".repeat(max_logout_message_size * 4);
    let issue_instant =
        time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
    let xml = format!(
        r#"<samlp:LogoutRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="oversized-logout" Version="2.0" IssueInstant="{issue_instant}" Destination="https://app.example.com/sso/saml2/sp/slo/saml-okta"><saml:Issuer>https://idp.example.com</saml:Issuer><saml:NameID>{filler}</saml:NameID></samlp:LogoutRequest>"#
    );
    assert!(xml.len() > max_logout_message_size);
    let saml_request = deflate_redirect_binding(&xml)?;
    assert!(saml_request.len() < max_logout_message_size);
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("SAMLRequest", &saml_request)
        .append_pair("RelayState", "/logged-out")
        .finish();

    let response = router
        .handle_async(json_request(
            Method::GET,
            &format!("/sso/saml2/sp/slo/saml-okta?{query}"),
            "",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        json_body(response)?["code"],
        "SAML_LOGOUT_MESSAGE_TOO_LARGE"
    );

    Ok(())
}

#[tokio::test]
async fn saml_slo_rejects_when_single_logout_is_disabled() -> Result<(), Box<dyn std::error::Error>>
{
    let (adapter, router) = router_with_options(SsoOptions::default())?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let logout_request = logout_request_xml("disabled-logout")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLRequest":{}}}"#,
                serde_json::to_string(&logout_request)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "SINGLE_LOGOUT_NOT_ENABLED");

    Ok(())
}

#[tokio::test]
async fn saml_slo_redirects_missing_logout_data_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            "{}",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("missing Location")?;
    assert!(location.ends_with("?error=missing_logout_data"));

    Ok(())
}

#[tokio::test]
async fn saml_slo_accepts_form_urlencoded_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;

    let response = router
        .handle_async(form_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            "RelayState=%2Fdone",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("missing Location")?;
    assert!(location.ends_with("?error=missing_logout_data"));

    Ok(())
}

#[tokio::test]
async fn saml_slo_rejects_non_success_logout_response() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let logout_response = logout_response_xml("request-that-failed")?;
    let logout_response = tamper_base64_xml(
        &logout_response,
        "urn:oasis:names:tc:SAML:2.0:status:Success",
        "urn:oasis:names:tc:SAML:2.0:status:Responder",
    )?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{}}}"#,
                serde_json::to_string(&logout_response)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "LOGOUT_FAILED_AT_IDP");

    Ok(())
}

#[tokio::test]
async fn saml_slo_rejects_unknown_pending_logout_response_without_consuming_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let relay_state = saml_sign_in_relay_state(&router).await?;
    let saml_response = valid_saml_response(&relay_state, "assertion-slo-mismatch")?;
    let acs = post_saml_acs(&router, &saml_response, &relay_state).await?;
    let saml_cookie = set_cookie_header(&acs)?;
    let logout = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/logout/saml-okta",
            r#"{"callbackURL":"/logged-out"}"#,
            Some(&saml_cookie),
        )?)
        .await?;
    let logout_location = logout
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("missing logout Location")?;
    let request_id = logout_request_id_from_location(logout_location)?;
    let logout_response = logout_response_xml("unknown-pending-request")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{}}}"#,
                serde_json::to_string(&logout_response)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "UNKNOWN_LOGOUT_REQUEST");
    assert!(adapter.records("verification").await.iter().any(|record| {
        record.get("identifier")
            == Some(&DbValue::String(format!(
                "saml-logout-request:{request_id}"
            )))
    }));

    Ok(())
}

#[tokio::test]
async fn saml_slo_logout_request_session_index_mismatch_preserves_session_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_single_logout = true;
    options.saml.want_logout_request_signed = false;
    let (adapter, router) = router_with_options(options)?;
    let cookie = seed_session(&adapter).await?;
    register_saml_provider_allowing_unsigned_assertions(&router, &cookie).await?;
    let relay_state = saml_sign_in_relay_state(&router).await?;
    let saml_response = valid_saml_response(&relay_state, "assertion-idp-slo-mismatch")?;
    let acs = post_saml_acs(&router, &saml_response, &relay_state).await?;
    assert_eq!(acs.status(), StatusCode::FOUND);
    assert_eq!(adapter.records("session").await.len(), 2);
    let logout_request = logout_request_xml("idp-logout-mismatch")?;
    let logout_request = tamper_base64_xml(
        &logout_request,
        "session-index-1",
        "different-session-index",
    )?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/slo/saml-okta",
            &format!(
                r#"{{"SAMLRequest":{},"RelayState":"/logged-out"}}"#,
                serde_json::to_string(&logout_request)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(adapter.records("session").await.len(), 2);
    assert!(adapter.records("verification").await.iter().any(|record| {
        record.get("identifier").is_some_and(|value| {
            matches!(
                value,
                DbValue::String(identifier) if identifier.starts_with("saml-session:saml-okta:")
            )
        })
    }));

    Ok(())
}
