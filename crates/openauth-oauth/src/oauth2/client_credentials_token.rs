use super::error::OAuthError;
use super::request::{
    apply_client_authentication, post_form, ClientAuthentication, OAuthFormRequest,
};
use super::tokens::{get_oauth2_tokens, OAuth2Tokens, ProviderOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCredentialsTokenRequest {
    pub options: ProviderOptions,
    pub scope: Option<String>,
    pub authentication: ClientAuthentication,
    pub resource: Vec<String>,
}

impl Default for ClientCredentialsTokenRequest {
    fn default() -> Self {
        Self {
            options: ProviderOptions::default(),
            scope: None,
            authentication: ClientAuthentication::Post,
            resource: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCredentialsGrant {
    pub token_endpoint: String,
    pub request: ClientCredentialsTokenRequest,
}

pub fn create_client_credentials_token_request(
    input: ClientCredentialsTokenRequest,
) -> Result<OAuthFormRequest, OAuthError> {
    let mut request = OAuthFormRequest::new();
    request.set_body("grant_type", "client_credentials");
    if let Some(scope) = input.scope {
        request.set_body("scope", scope);
    }
    for resource in input.resource {
        request.push_body("resource", resource);
    }
    apply_client_authentication(&mut request, &input.options, input.authentication, true)?;
    Ok(request)
}

pub fn client_credentials_token_request(
    input: ClientCredentialsTokenRequest,
) -> Result<OAuthFormRequest, OAuthError> {
    create_client_credentials_token_request(input)
}

pub async fn client_credentials_token(
    input: ClientCredentialsGrant,
) -> Result<OAuth2Tokens, OAuthError> {
    let request = client_credentials_token_request(input.request)?;
    let data = post_form(&input.token_endpoint, request).await?;
    get_oauth2_tokens(data)
}
