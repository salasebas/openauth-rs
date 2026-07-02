use super::*;

#[tokio::test]
async fn saml_acs_rejects_unsolicited_response_by_default() -> Result<(), Box<dyn std::error::Error>>
{
    let (_adapter, router) = router_with_options(SsoOptions::default())?;
    seed_saml_provider_record(&_adapter).await?;
    let saml_response = idp_initiated_saml_response("saml-okta", "assertion-unsolicited-reject")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/acs/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{}}}"#,
                serde_json::to_string(&saml_response)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "MISSING_RELAY_STATE");

    Ok(())
}

#[tokio::test]
async fn saml_acs_accepts_unsolicited_response_when_allow_idp_initiated_is_true(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.allow_idp_initiated = true;
    let (adapter, router) = router_with_options(options)?;
    seed_saml_provider_record(&adapter).await?;
    let saml_response = idp_initiated_saml_response("saml-okta", "assertion-unsolicited-accept")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/acs/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{}}}"#,
                serde_json::to_string(&saml_response)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(adapter.records("user").await.iter().any(|record| {
        record.get("email") == Some(&DbValue::String("saml-user@example.com".to_owned()))
    }));
    assert!(adapter.records("account").await.iter().any(|record| {
        record.get("provider_id") == Some(&DbValue::String("saml-okta".to_owned()))
    }));

    Ok(())
}

#[tokio::test]
async fn saml_acs_rejects_unsolicited_response_missing_sp_binding(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.allow_idp_initiated = true;
    let (adapter, router) = router_with_options(options)?;
    seed_saml_provider_record(&adapter).await?;
    let acs_url = "https://app.example.com/sso/saml2/sp/acs/saml-okta";
    let cases = [
        (
            "assertion-unsolicited-missing-destination",
            format!(r#" Destination="{acs_url}""#),
            String::new(),
            "SAML_DESTINATION_REQUIRED",
        ),
        (
            "assertion-unsolicited-missing-audience",
            "<saml:Audience>https://app.example.com/saml/sp</saml:Audience>".to_owned(),
            "<saml:Audience></saml:Audience>".to_owned(),
            "SAML_AUDIENCE_REQUIRED",
        ),
        (
            "assertion-unsolicited-missing-recipient",
            format!(r#" Recipient="{acs_url}""#),
            String::new(),
            "SAML_RECIPIENT_REQUIRED",
        ),
    ];

    for (assertion_id, needle, replacement, expected_code) in cases {
        let saml_response = idp_initiated_saml_response("saml-okta", assertion_id)?;
        let saml_response = tamper_base64_xml(&saml_response, &needle, &replacement)?;

        let response = router
            .handle_async(json_request(
                Method::POST,
                "/sso/saml2/sp/acs/saml-okta",
                &format!(
                    r#"{{"SAMLResponse":{}}}"#,
                    serde_json::to_string(&saml_response)?
                ),
                None,
            )?)
            .await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response)?["code"], expected_code);
    }
    assert!(adapter.records("user").await.is_empty());
    assert!(adapter.records("account").await.is_empty());

    Ok(())
}

#[tokio::test]
async fn saml_acs_rejects_idp_initiated_signup_when_implicit_sign_up_is_disabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions {
        disable_implicit_sign_up: true,
        ..SsoOptions::default()
    };
    options.saml.allow_idp_initiated = true;
    let (adapter, router) = router_with_options(options)?;
    seed_saml_provider_record(&adapter).await?;
    let saml_response =
        idp_initiated_saml_response("saml-okta", "assertion-idp-init-disabled-signup")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/acs/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{}}}"#,
                serde_json::to_string(&saml_response)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(json_body(response)?["code"], "SAML_SIGN_IN_FAILED");
    assert!(adapter.records("user").await.is_empty());

    Ok(())
}

#[tokio::test]
async fn saml_acs_skips_in_response_to_validation_when_disabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = SsoOptions::default();
    options.saml.enable_in_response_to_validation = false;
    let (adapter, router) = router_with_options(options)?;
    seed_saml_provider_record(&adapter).await?;
    let relay_state = saml_sign_in_relay_state(&router).await?;
    let mut saml_response = valid_saml_response(&relay_state, "assertion-in-response-to-disabled")?;
    saml_response = tamper_base64_xml(
        &saml_response,
        &format!(r#"InResponseTo="{relay_state}""#),
        r#"InResponseTo="stale-authn-request-id""#,
    )?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/sso/saml2/sp/acs/saml-okta",
            &format!(
                r#"{{"SAMLResponse":{},"RelayState":{}}}"#,
                serde_json::to_string(&saml_response)?,
                serde_json::to_string(&relay_state)?
            ),
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(adapter.records("account").await.iter().any(|record| {
        record.get("account_id") == Some(&DbValue::String("saml-subject-123".to_owned()))
    }));

    Ok(())
}
