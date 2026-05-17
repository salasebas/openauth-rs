use std::sync::Arc;

use http::{Method, StatusCode};
use openauth_core::api::{core_auth_async_endpoints, AuthRouter};
use openauth_core::context::create_auth_context_with_adapter;
use openauth_core::db::{DbAdapter, DbValue, FindOne, MemoryAdapter, Where};
use openauth_core::options::{AdvancedOptions, OpenAuthOptions};
use openauth_passkey::{
    passkey, AuthenticatorAttachment, AuthenticatorSelection, PasskeyOptions,
    PasskeyRegistrationOptions, PasskeyRegistrationUser, ResidentKeyRequirement,
    UserVerificationRequirement,
};
use serde_json::{json, Value};

use crate::support::{
    cookie_header_from_response, empty_request, join_cookies, json_request, seeded_router,
    set_cookie_values, sign_in_cookie,
};

#[tokio::test]
async fn generate_register_options_requires_session_by_default(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["code"], "SESSION_REQUIRED");
    Ok(())
}

#[tokio::test]
async fn generate_register_options_uses_resolve_user_without_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().registration(
        PasskeyRegistrationOptions::new()
            .require_session(false)
            .resolve_user(|input| {
                Some(
                    PasskeyRegistrationUser::new(
                        format!("user-{}", input.context.as_deref().unwrap_or("missing")),
                        input
                            .context
                            .unwrap_or_else(|| "missing@example.com".to_owned()),
                    )
                    .display_name("Pre-auth User"),
                )
            }),
    );
    let (_adapter, router, backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options?context=preauth@example.com",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["challenge"], "registration-challenge");
    assert_eq!(body["user"]["name"], "preauth@example.com");
    assert_eq!(body["user"]["displayName"], "Pre-auth User");
    assert!(set_cookie_values(&response)
        .iter()
        .any(|cookie| cookie.contains("better-auth-passkey")));
    let users = backend
        .registration_users
        .lock()
        .map_err(|_| "registration user mutex poisoned")?;
    assert_eq!(users.as_slice(), &["user-preauth@example.com"]);
    Ok(())
}

#[tokio::test]
async fn generate_register_options_requires_resolve_user_in_preauth_mode(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default()
        .registration(PasskeyRegistrationOptions::new().require_session(false));
    let (_adapter, router, _backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["code"], "RESOLVE_USER_REQUIRED");
    Ok(())
}

#[tokio::test]
async fn generate_register_options_rejects_invalid_resolved_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().registration(
        PasskeyRegistrationOptions::new()
            .require_session(false)
            .resolve_user(|_| None),
    );
    let (_adapter, router, _backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["code"], "RESOLVED_USER_INVALID");
    Ok(())
}

#[tokio::test]
async fn generate_register_options_uses_query_name_for_webauthn_user_name(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().registration(
        PasskeyRegistrationOptions::new()
            .require_session(false)
            .resolve_user(|_| {
                Some(PasskeyRegistrationUser::new(
                    "preauth-user",
                    "preauth@example.com",
                ))
            }),
    );
    let (_adapter, router, _backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options?name=Work%20Laptop",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["user"]["name"], "Work Laptop");
    assert_eq!(body["user"]["displayName"], "preauth@example.com");
    Ok(())
}

#[tokio::test]
async fn generate_register_options_includes_selection_attachment_and_extensions(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default()
        .authenticator_selection(
            AuthenticatorSelection::new()
                .resident_key(ResidentKeyRequirement::Required)
                .user_verification(UserVerificationRequirement::Discouraged)
                .authenticator_attachment(AuthenticatorAttachment::CrossPlatform),
        )
        .registration(
            PasskeyRegistrationOptions::new()
                .require_session(false)
                .resolve_user(|_| {
                    Some(PasskeyRegistrationUser::new(
                        "preauth-user",
                        "preauth@example.com",
                    ))
                })
                .extensions(json!({ "credProps": true, "hmacCreateSecret": true })),
        );
    let (_adapter, router, _backend) = seeded_router(options).await?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options?authenticatorAttachment=platform",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(
        body["authenticatorSelection"]["authenticatorAttachment"],
        "platform"
    );
    assert_eq!(body["authenticatorSelection"]["residentKey"], "required");
    assert_eq!(
        body["authenticatorSelection"]["userVerification"],
        "discouraged"
    );
    assert_eq!(body["extensions"]["credProps"], true);
    assert_eq!(body["extensions"]["hmacCreateSecret"], true);
    Ok(())
}

