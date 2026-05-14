//! Small framework-neutral helpers for MCP resource servers.

use http::{header, HeaderValue, Request, Response, StatusCode};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAuthClientOptions {
    pub auth_url: String,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAuthClient {
    auth_url: String,
    resource: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonRpcUnauthorized<'a> {
    jsonrpc: &'static str,
    error: JsonRpcError<'a>,
    id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError<'a> {
    code: i64,
    message: &'a str,
    #[serde(rename = "www-authenticate")]
    www_authenticate: &'a str,
}

impl McpAuthClient {
    pub fn new(options: McpAuthClientOptions) -> Self {
        Self {
            auth_url: options.auth_url.trim_end_matches('/').to_owned(),
            resource: options.resource,
        }
    }

    pub fn www_authenticate(&self) -> String {
        let base = self.resource.as_deref().unwrap_or(&self.auth_url);
        format!("Bearer resource_metadata=\"{base}/.well-known/oauth-protected-resource\"")
    }

    pub fn unauthorized_response(&self) -> Result<Response<Vec<u8>>, http::Error> {
        let authenticate = self.www_authenticate();
        let body = serde_json::to_vec(&JsonRpcUnauthorized {
            jsonrpc: "2.0",
            error: JsonRpcError {
                code: -32000,
                message: "Unauthorized: Authentication required",
                www_authenticate: &authenticate,
            },
            id: None,
        })
        .unwrap_or_default();
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::WWW_AUTHENTICATE, authenticate)
            .body(body)
    }

    pub fn cors_preflight_response(&self) -> Result<Response<Vec<u8>>, http::Error> {
        Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, self.allowed_origin())
            .header(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                "Content-Type, Authorization",
            )
            .header(header::ACCESS_CONTROL_MAX_AGE, "86400")
            .body(Vec::new())
    }

    pub fn bearer_token<B>(&self, request: &Request<B>) -> Option<String> {
        request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::to_owned)
    }

    fn allowed_origin(&self) -> HeaderValue {
        url::Url::parse(&self.auth_url)
            .ok()
            .and_then(|url| {
                let scheme = url.scheme();
                let host = url.host_str()?;
                let port = url
                    .port()
                    .map(|port| format!(":{port}"))
                    .unwrap_or_default();
                HeaderValue::from_str(&format!("{scheme}://{host}{port}")).ok()
            })
            .unwrap_or_else(|| HeaderValue::from_static("*"))
    }
}
