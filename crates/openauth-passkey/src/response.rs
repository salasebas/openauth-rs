use http::{header, HeaderValue, StatusCode};
use openauth_core::api::ApiResponse;
use openauth_core::cookies::Cookie;
use openauth_core::error::OpenAuthError;
use serde::Serialize;
use serde_json::json;

pub fn unauthorized() -> Result<ApiResponse, OpenAuthError> {
    error_response(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Unauthorized")
}

pub fn not_allowed() -> Result<ApiResponse, OpenAuthError> {
    error_response(
        StatusCode::UNAUTHORIZED,
        "YOU_ARE_NOT_ALLOWED_TO_REGISTER_THIS_PASSKEY",
        "You are not allowed to register this passkey",
    )
}

pub fn error_response(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Result<ApiResponse, OpenAuthError> {
    json_response(
        status,
        &json!({
            "code": code.into(),
            "message": message.into(),
            "original_message": null,
        }),
        Vec::new(),
    )
}

pub fn json_response<T>(
    status: StatusCode,
    body: &T,
    cookies: Vec<Cookie>,
) -> Result<ApiResponse, OpenAuthError>
where
    T: Serialize,
{
    let body = serde_json::to_vec(body).map_err(|error| OpenAuthError::Api(error.to_string()))?;
    let mut response = http::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .map_err(|error| OpenAuthError::Api(error.to_string()))?;
    for cookie in cookies {
        response.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_str(&serialize_cookie(&cookie))
                .map_err(|error| OpenAuthError::Cookie(error.to_string()))?,
        );
    }
    Ok(response)
}

fn serialize_cookie(cookie: &Cookie) -> String {
    let mut value = format!("{}={}", cookie.name, cookie.value);
    let attributes = &cookie.attributes;
    if let Some(max_age) = attributes.max_age {
        value.push_str("; Max-Age=");
        value.push_str(&max_age.to_string());
    }
    if let Some(expires) = &attributes.expires {
        value.push_str("; Expires=");
        value.push_str(expires);
    }
    if let Some(domain) = &attributes.domain {
        value.push_str("; Domain=");
        value.push_str(domain);
    }
    if let Some(path) = &cookie.attributes.path {
        value.push_str("; Path=");
        value.push_str(path);
    }
    if cookie.attributes.http_only.unwrap_or(false) {
        value.push_str("; HttpOnly");
    }
    if cookie.attributes.secure.unwrap_or(false) {
        value.push_str("; Secure");
    }
    if let Some(same_site) = &cookie.attributes.same_site {
        value.push_str("; SameSite=");
        value.push_str(same_site);
    }
    if attributes.partitioned.unwrap_or(false) {
        value.push_str("; Partitioned");
    }
    value
}
