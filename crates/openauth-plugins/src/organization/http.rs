use http::{header, StatusCode};
use openauth_core::api::{parse_request_body, ApiErrorResponse, ApiRequest, ApiResponse};
use openauth_core::auth::session::{GetSessionInput, SessionAuth};
use openauth_core::context::AuthContext;
use openauth_core::db::{DbAdapter, Session, User};
use openauth_core::error::OpenAuthError;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct CurrentSession {
    pub session: Session,
    pub user: User,
    pub active_organization_id: Option<String>,
}

pub fn json<T: Serialize>(status: StatusCode, body: &T) -> Result<ApiResponse, OpenAuthError> {
    let body = serde_json::to_vec(body).map_err(|error| OpenAuthError::Api(error.to_string()))?;
    http::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .map_err(|error| OpenAuthError::Api(error.to_string()))
}

pub fn error(status: StatusCode, code: &str, message: &str) -> Result<ApiResponse, OpenAuthError> {
    json(
        status,
        &ApiErrorResponse {
            code: code.to_owned(),
            message: message.to_owned(),
            original_message: None,
        },
    )
}

pub fn organization_error(status: StatusCode, code: &str) -> Result<ApiResponse, OpenAuthError> {
    error(status, code, super::errors::message(code))
}

pub fn body<T: DeserializeOwned>(request: &ApiRequest) -> Result<T, OpenAuthError> {
    parse_request_body(request)
}

pub fn adapter(context: &AuthContext) -> Result<std::sync::Arc<dyn DbAdapter>, OpenAuthError> {
    context.adapter().ok_or_else(|| {
        OpenAuthError::InvalidConfig("organization plugin requires an adapter".to_owned())
    })
}

pub async fn current_session(
    context: &AuthContext,
    request: &ApiRequest,
    store: &super::store::OrganizationStore<'_>,
) -> Result<Option<CurrentSession>, OpenAuthError> {
    let cookie_header = request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let Some(result) = SessionAuth::new(store.adapter(), context)
        .get_session(GetSessionInput::new(cookie_header))
        .await?
    else {
        return Ok(None);
    };
    let Some(session) = result.session else {
        return Ok(None);
    };
    let Some(user) = result.user else {
        return Ok(None);
    };
    let active_organization_id = store.active_organization_id(&session.token).await?;
    Ok(Some(CurrentSession {
        session,
        user,
        active_organization_id,
    }))
}
