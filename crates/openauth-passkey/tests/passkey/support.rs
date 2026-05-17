use std::sync::{Arc, Mutex};

use http::{header, Method, Request, StatusCode};
use openauth_core::api::{core_auth_async_endpoints, AuthRouter};
use openauth_core::context::create_auth_context_with_adapter;
use openauth_core::cookies::parse_set_cookie_header;
use openauth_core::crypto::password::hash_password;
use openauth_core::db::{Create, DbAdapter, DbValue, MemoryAdapter};
use openauth_core::options::{AdvancedOptions, OpenAuthOptions};
use openauth_passkey::{
    passkey, PasskeyAuthenticationStart, PasskeyOptions, PasskeyRegistrationStart,
    PasskeyRegistrationUser, PasskeyWebAuthnBackend, RegistrationWebAuthnOptions,
    VerifiedAuthentication, VerifiedPasskeyCredential, WebAuthnConfig,
};
use serde_json::{json, Value};
use time::OffsetDateTime;

pub async fn seeded_router(
    options: PasskeyOptions,
) -> Result<(Arc<MemoryAdapter>, AuthRouter, Arc<FakeWebAuthnBackend>), Box<dyn std::error::Error>>
{
    let adapter = Arc::new(MemoryAdapter::new());
    seed_user(adapter.as_ref()).await?;
    let backend = Arc::new(FakeWebAuthnBackend::default());
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("http://localhost:3000".to_owned()),
            secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
            advanced: AdvancedOptions {
                disable_csrf_check: true,
                disable_origin_check: true,
                ..AdvancedOptions::default()
            },
            plugins: vec![passkey(options.backend(backend.clone()))],
            ..OpenAuthOptions::default()
        },
        adapter.clone(),
    )?;
    let router = AuthRouter::with_async_endpoints(
        context,
        Vec::new(),
        core_auth_async_endpoints(adapter.clone()),
    )?;
    Ok((adapter, router, backend))
}

pub fn empty_request(
    method: Method,
    path: &str,
    cookie: Option<&str>,
) -> Result<Request<Vec<u8>>, http::Error> {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("http://localhost:3000{path}"));
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(Vec::new())
}

pub fn json_request(
    method: Method,
    path: &str,
    body: &str,
    cookie: Option<&str>,
) -> Result<Request<Vec<u8>>, http::Error> {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("http://localhost:3000{path}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(body.as_bytes().to_vec())
}

pub fn set_cookie_values(response: &http::Response<Vec<u8>>) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect()
}

