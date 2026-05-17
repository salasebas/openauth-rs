use std::sync::Arc;

use http::{header, Method, StatusCode};
use openauth_core::api::{
    create_auth_endpoint, parse_request_body, ApiRequest, AsyncAuthEndpoint, AuthEndpointOptions,
    OpenApiOperation,
};
use openauth_core::context::AuthContext;
use openauth_core::cookies::{set_session_cookie, SessionCookieOptions};
use openauth_core::db::DbAdapter;
use openauth_core::error::OpenAuthError;
use openauth_core::user::DbUserStore;
use openauth_core::verification::DbVerificationStore;
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::challenge::{create_challenge, find_challenge, ChallengeKind, ChallengeValue};
use crate::cookies::{challenge_cookie, challenge_token};
use crate::openapi::{
    id_body_schema, json_openapi_response, passkey_openapi_schema, query_parameter,
    update_passkey_body_schema, verify_authentication_body_schema, verify_registration_body_schema,
    webauthn_options_schema,
};
use crate::options::{
    AfterAuthenticationVerificationInput, AfterRegistrationVerificationInput,
    AuthenticatorAttachment, PasskeyOptions, PasskeyRegistrationUser, RegistrationWebAuthnOptions,
};
use crate::response::{error_response, json_response, not_allowed, unauthorized};
use crate::session::{
    create_session_for_user, current_session, registration_user, RegistrationUserError,
};
use crate::store::PasskeyStore;
use crate::webauthn::WebAuthnConfig;

pub fn endpoints(options: Arc<PasskeyOptions>) -> Vec<AsyncAuthEndpoint> {
    vec![
        generate_register_options_endpoint(Arc::clone(&options)),
        generate_authenticate_options_endpoint(Arc::clone(&options)),
        verify_registration_endpoint(Arc::clone(&options)),
        verify_authentication_endpoint(Arc::clone(&options)),
        list_passkeys_endpoint(Arc::clone(&options)),
        delete_passkey_endpoint(Arc::clone(&options)),
        update_passkey_endpoint(options),
    ]
}

fn generate_register_options_endpoint(options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/generate-register-options",
        Method::GET,
        AuthEndpointOptions::new()
            .operation_id("generatePasskeyRegistrationOptions")
            .openapi(
                OpenApiOperation::new("generatePasskeyRegistrationOptions")
                    .tag("Passkey")
                    .description("Generate registration options for a new passkey")
                    .parameter(query_parameter(
                        "authenticatorAttachment",
                        "Optional authenticator attachment: platform or cross-platform",
                    ))
                    .parameter(query_parameter("name", "Optional custom passkey name"))
                    .parameter(query_parameter(
                        "context",
                        "Optional context for pre-auth registration flows",
                    ))
                    .response(
                        "200",
                        json_openapi_response("Success", webauthn_options_schema()),
                    ),
            ),
        move |context, request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                let adapter = adapter(context)?;
                let session = current_session(context, &request).await?;
                let user =
                    match registration_user(&options, session.as_ref(), query_param(&request, "context")) {
                    Ok(user) => user,
                    Err(RegistrationUserError::SessionRequired) => {
                        return error_response(
                            StatusCode::UNAUTHORIZED,
                            "SESSION_REQUIRED",
                            "Passkey registration requires an authenticated session",
                        )
                    }
                    Err(RegistrationUserError::ResolveUserRequired) => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "RESOLVE_USER_REQUIRED",
                            "Passkey registration requires either an authenticated session or a resolveUser callback when requireSession is false",
                        )
                    }
                    Err(RegistrationUserError::ResolvedUserInvalid) => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "RESOLVED_USER_INVALID",
                            "Resolved user is invalid",
                        )
                    }
                };
                let user_passkeys = PasskeyStore::new(adapter.as_ref())
                    .list_by_user(&user.id)
                    .await?;
                let mut webauthn_user = user.clone();
                if let Some(name) = query_param(&request, "name") {
                    if webauthn_user.display_name.is_none() {
                        webauthn_user.display_name = Some(user.name.clone());
                    }
                    webauthn_user.name = name;
                }
                let attachment = query_param(&request, "authenticatorAttachment")
                    .as_deref()
                    .and_then(AuthenticatorAttachment::from_query);
                let request_options = RegistrationWebAuthnOptions::new(
                    options
                        .authenticator_selection
                        .with_attachment_override(attachment),
                    options.registration.extensions.clone(),
                );
                let start = options.backend.start_registration(
                    webauthn_config(context, &options, &request)?,
                    &webauthn_user,
                    user_passkeys
                        .into_iter()
                        .filter_map(|passkey| {
                            (!passkey.webauthn_credential.is_null())
                                .then_some(passkey.webauthn_credential)
                        })
                        .collect(),
                    request_options,
                )?;
                let context_value = query_param(&request, "context");
                let token = create_challenge(
                    adapter.as_ref(),
                    ChallengeValue {
                        kind: ChallengeKind::Registration,
                        state: start.state,
                        user: Some(user),
                        context: context_value,
                    },
                )
                .await?;
                json_response(
                    StatusCode::OK,
                    &start.options,
                    vec![challenge_cookie(context, &options, token)?],
                )
            })
        },
    )
}

