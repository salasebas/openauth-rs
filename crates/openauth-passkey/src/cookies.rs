use http::header;
use openauth_core::api::ApiRequest;
use openauth_core::context::AuthContext;
use openauth_core::cookies::{
    parse_cookies, sign_cookie_value, verify_cookie_value, Cookie, CookieOptions,
};
use openauth_core::error::OpenAuthError;

use crate::challenge::CHALLENGE_MAX_AGE_SECONDS;
use crate::options::PasskeyOptions;

pub fn challenge_cookie(
    context: &AuthContext,
    options: &PasskeyOptions,
    value: String,
) -> Result<Cookie, OpenAuthError> {
    Ok(Cookie {
        name: options.advanced.webauthn_challenge_cookie.clone(),
        value: sign_cookie_value(&value, &context.secret)?,
        attributes: CookieOptions {
            max_age: Some(CHALLENGE_MAX_AGE_SECONDS),
            path: Some("/".to_owned()),
            secure: context.auth_cookies.session_token.attributes.secure,
            http_only: Some(true),
            same_site: Some("lax".to_owned()),
            ..CookieOptions::default()
        },
    })
}

pub fn challenge_token(
    context: &AuthContext,
    options: &PasskeyOptions,
    request: &ApiRequest,
) -> Result<Option<String>, OpenAuthError> {
    let Some(cookie_header) = request_cookie_header(request) else {
        return Ok(None);
    };
    let Some(value) = parse_cookies(&cookie_header)
        .get(&options.advanced.webauthn_challenge_cookie)
        .cloned()
    else {
        return Ok(None);
    };
    verify_cookie_value(&value, &context.secret)
}

pub fn request_cookie_header(request: &ApiRequest) -> Option<String> {
    request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
