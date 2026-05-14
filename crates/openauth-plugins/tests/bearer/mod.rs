use std::sync::Arc;

use http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use openauth_core::api::{core_auth_async_endpoints, AuthRouter};
use openauth_core::context::create_auth_context_with_adapter;
use openauth_core::db::{Create, DbAdapter, DbRecord, DbValue, MemoryAdapter, Session, User};
use openauth_core::error::OpenAuthError;
use openauth_core::options::{AdvancedOptions, OpenAuthOptions};
use openauth_core::plugin::AuthPlugin;
use serde_json::Value;
use time::{Duration, OffsetDateTime};

type TestAdapter = MemoryAdapter;

#[test]
fn exposes_bearer_plugin_metadata() {
    let plugin = openauth_plugins::bearer::bearer();

    assert_eq!(openauth_plugins::bearer::UPSTREAM_PLUGIN_ID, "bearer");
    assert_eq!(plugin.id, "bearer");
    assert_eq!(plugin.version.as_deref(), Some(openauth_plugins::VERSION));
}

#[tokio::test]
async fn sign_up_response_exposes_auth_token_header() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
            HeaderMap::new(),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(auth_token_header(&response).is_some_and(|token| token.contains('.')));
    assert_exposes_auth_token(&response)?;
    Ok(())
}

#[tokio::test]
async fn get_session_accepts_signed_bearer_token() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/get-session",
            &tokens.signed,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["session"]["token"], tokens.raw);
    assert_eq!(body["user"]["email"], "ada@example.com");
    Ok(())
}

#[tokio::test]
async fn list_sessions_accepts_signed_bearer_token() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/list-sessions",
            &tokens.signed,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn bearer_scheme_is_case_insensitive_and_allows_extra_whitespace(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;

    for scheme in ["bearer", "BEARER", "BeArEr", "Bearer "] {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("{scheme}  {}", tokens.signed))?,
        );
        let response = router
            .handle_async(json_request(
                Method::GET,
                "/api/auth/get-session",
                "",
                None,
                headers,
            )?)
            .await?;
        let body: Value = serde_json::from_slice(response.body())?;
        assert_eq!(body["session"]["token"], tokens.raw);
    }
    Ok(())
}

#[tokio::test]
async fn signed_bearer_token_may_be_percent_encoded() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;
    let encoded = percent_encode_component(&tokens.signed);

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/get-session",
            &encoded,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["session"]["token"], tokens.raw);
    Ok(())
}

#[tokio::test]
async fn raw_session_token_is_accepted_when_signature_is_not_required(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    seed_user_and_session(&adapter).await;
    let router = router(adapter, openauth_plugins::bearer::bearer())?;

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/get-session",
            "token_1",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["session"]["token"], "token_1");
    Ok(())
}

#[tokio::test]
async fn raw_session_token_is_rejected_when_signature_is_required(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    seed_user_and_session(&adapter).await;
    let router = router(
        adapter,
        openauth_plugins::bearer::bearer_with_options(openauth_plugins::bearer::BearerOptions {
            require_signature: true,
        }),
    )?;

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/get-session",
            "token_1",
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert!(body.is_null());
    Ok(())
}

#[tokio::test]
async fn invalid_bearer_token_does_not_override_valid_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;
    let cookie = format!("better-auth.session_token={}", tokens.signed);

    let response = router
        .handle_async(bearer_request(
            Method::GET,
            "/api/auth/get-session",
            "invalid.token",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["session"]["token"], tokens.raw);
    Ok(())
}

#[tokio::test]
async fn missing_malformed_and_empty_bearer_headers_are_ignored(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    seed_user_and_session(&adapter).await;
    let router = router(adapter, openauth_plugins::bearer::bearer())?;

    for value in [None, Some("Basic token_1"), Some("Bearer    ")] {
        let mut headers = HeaderMap::new();
        if let Some(value) = value {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
        }
        let response = router
            .handle_async(json_request(
                Method::GET,
                "/api/auth/get-session",
                "",
                None,
                headers,
            )?)
            .await?;
        let body: Value = serde_json::from_slice(response.body())?;
        assert!(body.is_null());
    }
    Ok(())
}

#[tokio::test]
async fn sign_out_expired_cookie_does_not_emit_auth_token_header(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(TestAdapter::default());
    let router = router(adapter, openauth_plugins::bearer::bearer())?;
    let tokens = sign_up_and_tokens(&router).await?;

    let response = router
        .handle_async(bearer_request(
            Method::POST,
            "/api/auth/sign-out",
            &tokens.signed,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(auth_token_header(&response).is_none());
    Ok(())
}

fn router(adapter: Arc<TestAdapter>, plugin: AuthPlugin) -> Result<AuthRouter, OpenAuthError> {
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            secret: Some(secret().to_owned()),
            plugins: vec![plugin],
            advanced: AdvancedOptions {
                disable_csrf_check: true,
                disable_origin_check: true,
                ..AdvancedOptions::default()
            },
            ..OpenAuthOptions::default()
        },
        adapter.clone(),
    )?;
    AuthRouter::with_async_endpoints(context, Vec::new(), core_auth_async_endpoints(adapter))
}