fn generate_authenticate_options_endpoint(options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/generate-authenticate-options",
        Method::GET,
        AuthEndpointOptions::new()
            .operation_id("passkeyGenerateAuthenticateOptions")
            .openapi(
                OpenApiOperation::new("passkeyGenerateAuthenticateOptions")
                    .tag("Passkey")
                    .description("Generate authentication options for a passkey")
                    .response(
                        "200",
                        json_openapi_response("Success", webauthn_options_schema()),
                    ),
            ),
        move |context, request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                let adapter = adapter(context)?;
                let session = current_session(context, &request).await?;
                let credentials = if let Some((_, user, _)) = &session {
                    PasskeyStore::new(adapter.as_ref())
                        .list_by_user(&user.id)
                        .await?
                        .into_iter()
                        .filter_map(|passkey| {
                            (!passkey.webauthn_credential.is_null())
                                .then_some(passkey.webauthn_credential)
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let start = options.backend.start_authentication(
                    webauthn_config(context, &options, &request)?,
                    credentials,
                    options.authentication.extensions.clone(),
                )?;
                let token = create_challenge(
                    adapter.as_ref(),
                    ChallengeValue {
                        kind: ChallengeKind::Authentication,
                        state: start.state,
                        user: session.map(|(_, user, _)| PasskeyRegistrationUser {
                            id: user.id,
                            name: user.email,
                            display_name: None,
                        }),
                        context: None,
                    },
                )
                .await?;
                json_response(
                    StatusCode::OK,
                    &start.options,
                    vec![challenge_cookie(context, &options, token)?],
                )
            })
        },
    )
}

fn verify_registration_endpoint(options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/verify-registration",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("passkeyVerifyRegistration")
            .allowed_media_types(["application/json"])
            .body_schema(verify_registration_body_schema())
            .openapi(
                OpenApiOperation::new("passkeyVerifyRegistration")
                    .tag("Passkey")
                    .description("Verify registration of a new passkey")
                    .response(
                        "200",
                        json_openapi_response("Success", passkey_openapi_schema()),
                    ),
            ),
        move |context, request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                let adapter = adapter(context)?;
                let body: VerifyRegistrationBody = parse_request_body(&request)?;
                let token = match challenge_token(context, &options, &request)? {
                    Some(token) => token,
                    None => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "CHALLENGE_NOT_FOUND",
                            "Challenge not found",
                        )
                    }
                };
                let Some(challenge) = find_challenge(adapter.as_ref(), &token).await? else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CHALLENGE_NOT_FOUND",
                        "Challenge not found",
                    );
                };
                if challenge.kind != ChallengeKind::Registration {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CHALLENGE_NOT_FOUND",
                        "Challenge not found",
                    );
                }
                let session = current_session(context, &request).await?;
                let Some(resolved_user) = challenge.user.clone() else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "RESOLVED_USER_INVALID",
                        "Resolved user is invalid",
                    );
                };
                if let Some((_, user, _)) = &session {
                    if user.id != resolved_user.id {
                        return not_allowed();
                    }
                }
                let verified = match options.backend.finish_registration(
                    webauthn_config(context, &options, &request)?,
                    body.response.clone(),
                    challenge.state,
                ) {
                    Ok(verified) => verified,
                    Err(_) => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "FAILED_TO_VERIFY_REGISTRATION",
                            "Failed to verify registration",
                        )
                    }
                };
                let mut target_user_id = resolved_user.id.clone();
                if let Some(callback) = &options.registration.after_verification {
                    if let Some(user_id) = callback(AfterRegistrationVerificationInput {
                        user: resolved_user.clone(),
                        client_data: body.response,
                        context: challenge.context,
                    }) {
                        if user_id.is_empty() {
                            return error_response(
                                StatusCode::BAD_REQUEST,
                                "RESOLVED_USER_INVALID",
                                "Resolved user is invalid",
                            );
                        }
                        if let Some((_, user, _)) = &session {
                            if user.id != user_id {
                                return not_allowed();
                            }
                        }
                        target_user_id = user_id;
                    }
                }
                let passkey = PasskeyStore::new(adapter.as_ref())
                    .create(&target_user_id, body.name, verified)
                    .await?;
                DbVerificationStore::new(adapter.as_ref())
                    .delete_verification(&token)
                    .await?;
                json_response(StatusCode::OK, &passkey, Vec::new())
            })
        },
    )
}

