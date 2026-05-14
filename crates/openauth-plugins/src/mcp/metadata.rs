use http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use openauth_core::context::AuthContext;
use serde::Serialize;

use super::shared::{json_response, with_cors};
use super::ResolvedMcpOptions;

#[derive(Debug, Serialize)]
struct AuthorizationServerMetadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    registration_endpoint: String,
    scopes_supported: Vec<String>,
    response_types_supported: Vec<&'static str>,
    response_modes_supported: Vec<&'static str>,
    grant_types_supported: Vec<&'static str>,
    acr_values_supported: Vec<&'static str>,
    subject_types_supported: Vec<&'static str>,
    id_token_signing_alg_values_supported: Vec<&'static str>,
    token_endpoint_auth_methods_supported: Vec<&'static str>,
    code_challenge_methods_supported: Vec<&'static str>,
    claims_supported: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ProtectedResourceMetadata {
    resource: String,
    authorization_servers: Vec<String>,
    jwks_uri: String,
    scopes_supported: Vec<String>,
    bearer_methods_supported: Vec<&'static str>,
    resource_signing_alg_values_supported: Vec<&'static str>,
}

pub fn authorization_server_endpoint(options: ResolvedMcpOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/.well-known/oauth-authorization-server",
        Method::GET,
        AuthEndpointOptions::new().operation_id("getMcpOAuthConfig"),
        move |context, _request| {
            let options = options.clone();
            Box::pin(async move {
                let metadata = authorization_server_metadata(context, &options);
                with_cors(json_response(StatusCode::OK, &metadata)?)
            })
        },
    )
}

pub fn protected_resource_endpoint(options: ResolvedMcpOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/.well-known/oauth-protected-resource",
        Method::GET,
        AuthEndpointOptions::new().operation_id("getMcpProtectedResource"),
        move |context, _request| {
            let options = options.clone();
            Box::pin(async move {
                let metadata = protected_resource_metadata(context, &options);
                with_cors(json_response(StatusCode::OK, &metadata)?)
            })
        },
    )
}

fn authorization_server_metadata(
    context: &AuthContext,
    options: &ResolvedMcpOptions,
) -> AuthorizationServerMetadata {
    let issuer = context.base_url.clone();
    let base = auth_base_url(context);
    AuthorizationServerMetadata {
        issuer,
        authorization_endpoint: format!("{base}/mcp/authorize"),
        token_endpoint: format!("{base}/mcp/token"),
        userinfo_endpoint: format!("{base}/mcp/userinfo"),
        jwks_uri: format!("{base}/mcp/jwks"),
        registration_endpoint: format!("{base}/mcp/register"),
        scopes_supported: options.scopes.clone(),
        response_types_supported: vec!["code"],
        response_modes_supported: vec!["query"],
        grant_types_supported: vec!["authorization_code", "refresh_token"],
        acr_values_supported: vec![
            "urn:mace:incommon:iap:silver",
            "urn:mace:incommon:iap:bronze",
        ],
        subject_types_supported: vec!["public"],
        id_token_signing_alg_values_supported: vec!["HS256", "none"],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic",
            "client_secret_post",
            "none",
        ],
        code_challenge_methods_supported: vec!["S256"],
        claims_supported: vec![
            "sub",
            "iss",
            "aud",
            "exp",
            "nbf",
            "iat",
            "jti",
            "email",
            "email_verified",
            "name",
        ],
    }
}

fn protected_resource_metadata(
    context: &AuthContext,
    options: &ResolvedMcpOptions,
) -> ProtectedResourceMetadata {
    let origin = origin_from_base_url(&context.base_url);
    let base = auth_base_url(context);
    ProtectedResourceMetadata {
        resource: options.resource.clone().unwrap_or_else(|| origin.clone()),
        authorization_servers: vec![origin],
        jwks_uri: format!("{base}/mcp/jwks"),
        scopes_supported: options.scopes.clone(),
        bearer_methods_supported: vec!["header"],
        resource_signing_alg_values_supported: vec!["HS256", "none"],
    }
}

fn auth_base_url(context: &AuthContext) -> String {
    format!(
        "{}{}",
        context.base_url.trim_end_matches('/'),
        context.base_path.trim_end_matches('/')
    )
}

fn origin_from_base_url(base_url: &str) -> String {
    url::Url::parse(base_url)
        .ok()
        .and_then(|url| {
            let scheme = url.scheme();
            let host = url.host_str()?;
            let port = url
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default();
            Some(format!("{scheme}://{host}{port}"))
        })
        .unwrap_or_else(|| base_url.trim_end_matches('/').to_owned())
}
