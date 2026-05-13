use josekit::jwk::JwkSet;
use serde_json::Value;

use super::error::OAuthError;
use super::validate_authorization_code::{
    audience_matches, validate_temporal_claims, verify_jws_with_jwks, TokenValidationOptions,
    TokenValidationResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyAccessTokenRemote {
    pub introspect_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub force: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifyAccessTokenOptions {
    pub verify_options: TokenValidationOptions,
    pub scopes: Vec<String>,
    pub jwks_url: Option<String>,
    pub remote_verify: Option<VerifyAccessTokenRemote>,
}

pub async fn get_jwks(jwks_url: &str) -> Result<JwkSet, OAuthError> {
    let bytes = reqwest::Client::new()
        .get(jwks_url)
        .header("accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    JwkSet::from_bytes(bytes).map_err(Into::into)
}

pub async fn verify_jws_access_token(
    token: &str,
    jwks_url: &str,
    verify_options: TokenValidationOptions,
) -> Result<TokenValidationResult, OAuthError> {
    let jwks = get_jwks(jwks_url).await?;
    let mut result = verify_jws_with_jwks(token, &jwks, &verify_options)?;
    map_azp_to_client_id(&mut result.payload);
    Ok(result)
}

pub async fn verify_access_token(
    token: &str,
    options: VerifyAccessTokenOptions,
) -> Result<Value, OAuthError> {
    let mut payload = None;
    if let Some(jwks_url) = &options.jwks_url {
        if !options
            .remote_verify
            .as_ref()
            .is_some_and(|remote| remote.force)
        {
            if options.remote_verify.is_some() && !looks_like_jws(token) {
                payload = None;
            } else {
                match verify_jws_access_token(token, jwks_url, options.verify_options.clone()).await
                {
                    Ok(result) => payload = Some(result.payload),
                    Err(error)
                        if options.remote_verify.is_some() && is_opaque_token_error(&error) =>
                    {
                        payload = None;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }

    if let Some(remote) = options.remote_verify {
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &remote.client_id)
            .append_pair("client_secret", &remote.client_secret)
            .append_pair("token", token)
            .append_pair("token_type_hint", "access_token")
            .finish();
        let introspect = reqwest::Client::new()
            .post(remote.introspect_url)
            .header("accept", "application/json")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        if !introspect
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(OAuthError::TokenVerification("token inactive".to_owned()));
        }
        validate_introspection_claims(&introspect, &options.verify_options)?;
        payload = Some(introspect);
    }

    let payload =
        payload.ok_or_else(|| OAuthError::TokenVerification("no token payload".to_owned()))?;
    validate_scopes(&payload, &options.scopes)?;
    Ok(payload)
}

fn validate_introspection_claims(
    payload: &Value,
    options: &TokenValidationOptions,
) -> Result<(), OAuthError> {
    let Some(claims) = payload.as_object() else {
        return Err(OAuthError::TokenVerification(
            "introspection payload must be an object".to_owned(),
        ));
    };
    validate_temporal_claims(claims)?;
    if !options.audience.is_empty()
        && claims.contains_key("aud")
        && !audience_matches(claims.get("aud"), &options.audience)
    {
        return Err(OAuthError::TokenVerification(
            "audience mismatch".to_owned(),
        ));
    }
    if !options.issuer.is_empty() {
        let issuer = claims.get("iss").and_then(Value::as_str);
        if !issuer.is_some_and(|issuer| options.issuer.iter().any(|expected| expected == issuer)) {
            return Err(OAuthError::TokenVerification("issuer mismatch".to_owned()));
        }
    }
    Ok(())
}

fn map_azp_to_client_id(payload: &mut Value) {
    let Some(authorized_party) = payload.get("azp").cloned() else {
        return;
    };
    if let Some(object) = payload.as_object_mut() {
        object.insert("client_id".to_owned(), authorized_party);
    }
}

fn looks_like_jws(token: &str) -> bool {
    token.split('.').count() == 3
}

fn is_opaque_token_error(error: &OAuthError) -> bool {
    matches!(error, OAuthError::Jose(message) if message.to_ascii_lowercase().contains("header"))
}

fn validate_scopes(payload: &Value, required_scopes: &[String]) -> Result<(), OAuthError> {
    if required_scopes.is_empty() {
        return Ok(());
    }
    let scopes = payload
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();
    for scope in required_scopes {
        if !scopes.contains(scope.as_str()) {
            return Err(OAuthError::TokenVerification(format!(
                "invalid scope {scope}"
            )));
        }
    }
    Ok(())
}