fn verify_authentication_endpoint(options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/verify-authentication",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("passkeyVerifyAuthentication")
            .allowed_media_types(["application/json"])
            .body_schema(verify_authentication_body_schema())
            .openapi(
                OpenApiOperation::new("passkeyVerifyAuthentication")
                    .tag("Passkey")
                    .description("Verify authentication of a passkey")
                    .response(
                        "200",
                        json_openapi_response(
                            "Success",
                            json!({
                                "type": "object",
                                "properties": {
                                    "session": { "$ref": "#/components/schemas/Session" },
                                    "user": { "$ref": "#/components/schemas/User" },
                                },
                                "required": ["session", "user"],
                            }),
                        ),
                    ),
            ),
        move |context, request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                let adapter = adapter(context)?;
                let body: VerifyAuthenticationBody = parse_request_body(&request)?;
                let token = match challenge_token(context, &options, &request)? {
                    Some(token) => token,
                    None => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "CHALLENGE_NOT_FOUND",
                            "Challenge not found",
                        )
                    }
                };
                let Some(challenge) = find_challenge(adapter.as_ref(), &token).await? else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CHALLENGE_NOT_FOUND",
                        "Challenge not found",
                    );
                };
                if challenge.kind != ChallengeKind::Authentication {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CHALLENGE_NOT_FOUND",
                        "Challenge not found",
                    );
                }
                let credential_id =
                    body.response
                        .get("id")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            OpenAuthError::Api("passkey response id is required".to_owned())
                        })?;
                let store = PasskeyStore::new(adapter.as_ref());
                let Some(passkey) = store.find_by_credential_id(credential_id).await? else {
                    return error_response(
                        StatusCode::UNAUTHORIZED,
                        "PASSKEY_NOT_FOUND",
                        "Passkey not found",
                    );
                };
                if challenge
                    .user
                    .as_ref()
                    .is_some_and(|user| user.id != passkey.user_id)
                {
                    return error_response(
                        StatusCode::UNAUTHORIZED,
                        "PASSKEY_NOT_FOUND",
                        "Passkey not found",
                    );
                }
                let verified = match options.backend.finish_authentication(
                    webauthn_config(context, &options, &request)?,
                    body.response.clone(),
                    challenge.state,
                    Some(passkey.webauthn_credential.clone()),
                ) {
                    Ok(verified) => verified,
                    Err(_) => {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "AUTHENTICATION_FAILED",
                            "Authentication failed",
                        )
                    }
                };
                if let Some(callback) = &options.authentication.after_verification {
                    callback(AfterAuthenticationVerificationInput {
                        credential_id: passkey.credential_id.clone(),
                        client_data: body.response,
                    });
                }
                let _ = store
                    .update_after_authentication(&passkey.id, verified)
                    .await?;
                let user = DbUserStore::new(adapter.as_ref())
                    .find_user_by_id(&passkey.user_id)
                    .await?
                    .ok_or_else(|| OpenAuthError::Adapter("user not found".to_owned()))?;
                let session =
                    create_session_for_user(adapter.as_ref(), context, &request, &user).await?;
                DbVerificationStore::new(adapter.as_ref())
                    .delete_verification(&token)
                    .await?;
                let cookies = set_session_cookie(
                    &context.auth_cookies,
                    &context.secret,
                    &session.token,
                    SessionCookieOptions::default(),
                )?;
                json_response(
                    StatusCode::OK,
                    &json!({ "session": session, "user": user }),
                    cookies,
                )
            })
        },
    )
}

fn list_passkeys_endpoint(_options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/list-user-passkeys",
        Method::GET,
        AuthEndpointOptions::new().openapi(
            OpenApiOperation::new("listPasskeys")
                .tag("Passkey")
                .description("List all passkeys for the authenticated user")
                .response(
                    "200",
                    json_openapi_response(
                        "Passkeys retrieved successfully",
                        json!({
                            "type": "array",
                            "items": passkey_openapi_schema(),
                        }),
                    ),
                ),
        ),
        move |context, request| {
            Box::pin(async move {
                let adapter = adapter(context)?;
                let Some((_, user, cookies)) = current_session(context, &request).await? else {
                    return unauthorized();
                };
                let passkeys = PasskeyStore::new(adapter.as_ref())
                    .list_by_user(&user.id)
                    .await?;
                json_response(StatusCode::OK, &passkeys, cookies)
            })
        },
    )
}

