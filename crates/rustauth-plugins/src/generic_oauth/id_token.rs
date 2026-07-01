use rustauth_oauth::oauth2::{
    validate_token, OAuth2Tokens, OAuth2UserInfo, OAuthError, OAuthHttpClient,
    TokenValidationOptions, ValidateTokenOptions,
};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

use super::config::{GenericOAuthConfig, GenericOAuthProfileSource};
use super::user_info;

pub(super) async fn verified_user_info(
    tokens: &OAuth2Tokens,
    config: &GenericOAuthConfig,
    expected_nonce: Option<&str>,
    http_client: &OAuthHttpClient,
) -> Result<Option<OAuth2UserInfo>, OAuthError> {
    let GenericOAuthProfileSource::VerifiedIdToken(profile) = &config.profile_source else {
        return Ok(None);
    };
    let id_token = tokens
        .id_token
        .as_deref()
        .ok_or_else(|| OAuthError::TokenVerification("missing id_token".to_owned()))?;
    let expected_nonce = expected_nonce
        .filter(|nonce| !nonce.is_empty())
        .ok_or_else(|| OAuthError::TokenVerification("missing oidc nonce".to_owned()))?;
    let issuer = profile
        .issuer
        .as_deref()
        .filter(|issuer| !issuer.is_empty());
    let issuer = issuer.ok_or_else(|| {
        OAuthError::InvalidConfiguration(
            "OIDC ID token profile extraction requires an issuer".to_owned(),
        )
    })?;
    let jwks_url = profile.jwks_url.as_deref().filter(|url| !url.is_empty());
    let jwks_url = jwks_url.ok_or_else(|| {
        OAuthError::InvalidConfiguration(
            "OIDC ID token profile extraction requires a JWKS URL".to_owned(),
        )
    })?;

    let mut validation = TokenValidationOptions::default()
        .require_standard_claims()
        .leeway_seconds(profile.leeway_seconds);
    validation.audience = vec![config.client_id.clone()];
    validation.issuer = vec![issuer.to_owned()];
    let mut options = ValidateTokenOptions::new(validation);
    options.http = Some(http_client.clone());
    let result = validate_token(id_token, jwks_url, options).await?;
    let claims = result
        .payload
        .as_object()
        .ok_or_else(|| OAuthError::TokenVerification("ID token payload is not JSON".to_owned()))?;
    validate_nonce(claims, expected_nonce)?;
    validate_authorized_party(claims, &config.client_id)?;
    Ok(user_info::user_info_from_claims(&result.payload))
}

fn validate_nonce(claims: &Map<String, Value>, expected_nonce: &str) -> Result<(), OAuthError> {
    let nonce = match claims.get("nonce") {
        Some(Value::String(nonce)) if !nonce.is_empty() => nonce,
        Some(_) => {
            return Err(OAuthError::InvalidClaim {
                claim: "nonce",
                reason: "must be a non-empty string".to_owned(),
            });
        }
        None => {
            return Err(OAuthError::InvalidClaim {
                claim: "nonce",
                reason: "missing required claim".to_owned(),
            });
        }
    };
    if nonce != expected_nonce {
        return Err(OAuthError::TokenVerification("nonce mismatch".to_owned()));
    }
    Ok(())
}

fn validate_authorized_party(
    claims: &Map<String, Value>,
    client_id: &str,
) -> Result<(), OAuthError> {
    if distinct_audience_count(claims.get("aud")) <= 1 {
        return Ok(());
    }
    let authorized_party = claims.get("azp").and_then(Value::as_str);
    if authorized_party == Some(client_id) {
        return Ok(());
    }
    Err(OAuthError::TokenVerification(
        "authorized party mismatch".to_owned(),
    ))
}

fn distinct_audience_count(audience: Option<&Value>) -> usize {
    let Some(Value::Array(audiences)) = audience else {
        return 0;
    };
    audiences
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>()
        .len()
}
