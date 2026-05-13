//! Vercel social OAuth provider.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use openauth_oauth::oauth2::{
    authorization_code_request, create_authorization_url, validate_authorization_code,
    AuthorizationCodeRequest, AuthorizationUrlRequest, ClientAuthentication, ClientTokenRequest,
    OAuth2Tokens, OAuth2UserInfo, OAuthError, OAuthFormRequest, OAuthProviderContract,
    ProviderOptions,
};
use serde::{Deserialize, Serialize};
use url::Url;

pub const VERCEL_ID: &str = "vercel";
pub const VERCEL_NAME: &str = "Vercel";
pub const VERCEL_AUTHORIZATION_ENDPOINT: &str = "https://vercel.com/oauth/authorize";
pub const VERCEL_TOKEN_ENDPOINT: &str = "https://api.vercel.com/login/oauth/token";
pub const VERCEL_USERINFO_ENDPOINT: &str = "https://api.vercel.com/login/oauth/userinfo";

pub type VercelUserInfoFuture =
    Pin<Box<dyn Future<Output = Result<Option<VercelUserInfo>, OAuthError>> + Send>>;
pub type VercelGetUserInfo = Arc<dyn Fn(OAuth2Tokens) -> VercelUserInfoFuture + Send + Sync>;
pub type VercelProfileMapper = Arc<dyn Fn(&VercelProfile) -> VercelUserPatch + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VercelProfile {
    pub sub: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VercelUserPatch {
    pub id: Option<String>,
    pub name: Option<Option<String>>,
    pub email: Option<Option<String>>,
    pub image: Option<Option<String>>,
    pub email_verified: Option<bool>,
}

impl VercelUserPatch {
    fn apply_to(self, user: &mut OAuth2UserInfo) {
        if let Some(id) = self.id {
            user.id = id;
        }
        if let Some(name) = self.name {
            user.name = name;
        }
        if let Some(email) = self.email {
            user.email = email;
        }
        if let Some(image) = self.image {
            user.image = image;
        }
        if let Some(email_verified) = self.email_verified {
            user.email_verified = email_verified;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VercelUserInfo {
    pub user: OAuth2UserInfo,
    pub data: VercelProfile,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VercelAuthorizationUrlRequest {
    pub state: String,
    pub redirect_uri: String,
    pub code_verifier: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Clone, Default)]
pub struct VercelOptions {
    pub oauth: ProviderOptions,
    pub get_user_info: Option<VercelGetUserInfo>,
    pub map_profile_to_user: Option<VercelProfileMapper>,
}

impl From<ProviderOptions> for VercelOptions {
    fn from(oauth: ProviderOptions) -> Self {
        Self {
            oauth,
            get_user_info: None,
            map_profile_to_user: None,
        }
    }
}

#[derive(Clone)]
pub struct VercelProvider {
    options: VercelOptions,
    http_client: reqwest::Client,
}

pub fn vercel(options: impl Into<VercelOptions>) -> VercelProvider {
    VercelProvider::new(options)
}

impl VercelProvider {
    pub fn new(options: impl Into<VercelOptions>) -> Self {
        Self {
            options: options.into(),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn id(&self) -> &str {
        VERCEL_ID
    }

    pub fn name(&self) -> &str {
        VERCEL_NAME
    }

    pub fn options(&self) -> &VercelOptions {
        &self.options
    }

    pub fn provider_options(&self) -> &ProviderOptions {
        &self.options.oauth
    }

    pub fn token_endpoint(&self) -> &str {
        VERCEL_TOKEN_ENDPOINT
    }

    pub fn userinfo_endpoint(&self) -> &str {
        VERCEL_USERINFO_ENDPOINT
    }

    pub fn create_authorization_url(
        &self,
        request: VercelAuthorizationUrlRequest,
    ) -> Result<Url, OAuthError> {
        let code_verifier = request
            .code_verifier
            .ok_or(OAuthError::MissingOption("code_verifier"))?;

        create_authorization_url(AuthorizationUrlRequest {
            id: VERCEL_ID.to_owned(),
            options: self.options.oauth.clone(),
            authorization_endpoint: VERCEL_AUTHORIZATION_ENDPOINT.to_owned(),
            redirect_uri: request.redirect_uri,
            state: request.state,
            code_verifier: Some(code_verifier),
            scopes: self.scopes(request.scopes),
            ..AuthorizationUrlRequest::default()
        })
    }

    pub fn authorization_code_request(
        &self,
        code: impl Into<String>,
        code_verifier: Option<impl Into<String>>,
        redirect_uri: impl Into<String>,
    ) -> Result<OAuthFormRequest, OAuthError> {
        authorization_code_request(AuthorizationCodeRequest {
            code: code.into(),
            redirect_uri: redirect_uri.into(),
            options: self.options.oauth.clone(),
            code_verifier: code_verifier.map(Into::into),
            authentication: ClientAuthentication::Post,
            ..AuthorizationCodeRequest::default()
        })
    }

    pub async fn validate_authorization_code(
        &self,
        code: impl Into<String>,
        code_verifier: Option<impl Into<String>>,
        redirect_uri: impl Into<String>,
    ) -> Result<OAuth2Tokens, OAuthError> {
        validate_authorization_code(ClientTokenRequest {
            token_endpoint: VERCEL_TOKEN_ENDPOINT.to_owned(),
            request: AuthorizationCodeRequest {
                code: code.into(),
                redirect_uri: redirect_uri.into(),
                options: self.options.oauth.clone(),
                code_verifier: code_verifier.map(Into::into),
                authentication: ClientAuthentication::Post,
                ..AuthorizationCodeRequest::default()
            },
        })
        .await
    }

    pub async fn get_user_info(
        &self,
        token: &OAuth2Tokens,
    ) -> Result<Option<VercelUserInfo>, OAuthError> {
        if let Some(get_user_info) = &self.options.get_user_info {
            return get_user_info(token.clone()).await;
        }

        let Some(access_token) = token.access_token.as_deref() else {
            return Ok(None);
        };

        let response = self
            .http_client
            .get(VERCEL_USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .await?;
        if !response.status().is_success() {
            return Ok(None);
        }

        let profile = response.json::<VercelProfile>().await?;
        Ok(Some(self.map_profile(profile)))
    }

    pub fn user_info_from_profile(profile: VercelProfile) -> VercelUserInfo {
        let name = profile
            .name
            .clone()
            .or_else(|| profile.preferred_username.clone())
            .unwrap_or_default();

        VercelUserInfo {
            user: OAuth2UserInfo {
                id: profile.sub.clone(),
                name: Some(name),
                email: profile.email.clone(),
                image: profile.picture.clone(),
                email_verified: profile.email_verified.unwrap_or(false),
            },
            data: profile,
        }
    }

    pub fn map_profile(&self, profile: VercelProfile) -> VercelUserInfo {
        let mut user_info = Self::user_info_from_profile(profile);
        if let Some(map_profile_to_user) = &self.options.map_profile_to_user {
            map_profile_to_user(&user_info.data).apply_to(&mut user_info.user);
        }
        user_info
    }

    fn scopes(&self, request_scopes: Vec<String>) -> Vec<String> {
        let mut scopes = Vec::new();
        scopes.extend(self.options.oauth.scope.iter().cloned());
        scopes.extend(request_scopes);
        scopes
    }
}

impl OAuthProviderContract for VercelProvider {
    fn id(&self) -> &str {
        self.id()
    }

    fn name(&self) -> &str {
        self.name()
    }
}
