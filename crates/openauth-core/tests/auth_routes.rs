use std::collections::HashMap;
use std::sync::Arc;

use http::{header, Method, Request, StatusCode};
use openauth_core::api::{core_auth_async_endpoints, AuthRouter};
use openauth_core::context::create_auth_context;
use openauth_core::cookies::Cookie;
use openauth_core::crypto::password::hash_password;
use openauth_core::db::{
    run_transaction_without_native_support, AdapterFuture, Count, Create, DbAdapter, DbRecord,
    DbValue, Delete, DeleteMany, FindMany, FindOne, Session, TransactionCallback, Update,
    UpdateMany, User, Where, WhereOperator,
};
use openauth_core::error::OpenAuthError;
use openauth_core::options::{AdvancedOptions, OpenAuthOptions};
use serde_json::Value;
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;

#[derive(Default)]
struct RouteAdapter {
    users: Mutex<HashMap<String, DbRecord>>,
    accounts: Mutex<HashMap<String, DbRecord>>,
    sessions: Mutex<HashMap<String, DbRecord>>,
}

impl RouteAdapter {
    async fn insert_user(&self, user: User) {
        self.users
            .lock()
            .await
            .insert(user.email.clone(), user_record(user));
    }

    async fn insert_account(&self, record: DbRecord) -> Result<(), OpenAuthError> {
        let id = string_field(&record, "id")?.to_owned();
        self.accounts.lock().await.insert(id, record);
        Ok(())
    }

    async fn insert_session(&self, session: Session) {
        self.sessions
            .lock()
            .await
            .insert(session.token.clone(), session_record(session));
    }
}

impl DbAdapter for RouteAdapter {
    fn id(&self) -> &str {
        "route-memory"
    }