#[tokio::test]
async fn real_webauthn_backend_generates_registration_option_shape(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(MemoryAdapter::new());
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("http://localhost:3000".to_owned()),
            secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
            advanced: AdvancedOptions {
                disable_csrf_check: true,
                disable_origin_check: true,
                ..AdvancedOptions::default()
            },
            plugins: vec![passkey(
                PasskeyOptions::default().registration(
                    PasskeyRegistrationOptions::new()
                        .require_session(false)
                        .resolve_user(|_| {
                            Some(PasskeyRegistrationUser::new(
                                "real-user",
                                "real@example.com",
                            ))
                        }),
                ),
            )],
            ..OpenAuthOptions::default()
        },
        adapter,
    )?;
    let router = AuthRouter::with_async_endpoints(
        context,
        Vec::new(),
        core_auth_async_endpoints(Arc::new(MemoryAdapter::new())),
    )?;

    let response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert!(body["challenge"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(body["rp"]["id"], "localhost");
    assert_eq!(body["user"]["name"], "real@example.com");
    assert_eq!(body["authenticatorSelection"]["residentKey"], "preferred");
    assert_eq!(
        body["authenticatorSelection"]["userVerification"],
        "preferred"
    );
    assert!(body["pubKeyCredParams"]
        .as_array()
        .is_some_and(|values| !values.is_empty()));
    Ok(())
}

#[tokio::test]
async fn verify_registration_creates_passkey_and_deletes_challenge(
) -> Result<(), Box<dyn std::error::Error>> {
    let (adapter, router, _backend) = seeded_router(PasskeyOptions::default()).await?;
    let session_cookie = sign_in_cookie(&router).await?;
    let options_response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            Some(&session_cookie),
        )?)
        .await?;
    let passkey_cookie = cookie_header_from_response(&options_response);
    let cookie = join_cookies(&[session_cookie.as_str(), passkey_cookie.as_str()]);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/verify-registration",
            r#"{"response":{"id":"credential-id"},"name":"Laptop"}"#,
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["name"], "Laptop");
    assert_eq!(body["credentialId"], "credential-id");
    assert_eq!(adapter.len("verification").await, 0);
    Ok(())
}

#[tokio::test]
async fn after_registration_verification_can_override_preauth_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().registration(
        PasskeyRegistrationOptions::new()
            .require_session(false)
            .resolve_user(|_| {
                Some(PasskeyRegistrationUser::new(
                    "preauth-user",
                    "preauth@example.com",
                ))
            })
            .after_verification(|_| Some("user_1".to_owned())),
    );
    let (adapter, router, _backend) = seeded_router(options).await?;
    let options_response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options?context=link-token",
            None,
        )?)
        .await?;
    let passkey_cookie = cookie_header_from_response(&options_response);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/verify-registration",
            r#"{"response":{"id":"override-credential"}}"#,
            Some(&passkey_cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["userId"], "user_1");
    let record = adapter
        .find_one(FindOne::new("passkey").where_clause(Where::new(
            "credential_id",
            DbValue::String("override-credential".to_owned()),
        )))
        .await?
        .ok_or("missing passkey")?;
    assert_eq!(
        record.get("user_id"),
        Some(&DbValue::String("user_1".to_owned()))
    );
    Ok(())
}

#[tokio::test]
async fn after_registration_verification_cannot_override_session_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let options = PasskeyOptions::default().registration(
        PasskeyRegistrationOptions::new().after_verification(|_| Some("user_2".to_owned())),
    );
    let (adapter, router, _backend) = seeded_router(options).await?;
    let session_cookie = sign_in_cookie(&router).await?;
    let options_response = router
        .handle_async(empty_request(
            Method::GET,
            "/api/auth/passkey/generate-register-options",
            Some(&session_cookie),
        )?)
        .await?;
    let passkey_cookie = cookie_header_from_response(&options_response);
    let cookie = join_cookies(&[session_cookie.as_str(), passkey_cookie.as_str()]);

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/passkey/verify-registration",
            r#"{"response":{"id":"mismatch-credential"}}"#,
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(adapter.len("passkey").await, 0);
    Ok(())
}