pub fn cookie_header_from_response(response: &http::Response<Vec<u8>>) -> String {
    set_cookie_values(response)
        .iter()
        .filter_map(|value| {
            parse_set_cookie_header(value)
                .into_iter()
                .next()
                .map(|(name, cookie)| format!("{name}={}", cookie.value))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn join_cookies(values: &[&str]) -> String {
    values
        .iter()
        .filter(|value| !value.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("; ")
}

pub async fn sign_in_cookie(router: &AuthRouter) -> Result<String, Box<dyn std::error::Error>> {
    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-in/email",
            r#"{"email":"ada@example.com","password":"password123"}"#,
            None,
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(cookie_header_from_response(&response))
}

pub async fn session_cookie_for(
    adapter: &MemoryAdapter,
    user_id: &str,
    token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let now = OffsetDateTime::now_utc();
    adapter
        .create(
            Create::new("session")
                .data("id", DbValue::String(format!("session-{token}")))
                .data("user_id", DbValue::String(user_id.to_owned()))
                .data("token", DbValue::String(token.to_owned()))
                .data(
                    "expires_at",
                    DbValue::Timestamp(now + time::Duration::hours(1)),
                )
                .data("ip_address", DbValue::Null)
                .data("user_agent", DbValue::Null)
                .data("created_at", DbValue::Timestamp(now))
                .data("updated_at", DbValue::Timestamp(now))
                .force_allow_id(),
        )
        .await?;
    let context = openauth_core::context::create_auth_context(OpenAuthOptions {
        secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
        ..OpenAuthOptions::default()
    })?;
    let cookies = openauth_core::cookies::set_session_cookie(
        &context.auth_cookies,
        &context.secret,
        token,
        openauth_core::cookies::SessionCookieOptions::default(),
    )?;
    Ok(cookies
        .into_iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; "))
}

async fn seed_user(adapter: &MemoryAdapter) -> Result<(), Box<dyn std::error::Error>> {
    let now = OffsetDateTime::now_utc();
    adapter
        .create(
            Create::new("user")
                .data("id", DbValue::String("user_1".to_owned()))
                .data("name", DbValue::String("Ada".to_owned()))
                .data("email", DbValue::String("ada@example.com".to_owned()))
                .data("email_verified", DbValue::Boolean(true))
                .data("image", DbValue::Null)
                .data("created_at", DbValue::Timestamp(now))
                .data("updated_at", DbValue::Timestamp(now))
                .force_allow_id(),
        )
        .await?;
    adapter
        .create(
            Create::new("account")
                .data("id", DbValue::String("account_1".to_owned()))
                .data("provider_id", DbValue::String("credential".to_owned()))
                .data("account_id", DbValue::String("user_1".to_owned()))
                .data("user_id", DbValue::String("user_1".to_owned()))
                .data("access_token", DbValue::Null)
                .data("refresh_token", DbValue::Null)
                .data("id_token", DbValue::Null)
                .data("access_token_expires_at", DbValue::Null)
                .data("refresh_token_expires_at", DbValue::Null)
                .data("scope", DbValue::Null)
                .data("password", DbValue::String(hash_password("password123")?))
                .data("created_at", DbValue::Timestamp(now))
                .data("updated_at", DbValue::Timestamp(now))
                .force_allow_id(),
        )
        .await?;
    Ok(())
}

pub async fn seed_user_two(adapter: &MemoryAdapter) -> Result<(), Box<dyn std::error::Error>> {
    let now = OffsetDateTime::now_utc();
    adapter
        .create(
            Create::new("user")
                .data("id", DbValue::String("user_2".to_owned()))
                .data("name", DbValue::String("Grace".to_owned()))
                .data("email", DbValue::String("grace@example.com".to_owned()))
                .data("email_verified", DbValue::Boolean(true))
                .data("image", DbValue::Null)
                .data("created_at", DbValue::Timestamp(now))
                .data("updated_at", DbValue::Timestamp(now))
                .force_allow_id(),
        )
        .await?;
    Ok(())
}

pub async fn seed_passkey(
    adapter: &MemoryAdapter,
    id: &str,
    user_id: &str,
    name: &str,
    credential_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    adapter
        .create(
            Create::new("passkey")
                .data("id", DbValue::String(id.to_owned()))
                .data("name", DbValue::String(name.to_owned()))
                .data("public_key", DbValue::String("public-key".to_owned()))
                .data("user_id", DbValue::String(user_id.to_owned()))
                .data("credential_id", DbValue::String(credential_id.to_owned()))
                .data("counter", DbValue::Number(0))
                .data("device_type", DbValue::String("singleDevice".to_owned()))
                .data("backed_up", DbValue::Boolean(false))
                .data("transports", DbValue::String("internal".to_owned()))
                .data("created_at", DbValue::Timestamp(OffsetDateTime::now_utc()))
                .data("aaguid", DbValue::String("aaguid".to_owned()))
                .data(
                    "webauthn_credential",
                    DbValue::Json(json!({ "id": credential_id })),
                )
                .force_allow_id(),
        )
        .await?;
    Ok(())
}

#[derive(Default)]
pub struct FakeWebAuthnBackend {
    pub registration_users: Mutex<Vec<String>>,
}

impl PasskeyWebAuthnBackend for FakeWebAuthnBackend {
    fn start_registration(
        &self,
        config: WebAuthnConfig,
        user: &PasskeyRegistrationUser,
        _exclude_credentials: Vec<Value>,
        request_options: RegistrationWebAuthnOptions,
    ) -> Result<PasskeyRegistrationStart, openauth_core::error::OpenAuthError> {
        self.registration_users
            .lock()
            .map_err(|_| openauth_core::error::OpenAuthError::Adapter("mutex poisoned".to_owned()))?
            .push(user.id.clone());
        let mut options = json!({
            "challenge": "registration-challenge",
            "rp": { "id": config.rp_id, "name": config.rp_name },
            "user": {
                "id": user.id,
                "name": user.name,
                "displayName": user.display_name.as_deref().unwrap_or(&user.name),
            },
            "pubKeyCredParams": [],
            "authenticatorSelection": request_options.authenticator_selection.to_json(),
        });
        if let Some(extensions) = request_options.extensions {
            options["extensions"] = extensions;
        }
        Ok(PasskeyRegistrationStart {
            options,
            state: json!({ "kind": "registration-state" }),
        })
    }

    fn finish_registration(
        &self,
        _config: WebAuthnConfig,
        response: Value,
        _state: Value,
    ) -> Result<VerifiedPasskeyCredential, openauth_core::error::OpenAuthError> {
        let credential_id = response
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("credential-id")
            .to_owned();
        Ok(VerifiedPasskeyCredential {
            credential_id: credential_id.clone(),
            public_key: "public-key".to_owned(),
            counter: 0,
            device_type: "singleDevice".to_owned(),
            backed_up: false,
            transports: Some("internal".to_owned()),
            aaguid: Some("test-aaguid".to_owned()),
            credential: json!({ "id": credential_id }),
        })
    }

    fn start_authentication(
        &self,
        config: WebAuthnConfig,
        credentials: Vec<Value>,
        extensions: Option<Value>,
    ) -> Result<PasskeyAuthenticationStart, openauth_core::error::OpenAuthError> {
        let mut options = json!({
            "challenge": "authentication-challenge",
            "rpId": config.rp_id,
            "userVerification": "preferred",
        });
        if !credentials.is_empty() {
            options["allowCredentials"] = json!(credentials);
        }
        if let Some(extensions) = extensions {
            options["extensions"] = extensions;
        }
        Ok(PasskeyAuthenticationStart {
            options,
            state: json!({ "kind": "authentication-state" }),
        })
    }

    fn finish_authentication(
        &self,
        _config: WebAuthnConfig,
        _response: Value,
        _state: Value,
        _credential: Option<Value>,
    ) -> Result<VerifiedAuthentication, openauth_core::error::OpenAuthError> {
        Ok(VerifiedAuthentication {
            credential: None,
            new_counter: 1,
        })
    }
}