    fn create<'a>(&'a self, query: Create) -> AdapterFuture<'a, DbRecord> {
        Box::pin(async move {
            match query.model.as_str() {
                "user" => {
                    let email = string_field(&query.data, "email")?.to_owned();
                    self.users.lock().await.insert(email, query.data.clone());
                    Ok(query.data)
                }
                "account" => {
                    let id = string_field(&query.data, "id")?.to_owned();
                    self.accounts.lock().await.insert(id, query.data.clone());
                    Ok(query.data)
                }
                "session" => {
                    let token = string_field(&query.data, "token")?.to_owned();
                    self.sessions.lock().await.insert(token, query.data.clone());
                    Ok(query.data)
                }
                model => Err(OpenAuthError::Adapter(format!(
                    "unexpected create model `{model}`"
                ))),
            }
        })
    }

    fn find_one<'a>(&'a self, query: FindOne) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move {
            match query.model.as_str() {
                "user" => {
                    if let Ok(email) = string_filter(&query.where_clauses, "email") {
                        return Ok(self.users.lock().await.get(email).cloned());
                    }
                    let id = string_filter(&query.where_clauses, "id")?;
                    Ok(self
                        .users
                        .lock()
                        .await
                        .values()
                        .find(|record| matches!(record.get("id"), Some(DbValue::String(value)) if value == id))
                        .cloned())
                }
                "account" => {
                    let user_id = string_filter(&query.where_clauses, "user_id")?;
                    let provider_id = string_filter(&query.where_clauses, "provider_id")?;
                    Ok(self
                        .accounts
                        .lock()
                        .await
                        .values()
                        .find(|record| {
                            matches!(record.get("user_id"), Some(DbValue::String(value)) if value == user_id)
                                && matches!(record.get("provider_id"), Some(DbValue::String(value)) if value == provider_id)
                        })
                        .cloned())
                }
                "session" => {
                    let token = string_filter(&query.where_clauses, "token")?;
                    Ok(self.sessions.lock().await.get(token).cloned())
                }
                model => Err(OpenAuthError::Adapter(format!(
                    "unexpected find_one model `{model}`"
                ))),
            }
        })
    }

    fn find_many<'a>(&'a self, query: FindMany) -> AdapterFuture<'a, Vec<DbRecord>> {
        Box::pin(async move {
            match query.model.as_str() {
                "account" => {
                    let user_id = string_filter(&query.where_clauses, "user_id")?;
                    Ok(self
                        .accounts
                        .lock()
                        .await
                        .values()
                        .filter(|record| {
                            matches!(record.get("user_id"), Some(DbValue::String(value)) if value == user_id)
                        })
                        .cloned()
                        .collect())
                }
                "session" => {
                    let user_id = string_filter(&query.where_clauses, "user_id")?;
                    Ok(self
                        .sessions
                        .lock()
                        .await
                        .values()
                        .filter(|record| {
                            matches!(record.get("user_id"), Some(DbValue::String(value)) if value == user_id)
                        })
                        .cloned()
                        .collect())
                }
                _ => Ok(Vec::new()),
            }
        })
    }

    fn count<'a>(&'a self, _query: Count) -> AdapterFuture<'a, u64> {
        Box::pin(async { Ok(0) })
    }

    fn update<'a>(&'a self, query: Update) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move {
            let token = string_filter(&query.where_clauses, "token")?;
            let mut sessions = self.sessions.lock().await;
            let Some(record) = sessions.get_mut(token) else {
                return Ok(None);
            };
            for (key, value) in query.data {
                record.insert(key, value);
            }
            Ok(Some(record.clone()))
        })
    }

    fn update_many<'a>(&'a self, _query: UpdateMany) -> AdapterFuture<'a, u64> {
        Box::pin(async { Ok(0) })
    }

    fn delete<'a>(&'a self, query: Delete) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            let token = string_filter(&query.where_clauses, "token")?;
            self.sessions.lock().await.remove(token);
            Ok(())
        })
    }

    fn delete_many<'a>(&'a self, query: DeleteMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move {
            match query.model.as_str() {
                "session" => {
                    let user_id = string_filter(&query.where_clauses, "user_id")?;
                    let mut sessions = self.sessions.lock().await;
                    let before = sessions.len();
                    sessions.retain(|_, record| {
                        !matches!(record.get("user_id"), Some(DbValue::String(value)) if value == user_id)
                    });
                    Ok((before - sessions.len()) as u64)
                }
                _ => Ok(0),
            }
        })
    }

    fn transaction<'a>(&'a self, callback: TransactionCallback<'a>) -> AdapterFuture<'a, ()> {
        run_transaction_without_native_support(self, callback)
    }
}

#[tokio::test]
async fn sign_up_email_route_creates_session_and_sets_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let router = router(adapter.clone())?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-up/email",
            r#"{"name":"Ada","email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert!(body["session"].is_null());
    assert!(body["token"]
        .as_str()
        .is_some_and(|token| !token.is_empty()));
    assert_eq!(body["user"]["email"], "ada@example.com");
    assert_eq!(adapter.users.lock().await.len(), 1);
    assert_eq!(adapter.sessions.lock().await.len(), 1);
    assert!(set_cookie_values(&response)
        .iter()
        .any(|cookie| cookie.starts_with("better-auth.session_token=")));
    Ok(())
}

#[tokio::test]
async fn sign_in_email_route_rejects_invalid_credentials() -> Result<(), Box<dyn std::error::Error>>
{
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_account(credential_account_record(
            "user_1",
            &hash_password("other-password")?,
            now,
        ))
        .await?;
    let router = router(adapter.clone())?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-in/email",
            r#"{"email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["code"], "INVALID_EMAIL_OR_PASSWORD");
    assert!(adapter.sessions.lock().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn sign_in_email_route_returns_token_user_and_sets_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_account(credential_account_record(
            "user_1",
            &hash_password("secret123")?,
            now,
        ))
        .await?;
    let router = router(adapter.clone())?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-in/email",
            r#"{"email":"ada@example.com","password":"secret123"}"#,
            None,
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert!(body["session"].is_null());
    assert!(body["token"]
        .as_str()
        .is_some_and(|token| !token.is_empty()));
    assert_eq!(body["user"]["id"], "user_1");
    assert_eq!(adapter.sessions.lock().await.len(), 1);
    assert!(set_cookie_values(&response)
        .iter()
        .any(|cookie| cookie.starts_with("better-auth.session_token=")));
    Ok(())
}

#[tokio::test]
async fn get_session_route_returns_session_from_signed_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::GET,
            "/api/auth/get-session",
            "",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["session"]["token"], "token_1");
    assert_eq!(body["user"]["id"], "user_1");
    Ok(())
}

