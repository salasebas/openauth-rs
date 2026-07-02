use std::sync::Arc;

use http::header;
use rustauth_core::api::{ApiRequest, AsyncAuthEndpoint};
use rustauth_core::context::AuthContext;
use rustauth_core::error::RustAuthError;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::options::{PasskeyExtensionsInput, PasskeyOptions};
use crate::webauthn::WebAuthnConfig;

mod authentication;
mod management;
mod registration;

pub fn endpoints(options: Arc<PasskeyOptions>) -> Vec<AsyncAuthEndpoint> {
    vec![
        registration::generate_register_options_endpoint(Arc::clone(&options)),
        authentication::generate_authenticate_options_endpoint(Arc::clone(&options)),
        registration::verify_registration_endpoint(Arc::clone(&options)),
        authentication::verify_authentication_endpoint(Arc::clone(&options)),
        management::list_passkeys_endpoint(Arc::clone(&options)),
        management::delete_passkey_endpoint(Arc::clone(&options)),
        management::update_passkey_endpoint(options),
    ]
}

#[derive(Debug, Deserialize)]
pub(crate) struct VerifyRegistrationBody {
    pub response: Value,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VerifyAuthenticationBody {
    pub response: Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IdBody {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdatePasskeyBody {
    pub id: String,
    pub name: String,
}

const PASSKEY_ORIGIN_REQUIRED: &str =
    "passkey requires an explicit origin, a request Origin header, or a configured base_url";
const PASSKEY_RP_ID_REQUIRED: &str =
    "passkey requires an explicit rp_id or a host derivable from base_url or origin";

#[derive(Debug)]
pub(crate) enum WebAuthnConfigError {
    InvalidOrigin,
    RustAuth(RustAuthError),
}

impl From<RustAuthError> for WebAuthnConfigError {
    fn from(error: RustAuthError) -> Self {
        Self::RustAuth(error)
    }
}

fn resolve_passkey_origins(
    context: &AuthContext,
    options: &PasskeyOptions,
    request: &ApiRequest,
) -> Result<Vec<String>, WebAuthnConfigError> {
    if !options.origin.is_empty() {
        return Ok(options.origin.clone());
    }
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        let origin = origin.trim_end_matches('/').to_owned();
        if context.is_trusted_origin_for_request(&origin, None, Some(request))? {
            return Ok(vec![origin]);
        }
        if context.base_url.is_empty() {
            return Err(WebAuthnConfigError::InvalidOrigin);
        }
    }
    if !context.base_url.is_empty() {
        return Ok(vec![context.base_url.trim_end_matches('/').to_owned()]);
    }
    Err(RustAuthError::InvalidConfig(PASSKEY_ORIGIN_REQUIRED.to_owned()).into())
}

fn resolve_passkey_rp_id(
    context: &AuthContext,
    options: &PasskeyOptions,
    origins: &[String],
) -> Result<String, RustAuthError> {
    if let Some(rp_id) = &options.rp_id {
        return Ok(rp_id.clone());
    }
    if let Some(host) = host_from_url(context.base_url.as_str()) {
        return Ok(host);
    }
    if let Some(host) = origins.first().and_then(|origin| host_from_url(origin)) {
        return Ok(host);
    }
    Err(RustAuthError::InvalidConfig(
        PASSKEY_RP_ID_REQUIRED.to_owned(),
    ))
}

fn passkey_webauthn_config(
    context: &AuthContext,
    options: &PasskeyOptions,
    origins: Vec<String>,
) -> Result<WebAuthnConfig, RustAuthError> {
    let rp_id = resolve_passkey_rp_id(context, options, &origins)?;
    Ok(WebAuthnConfig {
        rp_id,
        rp_name: options
            .rp_name
            .clone()
            .unwrap_or_else(|| context.app_name.clone()),
        origins,
    })
}

pub(crate) fn webauthn_config(
    context: &AuthContext,
    options: &PasskeyOptions,
    request: &ApiRequest,
) -> Result<WebAuthnConfig, WebAuthnConfigError> {
    let origins = resolve_passkey_origins(context, options, request)?;
    passkey_webauthn_config(context, options, origins).map_err(WebAuthnConfigError::from)
}

fn host_from_url(value: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
}

pub(crate) fn query_param(request: &ApiRequest, name: &str) -> Option<String> {
    request.uri().query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
    })
}

pub(crate) async fn resolve_extensions(
    resolver: &Option<crate::options::PasskeyExtensionsResolver>,
    input: PasskeyExtensionsInput,
) -> Option<Value> {
    match resolver {
        Some(resolver) => resolver(input).await,
        None => None,
    }
}
