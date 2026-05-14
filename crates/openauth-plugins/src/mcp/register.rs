use http::{header, Method, StatusCode};
use openauth_core::api::{
    create_auth_endpoint, parse_request_body, AsyncAuthEndpoint, AuthEndpointOptions,
};
use openauth_core::db::{Create, DbValue};
use serde_json::{json, Value};
use time::OffsetDateTime;

use super::shared::{
    adapter, current_session, json_response, oauth_error, random_token, with_cors,
};
use super::{ResolvedMcpOptions, TokenEndpointAuthMethod};

pub fn register_endpoint(_options: ResolvedMcpOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/mcp/register",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("registerMcpClient")
            .allowed_media_types(["application/json"]),
        move |context, request| {
            Box::pin(async move {
                let adapter = adapter(context)?;
                let body: Value = parse_request_body(&request)?;
                let grant_types = string_array(&body, "grant_types")
                    .unwrap_or_else(|| vec!["authorization_code".to_owned()]);
                let response_types = string_array(&body, "response_types")
                    .unwrap_or_else(|| vec!["code".to_owned()]);
                let redirect_uris = string_array(&body, "redirect_uris").unwrap_or_default();

                if requires_redirect_uri(&grant_types) && redirect_uris.is_empty() {
                    return oauth_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_redirect_uri",
                        "Redirect URIs are required for authorization_code and implicit grant types",
                    );
                }
                if grant_types
                    .iter()
                    .any(|grant| grant == "authorization_code")
                    && !response_types.iter().any(|response| response == "code")
                {
                    return oauth_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_client_metadata",
                        "When 'authorization_code' grant type is used, 'code' response type must be included",
                    );
                }
                if grant_types.iter().any(|grant| grant == "implicit")
                    && !response_types.iter().any(|response| response == "token")
                {
                    return oauth_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_client_metadata",
                        "When 'implicit' grant type is used, 'token' response type must be included",
                    );
                }

                let session = current_session(adapter.as_ref(), context, &request).await?;
                let auth_method = auth_method(&body);
                let client_type = if auth_method == TokenEndpointAuthMethod::None {
                    "public"
                } else {
                    "web"
                };
                let client_id = random_token();
                let client_secret = (client_type != "public").then(random_token);
                let now = OffsetDateTime::now_utc();

                adapter
                    .create(
                        Create::new("oauthApplication")
                            .data("name", nullable_string(&body, "client_name"))
                            .data("icon", nullable_string(&body, "logo_uri"))
                            .data("metadata", metadata_string(&body))
                            .data("clientId", DbValue::String(client_id.clone()))
                            .data(
                                "clientSecret",
                                client_secret
                                    .clone()
                                    .map(DbValue::String)
                                    .unwrap_or(DbValue::Null),
                            )
                            .data("redirectUrls", DbValue::String(redirect_uris.join(",")))
                            .data("type", DbValue::String(client_type.to_owned()))
                            .data(
                                "authenticationScheme",
                                DbValue::String(auth_method.as_str().to_owned()),
                            )
                            .data("disabled", DbValue::Boolean(false))
                            .data(
                                "userId",
                                session
                                    .map(|session| DbValue::String(session.user_id))
                                    .unwrap_or(DbValue::Null),
                            )
                            .data("createdAt", DbValue::Timestamp(now))
                            .data("updatedAt", DbValue::Timestamp(now)),
                    )
                    .await?;

                let mut response = json!({
                    "client_id": client_id,
                    "client_id_issued_at": now.unix_timestamp(),
                    "redirect_uris": redirect_uris,
                    "token_endpoint_auth_method": auth_method.as_str(),
                    "grant_types": grant_types,
                    "response_types": response_types,
                    "client_name": string_value(&body, "client_name"),
                    "client_uri": string_value(&body, "client_uri"),
                    "logo_uri": string_value(&body, "logo_uri"),
                    "scope": string_value(&body, "scope"),
                    "contacts": string_array(&body, "contacts"),
                    "tos_uri": string_value(&body, "tos_uri"),
                    "policy_uri": string_value(&body, "policy_uri"),
                    "jwks_uri": string_value(&body, "jwks_uri"),
                    "jwks": body.get("jwks").cloned(),
                    "software_id": string_value(&body, "software_id"),
                    "software_version": string_value(&body, "software_version"),
                    "software_statement": string_value(&body, "software_statement"),
                    "metadata": body.get("metadata").cloned(),
                });
                if let Some(secret) = client_secret {
                    response["client_secret"] = Value::String(secret);
                    response["client_secret_expires_at"] = json!(0);
                }
                let mut response = json_response(StatusCode::CREATED, &response)?;
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    http::HeaderValue::from_static("no-store"),
                );
                response
                    .headers_mut()
                    .insert(header::PRAGMA, http::HeaderValue::from_static("no-cache"));
                with_cors(response)
            })
        },
    )
}

fn requires_redirect_uri(grant_types: &[String]) -> bool {
    grant_types.is_empty()
        || grant_types
            .iter()
            .any(|grant| grant == "authorization_code" || grant == "implicit")
}

fn auth_method(body: &Value) -> TokenEndpointAuthMethod {
    match string_value(body, "token_endpoint_auth_method").as_deref() {
        Some("none") => TokenEndpointAuthMethod::None,
        Some("client_secret_post") => TokenEndpointAuthMethod::ClientSecretPost,
        _ => TokenEndpointAuthMethod::ClientSecretBasic,
    }
}

fn string_value(body: &Value, field: &str) -> Option<String> {
    body.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn nullable_string(body: &Value, field: &str) -> DbValue {
    string_value(body, field)
        .map(DbValue::String)
        .unwrap_or(DbValue::Null)
}

fn metadata_string(body: &Value) -> DbValue {
    body.get("metadata")
        .map(|metadata| DbValue::String(metadata.to_string()))
        .unwrap_or(DbValue::Null)
}

fn string_array(body: &Value, field: &str) -> Option<Vec<String>> {
    body.get(field)?.as_array().map(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}