#[tokio::test]
async fn sign_out_route_deletes_session_and_expires_cookie(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/sign-out",
            "{}",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(adapter.sessions.lock().await.is_empty());
    assert!(set_cookie_values(&response)
        .iter()
        .any(|cookie| cookie.starts_with("better-auth.session_token=;")
            && cookie.contains("Max-Age=0")));
    Ok(())
}

#[tokio::test]
async fn list_sessions_route_returns_active_user_sessions() -> Result<(), Box<dyn std::error::Error>>
{
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    adapter
        .insert_session(Session {
            id: "session_2".to_owned(),
            token: "token_2".to_owned(),
            ..session(now, now + Duration::hours(2))
        })
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::GET,
            "/api/auth/list-sessions",
            "",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body.as_array().map(Vec::len), Some(2));
    assert_eq!(body[0]["user_id"], "user_1");
    Ok(())
}

#[tokio::test]
async fn revoke_session_route_deletes_session_for_current_user(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    adapter
        .insert_session(Session {
            id: "session_2".to_owned(),
            token: "token_2".to_owned(),
            ..session(now, now + Duration::hours(2))
        })
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/revoke-session",
            r#"{"token":"token_2"}"#,
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(response.body())?;
    assert_eq!(body["status"], true);
    assert!(!adapter.sessions.lock().await.contains_key("token_2"));
    Ok(())
}

#[tokio::test]
async fn revoke_sessions_route_deletes_all_current_user_sessions(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    adapter
        .insert_session(Session {
            id: "session_2".to_owned(),
            token: "token_2".to_owned(),
            ..session(now, now + Duration::hours(2))
        })
        .await;
    adapter
        .insert_session(Session {
            id: "session_3".to_owned(),
            user_id: "user_2".to_owned(),
            token: "token_3".to_owned(),
            ..session(now, now + Duration::hours(2))
        })
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/revoke-sessions",
            "{}",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let sessions = adapter.sessions.lock().await;
    assert!(!sessions.contains_key("token_1"));
    assert!(!sessions.contains_key("token_2"));
    assert!(sessions.contains_key("token_3"));
    Ok(())
}

#[tokio::test]
async fn revoke_other_sessions_route_keeps_current_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let now = OffsetDateTime::now_utc();
    adapter.insert_user(user(now)).await;
    adapter
        .insert_session(session(now, now + Duration::hours(1)))
        .await;
    adapter
        .insert_session(Session {
            id: "session_2".to_owned(),
            token: "token_2".to_owned(),
            ..session(now, now + Duration::hours(2))
        })
        .await;
    let router = router(adapter.clone())?;
    let cookie = signed_session_cookie("token_1")?;

    let response = router
        .handle_async(json_request(
            Method::POST,
            "/api/auth/revoke-other-sessions",
            "{}",
            Some(&cookie),
        )?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let sessions = adapter.sessions.lock().await;
    assert!(sessions.contains_key("token_1"));
    assert!(!sessions.contains_key("token_2"));
    Ok(())
}

