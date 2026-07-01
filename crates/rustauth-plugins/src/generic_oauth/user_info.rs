use rustauth_oauth::oauth2::{OAuth2Tokens, OAuth2UserInfo, OAuthError, OAuthHttpClient};
use serde_json::Value;

pub async fn get_user_info(
    tokens: &OAuth2Tokens,
    user_info_url: Option<&str>,
    http_client: &OAuthHttpClient,
) -> Result<Option<OAuth2UserInfo>, OAuthError> {
    let Some(url) = user_info_url else {
        return Ok(None);
    };
    let Some(access_token) = tokens.access_token.as_deref() else {
        return Ok(None);
    };
    let bytes = http_client
        .get_bytes_with_headers(url, &[("authorization", &format!("Bearer {access_token}"))])
        .await?;
    let profile = serde_json::from_slice::<Value>(&bytes)
        .map_err(|error| OAuthError::InvalidResponse(error.to_string()))?;
    Ok(user_info_from_claims(&profile))
}

pub fn user_info_from_claims(profile: &Value) -> Option<OAuth2UserInfo> {
    let id = string_value(profile, "sub")
        .or_else(|| string_value(profile, "id"))
        .or_else(|| string_value(profile, "user_id"))
        .unwrap_or_default();
    if id.is_empty() {
        return None;
    }
    Some(OAuth2UserInfo {
        id,
        name: string_value(profile, "name")
            .or_else(|| string_value(profile, "preferred_username"))
            .or_else(|| full_name(profile)),
        email: string_value(profile, "email")
            .or_else(|| string_value(profile, "preferred_username")),
        image: string_value(profile, "picture").or_else(|| string_value(profile, "image")),
        email_verified: profile
            .get("email_verified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn string_value(profile: &Value, key: &str) -> Option<String> {
    match profile.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn full_name(profile: &Value) -> Option<String> {
    let given = string_value(profile, "given_name").unwrap_or_default();
    let family = string_value(profile, "family_name").unwrap_or_default();
    let name = format!("{given} {family}").trim().to_owned();
    (!name.is_empty()).then_some(name)
}