fn delete_passkey_endpoint(_options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/delete-passkey",
        Method::POST,
        AuthEndpointOptions::new()
            .allowed_media_types(["application/json"])
            .body_schema(id_body_schema())
            .openapi(
                OpenApiOperation::new("deletePasskey")
                    .tag("Passkey")
                    .description("Delete a specific passkey")
                    .response(
                        "200",
                        json_openapi_response(
                            "Passkey deleted successfully",
                            json!({
                                "type": "object",
                                "properties": {
                                    "status": { "type": "boolean" },
                                },
                                "required": ["status"],
                            }),
                        ),
                    ),
            ),
        move |context, request| {
            Box::pin(async move {
                let adapter = adapter(context)?;
                let body: IdBody = parse_request_body(&request)?;
                let Some((_, user, cookies)) = current_session(context, &request).await? else {
                    return unauthorized();
                };
                let store = PasskeyStore::new(adapter.as_ref());
                let Some(passkey) = store.find_by_id(&body.id).await? else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "PASSKEY_NOT_FOUND",
                        "Passkey not found",
                    );
                };
                if passkey.user_id != user.id {
                    return not_allowed();
                }
                store.delete_for_user(&body.id, &user.id).await?;
                json_response(StatusCode::OK, &json!({ "status": true }), cookies)
            })
        },
    )
}

fn update_passkey_endpoint(_options: Arc<PasskeyOptions>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/passkey/update-passkey",
        Method::POST,
        AuthEndpointOptions::new()
            .allowed_media_types(["application/json"])
            .body_schema(update_passkey_body_schema())
            .openapi(
                OpenApiOperation::new("updatePasskey")
                    .tag("Passkey")
                    .description("Update a specific passkey name")
                    .response(
                        "200",
                        json_openapi_response(
                            "Passkey updated successfully",
                            json!({
                                "type": "object",
                                "properties": {
                                    "passkey": passkey_openapi_schema(),
                                },
                                "required": ["passkey"],
                            }),
                        ),
                    ),
            ),
        move |context, request| {
            Box::pin(async move {
                let adapter = adapter(context)?;
                let body: UpdatePasskeyBody = parse_request_body(&request)?;
                let Some((_, user, cookies)) = current_session(context, &request).await? else {
                    return unauthorized();
                };
                let store = PasskeyStore::new(adapter.as_ref());
                let Some(existing) = store.find_by_id(&body.id).await? else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "PASSKEY_NOT_FOUND",
                        "Passkey not found",
                    );
                };
                if existing.user_id != user.id {
                    return not_allowed();
                }
                let Some(passkey) = store
                    .update_name_for_user(&body.id, &user.id, body.name)
                    .await?
                else {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "FAILED_TO_UPDATE_PASSKEY",
                        "Failed to update passkey",
                    );
                };
                json_response(StatusCode::OK, &json!({ "passkey": passkey }), cookies)
            })
        },
    )
}

#[derive(Debug, Deserialize)]
struct VerifyRegistrationBody {
    response: Value,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyAuthenticationBody {
    response: Value,
}

#[derive(Debug, Deserialize)]
struct IdBody {
    id: String,
}

#[derive(Debug, Deserialize)]
struct UpdatePasskeyBody {
    id: String,
    name: String,
}

fn adapter(context: &AuthContext) -> Result<Arc<dyn DbAdapter>, OpenAuthError> {
    context.adapter().ok_or_else(|| {
        OpenAuthError::InvalidConfig("passkey requires a database adapter".to_owned())
    })
}

fn webauthn_config(
    context: &AuthContext,
    options: &PasskeyOptions,
    request: &ApiRequest,
) -> Result<WebAuthnConfig, OpenAuthError> {
    let origins = if options.origin.is_empty() {
        request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            .map(|origin| vec![origin.trim_end_matches('/').to_owned()])
            .or_else(|| (!context.base_url.is_empty()).then(|| vec![context.base_url.clone()]))
            .unwrap_or_else(|| vec!["http://localhost".to_owned()])
    } else {
        options.origin.clone()
    };
    let rp_id = options
        .rp_id
        .clone()
        .or_else(|| host_from_url(context.base_url.as_str()))
        .or_else(|| origins.first().and_then(|origin| host_from_url(origin)))
        .unwrap_or_else(|| "localhost".to_owned());
    Ok(WebAuthnConfig {
        rp_id,
        rp_name: options
            .rp_name
            .clone()
            .unwrap_or_else(|| context.app_name.clone()),
        origins,
    })
}

fn host_from_url(value: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
}

fn query_param(request: &ApiRequest, name: &str) -> Option<String> {
    request.uri().query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
    })
}
