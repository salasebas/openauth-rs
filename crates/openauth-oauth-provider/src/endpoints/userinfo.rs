use super::*;

pub(super) fn userinfo_endpoint(options: Arc<ResolvedOAuthProviderOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/oauth2/userinfo",
        Method::GET,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                let Some(adapter) = context.adapter() else {
                    return error_response(OAuthProviderError::invalid_request(
                        "database adapter required",
                    ));
                };
                let Some(token) = bearer_token(&request) else {
                    return error_response(OAuthProviderError::new(
                        StatusCode::UNAUTHORIZED,
                        "invalid_request",
                        "authorization header not found",
                    ));
                };
                let Some(validated) =
                    validate_access_token(context, adapter.as_ref(), &options, &token).await?
                else {
                    return error_response(OAuthProviderError::new(
                        StatusCode::UNAUTHORIZED,
                        "invalid_token",
                        "invalid token",
                    ));
                };
                if !validated.active {
                    return error_response(OAuthProviderError::new(
                        StatusCode::UNAUTHORIZED,
                        "invalid_token",
                        "invalid token",
                    ));
                }
                if !validated.scopes.iter().any(|scope| scope == "openid") {
                    return error_response(OAuthProviderError::invalid_scope(
                        "Missing required scope",
                    ));
                }
                let Some(user_id) = validated.user_id.as_deref() else {
                    return error_response(OAuthProviderError::invalid_request("user not found"));
                };
                let Some(user) = adapter
                    .find_one(crate::utils::find_by_string("user", "id", user_id))
                    .await?
                    .map(crate::utils::user_from_record)
                    .transpose()?
                else {
                    return error_response(OAuthProviderError::invalid_request("user not found"));
                };
                let sub = if let Some(client_id) = validated.client_id.as_deref() {
                    match get_client_cached(adapter.as_ref(), &options, client_id).await? {
                        Some(client) => {
                            crate::token::resolve_subject_identifier(&user.id, &client, &options)?
                        }
                        None => user.id.clone(),
                    }
                } else {
                    user.id.clone()
                };
                let mut claims = serde_json::Map::new();
                claims.insert("sub".to_owned(), serde_json::Value::String(sub));
                if validated.scopes.iter().any(|scope| scope == "profile") {
                    claims.insert(
                        "name".to_owned(),
                        serde_json::Value::String(user.name.clone()),
                    );
                    if let Some(image) = &user.image {
                        claims.insert(
                            "picture".to_owned(),
                            serde_json::Value::String(image.clone()),
                        );
                    }
                }
                if validated.scopes.iter().any(|scope| scope == "email") {
                    claims.insert(
                        "email".to_owned(),
                        serde_json::Value::String(user.email.clone()),
                    );
                    claims.insert(
                        "email_verified".to_owned(),
                        serde_json::Value::Bool(user.email_verified),
                    );
                }
                if let Some(resolver) = &options.custom_userinfo_claims {
                    claims.extend(
                        resolver
                            .resolve(CustomUserInfoClaimsInput {
                                user,
                                scopes: validated.scopes,
                                jwt: validated.claims,
                            })
                            .await?,
                    );
                }
                json_response(StatusCode::OK, &serde_json::Value::Object(claims))
            })
        },
    )
}
