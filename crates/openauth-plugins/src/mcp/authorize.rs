use http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use openauth_core::db::{Create, DbValue};
use serde_json::json;
use time::{Duration, OffsetDateTime};

use super::shared::{
    adapter, current_session, find_client, random_token, redirect, redirect_error_url,
};
use super::ResolvedMcpOptions;

pub fn authorize_endpoint(options: ResolvedMcpOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/mcp/authorize",
        Method::GET,
        AuthEndpointOptions::new().operation_id("mcpOAuthAuthorize"),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = adapter(context)?;
                let mut query = query_map(request.uri().query().unwrap_or_default());
                let Some(session) = current_session(adapter.as_ref(), context, &request).await?
                else {
                    let target = if request.uri().query().is_some() {
                        format!(
                            "{}?{}",
                            options.login_page,
                            request.uri().query().unwrap_or_default()
                        )
                    } else {
                        options.login_page.clone()
                    };
                    return redirect(&target);
                };
                let error_url = format!("{}{}{}", context.base_url, context.base_path, "/error");

                let Some(client_id) = query.get("client_id").cloned() else {
                    return redirect(&format!("{error_url}?error=invalid_client"));
                };
                if !query.contains_key("response_type") {
                    return redirect(&redirect_error_url(
                        &error_url,
                        "invalid_request",
                        "response_type is required",
                    ));
                }
                let Some(client) = find_client(adapter.as_ref(), &client_id).await? else {
                    return redirect(&format!("{error_url}?error=invalid_client"));
                };
                let Some(redirect_uri) = query.get("redirect_uri").cloned() else {
                    return super::shared::oauth_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_request",
                        "redirect_uri is required",
                    );
                };
                if !client.redirect_urls.iter().any(|url| url == &redirect_uri) {
                    return super::shared::oauth_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_request",
                        "Invalid redirect URI",
                    );
                }
                if client.disabled {
                    return redirect(&format!("{error_url}?error=client_disabled"));
                }
                if query.get("response_type").map(String::as_str) != Some("code") {
                    return redirect(&format!("{error_url}?error=unsupported_response_type"));
                }

                let request_scope = query
                    .get("scope")
                    .map(|scope| {
                        scope
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec!["openid".to_owned()]);
                let invalid_scopes = request_scope
                    .iter()
                    .filter(|scope| !options.scopes.contains(scope))
                    .cloned()
                    .collect::<Vec<_>>();
                if !invalid_scopes.is_empty() {
                    return redirect(&redirect_error_url(
                        &redirect_uri,
                        "invalid_scope",
                        &format!(
                            "The following scopes are invalid: {}",
                            invalid_scopes.join(", ")
                        ),
                    ));
                }

                let has_challenge = query.contains_key("code_challenge");
                let has_method = query.contains_key("code_challenge_method");
                if options.require_pkce && (!has_challenge || !has_method) {
                    return redirect(&redirect_error_url(
                        &redirect_uri,
                        "invalid_request",
                        "pkce is required",
                    ));
                }
                if !has_method {
                    query.insert("code_challenge_method".to_owned(), "plain".to_owned());
                }
                let method = query
                    .get("code_challenge_method")
                    .map(|value| value.to_ascii_lowercase())
                    .unwrap_or_else(|| "plain".to_owned());
                let method_allowed = method == "s256"
                    || (options.allow_plain_code_challenge_method && method == "plain");
                if !method_allowed {
                    return redirect(&redirect_error_url(
                        &redirect_uri,
                        "invalid_request",
                        "invalid code_challenge method",
                    ));
                }

                let code = random_token();
                let now = OffsetDateTime::now_utc();
                let value = json!({
                    "clientId": client.client_id,
                    "redirectURI": redirect_uri,
                    "scope": request_scope,
                    "userId": session.user_id,
                    "authTime": session.created_at.unix_timestamp(),
                    "requireConsent": query.get("prompt").map(String::as_str) == Some("consent"),
                    "state": query.get("state"),
                    "codeChallenge": query.get("code_challenge"),
                    "codeChallengeMethod": query.get("code_challenge_method"),
                    "nonce": query.get("nonce"),
                });
                adapter
                    .create(
                        Create::new("verification")
                            .data("id", DbValue::String(format!("mcp_code_{code}")))
                            .data("identifier", DbValue::String(code.clone()))
                            .data("value", DbValue::String(value.to_string()))
                            .data(
                                "expires_at",
                                DbValue::Timestamp(
                                    now + Duration::seconds(options.code_expires_in as i64),
                                ),
                            )
                            .data("created_at", DbValue::Timestamp(now))
                            .data("updated_at", DbValue::Timestamp(now)),
                    )
                    .await?;

                if query.get("prompt").map(String::as_str) == Some("consent") {
                    if let Some(consent_page) = &options.consent_page {
                        let consent_uri = format!(
                            "{consent_page}?consent_code={code}&client_id={}&scope={}",
                            client.client_id,
                            request_scope.join(" ")
                        );
                        return redirect(&consent_uri);
                    }
                }

                let mut redirect_url = url::Url::parse(&redirect_uri)
                    .map_err(|error| openauth_core::error::OpenAuthError::Api(error.to_string()))?;
                redirect_url.query_pairs_mut().append_pair("code", &code);
                if let Some(state) = query.get("state") {
                    redirect_url.query_pairs_mut().append_pair("state", state);
                }
                redirect(redirect_url.as_str())
            })
        },
    )
}

fn query_map(query: &str) -> std::collections::BTreeMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}