#[test]
fn core_auth_routes_expose_upstream_openapi_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = Arc::new(RouteAdapter::default());
    let router = router(adapter)?;
    let openapi = router.openapi_schema();

    assert_eq!(
        openapi["paths"]["/sign-up/email"]["post"]["responses"]["200"]["description"],
        "Successfully created user"
    );
    assert_eq!(
        openapi["paths"]["/sign-in/email"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["required"],
        serde_json::json!(["redirect", "token", "user"])
    );
    assert_eq!(
        openapi["paths"]["/get-session"]["get"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["type"],
        serde_json::json!(["object", "null"])
    );
    assert_eq!(
        openapi["paths"]["/get-session"]["post"]["requestBody"]["content"]["application/json"]
            ["schema"],
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    );
    assert_eq!(
        openapi["paths"]["/sign-out"]["post"]["responses"]["200"]["content"]["application/json"]
            ["schema"]["properties"]["success"]["type"],
        "boolean"
    );
    assert_eq!(
        openapi["paths"]["/list-sessions"]["get"]["operationId"],
        "listUserSessions"
    );
    assert_eq!(
        openapi["paths"]["/revoke-session"]["post"]["requestBody"]["content"]["application/json"]
            ["schema"]["required"],
        serde_json::json!(["token"])
    );
    Ok(())
}

fn router(adapter: Arc<RouteAdapter>) -> Result<AuthRouter, OpenAuthError> {
    let context = create_auth_context(OpenAuthOptions {
        secret: Some(secret().to_owned()),
        advanced: AdvancedOptions {
            disable_csrf_check: true,
            ..AdvancedOptions::default()
        },
        ..OpenAuthOptions::default()
    })?;
    AuthRouter::with_async_endpoints(context, Vec::new(), core_auth_async_endpoints(adapter))
}

fn json_request(
    method: Method,
    path: &str,
    body: &str,
    cookie: Option<&str>,
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
    builder.body(body.as_bytes().to_vec())
}

fn secret() -> &'static str {
    "test-secret-123456789012345678901234"
}

fn signed_session_cookie(token: &str) -> Result<String, OpenAuthError> {
    let context = create_auth_context(OpenAuthOptions {
        secret: Some(secret().to_owned()),
        ..OpenAuthOptions::default()
    })?;
    let cookies = openauth_core::cookies::set_session_cookie(
        &context.auth_cookies,
        &context.secret,
        token,
        openauth_core::cookies::SessionCookieOptions::default(),
    )?;
    Ok(cookie_header(&cookies))
}

fn cookie_header(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn set_cookie_values(response: &http::Response<Vec<u8>>) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect()
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

fn credential_account_record(user_id: &str, password_hash: &str, now: OffsetDateTime) -> DbRecord {
    let mut record = DbRecord::new();
    record.insert("id".to_owned(), DbValue::String("account_1".to_owned()));
    record.insert(
        "provider_id".to_owned(),
        DbValue::String("credential".to_owned()),
    );
    record.insert("account_id".to_owned(), DbValue::String(user_id.to_owned()));
    record.insert("user_id".to_owned(), DbValue::String(user_id.to_owned()));
    record.insert("access_token".to_owned(), DbValue::Null);
    record.insert("refresh_token".to_owned(), DbValue::Null);
    record.insert("id_token".to_owned(), DbValue::Null);
    record.insert("access_token_expires_at".to_owned(), DbValue::Null);
    record.insert("refresh_token_expires_at".to_owned(), DbValue::Null);
    record.insert("scope".to_owned(), DbValue::Null);
    record.insert(
        "password".to_owned(),
        DbValue::String(password_hash.to_owned()),
    );
    record.insert("created_at".to_owned(), DbValue::Timestamp(now));
    record.insert("updated_at".to_owned(), DbValue::Timestamp(now));
    record
}

fn string_filter<'a>(where_clauses: &'a [Where], field: &str) -> Result<&'a str, OpenAuthError> {
    where_clauses
        .iter()
        .find_map(|where_clause| {
            match (
                where_clause.field.as_str(),
                where_clause.operator,
                &where_clause.value,
            ) {
                (candidate, WhereOperator::Eq, DbValue::String(value)) if candidate == field => {
                    Some(value.as_str())
                }
                _ => None,
            }
        })
        .ok_or_else(|| OpenAuthError::Adapter(format!("missing {field} filter")))
}

fn string_field<'a>(record: &'a DbRecord, field: &str) -> Result<&'a str, OpenAuthError> {
    match record.get(field) {
        Some(DbValue::String(value)) => Ok(value),
        _ => Err(OpenAuthError::Adapter(format!(
            "missing string field `{field}`"
        ))),
    }
}
