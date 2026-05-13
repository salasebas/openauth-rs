use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Duration, OffsetDateTime};

use super::error::OAuthError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientId {
    Single(String),
    Multiple(Vec<String>),
}

impl ClientId {
    pub fn primary(&self) -> Option<&str> {
        match self {
            Self::Single(value) if !value.is_empty() => Some(value),
            Self::Single(_) => None,
            Self::Multiple(values) => values
                .first()
                .map(String::as_str)
                .filter(|value| !value.is_empty()),
        }
    }
}

impl From<&str> for ClientId {
    fn from(value: &str) -> Self {
        Self::Single(value.to_owned())
    }
}

impl From<String> for ClientId {
    fn from(value: String) -> Self {
        Self::Single(value)
    }
}

impl From<Vec<String>> for ClientId {
    fn from(value: Vec<String>) -> Self {
        Self::Multiple(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderOptions {
    pub client_id: Option<ClientId>,
    pub client_secret: Option<String>,
    pub scope: Vec<String>,
    pub disable_default_scope: bool,
    pub redirect_uri: Option<String>,
    pub authorization_endpoint: Option<String>,
    pub client_key: Option<String>,
    pub disable_id_token_sign_in: bool,
    pub disable_implicit_sign_up: bool,
    pub disable_sign_up: bool,
    pub prompt: Option<String>,
    pub response_mode: Option<String>,
    pub override_user_info_on_sign_in: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuth2Tokens {
    pub token_type: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: Option<OffsetDateTime>,
    pub refresh_token_expires_at: Option<OffsetDateTime>,
    pub scopes: Vec<String>,
    pub id_token: Option<String>,
    pub raw: Value,
}

impl Default for OAuth2Tokens {
    fn default() -> Self {
        Self {
            token_type: None,
            access_token: None,
            refresh_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scopes: Vec::new(),
            id_token: None,
            raw: Value::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuth2UserInfo {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub image: Option<String>,
    pub email_verified: bool,
}

pub fn get_primary_client_id(client_id: &Option<ClientId>) -> Option<&str> {
    client_id.as_ref().and_then(ClientId::primary)
}

pub fn get_oauth2_tokens(data: Value) -> Result<OAuth2Tokens, OAuthError> {
    let object = data.as_object().ok_or_else(|| {
        OAuthError::InvalidResponse("token response must be a JSON object".to_owned())
    })?;
    let now = OffsetDateTime::now_utc();
    let expires_at = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_i64)
            .map(|seconds| now + Duration::seconds(seconds))
    };

    Ok(OAuth2Tokens {
        token_type: string_field(object, "token_type"),
        access_token: string_field(object, "access_token"),
        refresh_token: string_field(object, "refresh_token"),
        access_token_expires_at: expires_at("expires_in"),
        refresh_token_expires_at: expires_at("refresh_token_expires_in"),
        scopes: scopes_field(object.get("scope")),
        id_token: string_field(object, "id_token"),
        raw: data,
    })
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn scopes_field(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(scope)) => scope.split_whitespace().map(str::to_owned).collect(),
        Some(Value::Array(scopes)) => scopes
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}