struct SignUpTokens {
    raw: String,
    signed: String,
}

async fn sign_up_and_tokens(
    router: &AuthRouter,
) -> Result<SignUpTokens, Box<dyn std::error::Error>> {
    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
            HeaderMap::new(),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    let raw = body["token"]
        .as_str()
        .ok_or("missing sign-up token")?
        .to_owned();
    let signed = auth_token_header(&response).ok_or("missing set-auth-token header")?;
    Ok(SignUpTokens { raw, signed })
}

fn json_request(
    method: Method,
    path: &str,
    body: &str,
    cookie: Option<&str>,
    headers: HeaderMap,
) -> Result<Request<Vec<u8>>, http::Error> {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("http://localhost:3000{path}"));
    if !body.is_empty() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    for (name, value) in headers {
        if let Some(name) = name {
            builder = builder.header(name, value);
        }
    }
    builder.body(body.as_bytes().to_vec())
}

fn bearer_request(
    method: Method,
    path: &str,
    token: &str,
    cookie: Option<&str>,
) -> Result<Request<Vec<u8>>, http::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))
            .unwrap_or_else(|_| HeaderValue::from_static("Bearer invalid")),
    );
    json_request(method, path, "", cookie, headers)
}

fn auth_token_header(response: &http::Response<Vec<u8>>) -> Option<String> {
    response
        .headers()
        .get("set-auth-token")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn assert_exposes_auth_token(
    response: &http::Response<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let exposed = response
        .headers()
        .get("access-control-expose-headers")
        .ok_or("missing access-control-expose-headers")?
        .to_str()?;
    assert!(exposed
        .split(',')
        .map(str::trim)
        .any(|header| header.eq_ignore_ascii_case("set-auth-token")));
    Ok(())
}

async fn seed_user_and_session(adapter: &TestAdapter) {
    let now = OffsetDateTime::now_utc();
    let _ = adapter
        .create(create_query("user", user_record(user(now))))
        .await;
    let _ = adapter
        .create(create_query(
            "session",
            session_record(session(now, now + Duration::hours(1))),
        ))
        .await;
}

fn create_query(model: &str, record: DbRecord) -> Create {
    record
        .into_iter()
        .fold(Create::new(model), |query, (field, value)| {
            query.data(field, value)
        })
}

fn secret() -> &'static str {
    "test-secret-123456789012345678901234"
}

fn user(now: OffsetDateTime) -> User {
    User {
        id: "user_1".to_owned(),
        name: "Ada".to_owned(),
        email: "ada@example.com".to_owned(),
        email_verified: true,
        image: None,
        created_at: now,
        updated_at: now,
    }
}

fn session(now: OffsetDateTime, expires_at: OffsetDateTime) -> Session {
    Session {
        id: "session_1".to_owned(),
        user_id: "user_1".to_owned(),
        expires_at,
        token: "token_1".to_owned(),
        ip_address: None,
        user_agent: None,
        created_at: now,
        updated_at: now,
    }
}

fn user_record(user: User) -> DbRecord {
    let mut record = DbRecord::new();
    record.insert("id".to_owned(), DbValue::String(user.id));
    record.insert("name".to_owned(), DbValue::String(user.name));
    record.insert("email".to_owned(), DbValue::String(user.email));
    record.insert(
        "email_verified".to_owned(),
        DbValue::Boolean(user.email_verified),
    );
    record.insert(
        "image".to_owned(),
        user.image.map(DbValue::String).unwrap_or(DbValue::Null),
    );
    record.insert("created_at".to_owned(), DbValue::Timestamp(user.created_at));
    record.insert("updated_at".to_owned(), DbValue::Timestamp(user.updated_at));
    record
}

fn session_record(session: Session) -> DbRecord {
    let mut record = DbRecord::new();
    record.insert("id".to_owned(), DbValue::String(session.id));
    record.insert("user_id".to_owned(), DbValue::String(session.user_id));
    record.insert(
        "expires_at".to_owned(),
        DbValue::Timestamp(session.expires_at),
    );
    record.insert("token".to_owned(), DbValue::String(session.token));
    record.insert("ip_address".to_owned(), DbValue::Null);
    record.insert("user_agent".to_owned(), DbValue::Null);
    record.insert(
        "created_at".to_owned(),
        DbValue::Timestamp(session.created_at),
    );
    record.insert(
        "updated_at".to_owned(),
        DbValue::Timestamp(session.updated_at),
    );
    record
}

fn percent_encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
