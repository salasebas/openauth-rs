//! Model Context Protocol OAuth plugin.

mod authorize;
pub mod client;
mod metadata;
mod register;
mod schema;
mod session;
mod shared;
mod token;

use openauth_core::plugin::AuthPlugin;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const UPSTREAM_PLUGIN_ID: &str = "mcp";

const DEFAULT_SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];

/// Token endpoint authentication methods accepted by dynamic registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenEndpointAuthMethod {
    None,
    ClientSecretBasic,
    ClientSecretPost,
}

impl TokenEndpointAuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ClientSecretBasic => "client_secret_basic",
            Self::ClientSecretPost => "client_secret_post",
        }
    }
}

/// Optional OIDC-style settings used by the MCP plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpOidcConfig {
    pub scopes: Vec<String>,
    pub code_expires_in: u64,
    pub access_token_expires_in: u64,
    pub refresh_token_expires_in: u64,
    pub allow_plain_code_challenge_method: bool,
    pub require_pkce: bool,
}

impl Default for McpOidcConfig {
    fn default() -> Self {
        Self {
            scopes: Vec::new(),
            code_expires_in: 600,
            access_token_expires_in: 3600,
            refresh_token_expires_in: 604800,
            allow_plain_code_challenge_method: true,
            require_pkce: false,
        }
    }
}

/// User-facing MCP plugin options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpOptions {
    pub login_page: String,
    pub consent_page: Option<String>,
    pub resource: Option<String>,
    pub oidc_config: McpOidcConfig,
}

/// Resolved MCP options after upstream-compatible defaults are applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedMcpOptions {
    pub login_page: String,
    pub consent_page: Option<String>,
    pub resource: Option<String>,
    pub scopes: Vec<String>,
    pub code_expires_in: u64,
    pub access_token_expires_in: u64,
    pub refresh_token_expires_in: u64,
    pub allow_plain_code_challenge_method: bool,
    pub require_pkce: bool,
}

/// Typed MCP plugin returned by [`mcp`].
#[derive(Debug, Clone)]
pub struct McpPlugin {
    pub id: String,
    pub version: String,
    pub options: ResolvedMcpOptions,
    auth_plugin: AuthPlugin,
}

impl McpPlugin {
    pub fn into_auth_plugin(self) -> AuthPlugin {
        self.auth_plugin
    }

    pub fn as_auth_plugin(&self) -> &AuthPlugin {
        &self.auth_plugin
    }
}

/// MCP configuration errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpConfigError {
    #[error("login_page is required")]
    MissingLoginPage,
}

/// Build the MCP OAuth plugin.
pub fn mcp(options: McpOptions) -> Result<McpPlugin, McpConfigError> {
    if options.login_page.is_empty() {
        return Err(McpConfigError::MissingLoginPage);
    }

    let mut scopes = DEFAULT_SCOPES
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for scope in options.oidc_config.scopes {
        if !scope.is_empty() && !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }

    let resolved = ResolvedMcpOptions {
        login_page: options.login_page,
        consent_page: options.consent_page,
        resource: options.resource,
        scopes,
        code_expires_in: options.oidc_config.code_expires_in,
        access_token_expires_in: options.oidc_config.access_token_expires_in,
        refresh_token_expires_in: options.oidc_config.refresh_token_expires_in,
        allow_plain_code_challenge_method: options.oidc_config.allow_plain_code_challenge_method,
        require_pkce: options.oidc_config.require_pkce,
    };

    let auth_plugin = AuthPlugin::new(UPSTREAM_PLUGIN_ID)
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_options(serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null))
        .with_schema(schema::oauth_application_schema())
        .with_schema(schema::oauth_access_token_schema())
        .with_schema(schema::oauth_consent_schema())
        .with_endpoint(metadata::authorization_server_endpoint(resolved.clone()))
        .with_endpoint(metadata::protected_resource_endpoint(resolved.clone()))
        .with_endpoint(register::register_endpoint(resolved.clone()))
        .with_endpoint(authorize::authorize_endpoint(resolved.clone()))
        .with_endpoint(token::token_endpoint(resolved.clone()))
        .with_endpoint(session::get_session_endpoint());

    Ok(McpPlugin {
        id: UPSTREAM_PLUGIN_ID.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        options: resolved,
        auth_plugin,
    })
}
