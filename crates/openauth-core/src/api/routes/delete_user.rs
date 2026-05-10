use std::sync::Arc;

use http::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::shared::{
    current_session, error_response, json_response, query_param, request_cookie_header,
};
use crate::api::{
    create_auth_endpoint, parse_request_body, AsyncAuthEndpoint, AuthEndpointOptions, BodyField,
    BodySchema, JsonSchemaType, OpenApiOperation,
};
use crate::cookies::delete_session_cookie;
use crate::db::DbAdapter;
use crate::session::DbSessionStore;
use crate::user::DbUserStore;
use crate::verification::DbVerificationStore;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteUserBody {
    #[serde(default, alias = "callbackURL")]
    callback_url: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeleteUserResponse {
    success: bool,
    message: &'static str,
}

pub(super) fn delete_user_endpoint(adapter: Arc<dyn DbAdapter>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/delete-user",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("deleteUser")
            .body_schema(delete_user_body_schema())
            .openapi(delete_user_openapi("deleteUser", "Delete the user")),
        move |context, request| {
            let adapter = Arc::clone(&adapter);
            Box::pin(async move {
                if !context.options.user.delete_user.enabled {
                    return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Not found");
                }
                let body: DeleteUserBody = parse_request_body(&request)?;
                let _callback_url_seen = body.callback_url.as_deref();
                if let Some(token) = body.token.as_deref() {
                    return delete_user_by_token(adapter.as_ref(), context, &request, token).await;
                }
                let Some((_session, user, _cookies)) =
                    current_session(adapter.as_ref(), context, &request).await?
                else {
                    return super::shared::unauthorized();
                };
                if let Some(password) = body.password.as_deref() {
                    if !verify_delete_password(adapter.as_ref(), context, &user.id, password)
                        .await?
                    {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "INVALID_PASSWORD",
                            "Invalid password",
                        );
                    }
                } else if context.session_config.fresh_age != 0 {
                    let session = current_session(adapter.as_ref(), context, &request)
                        .await?
                        .map(|(session, _, _)| session);
                    let Some(session) = session else {
                        return super::shared::unauthorized();
                    };
                    let age = time::OffsetDateTime::now_utc() - session.created_at;
                    if age.whole_seconds() >= context.session_config.fresh_age as i64 {
                        return error_response(
                            StatusCode::BAD_REQUEST,
                            "SESSION_EXPIRED",
                            "Session expired",
                        );
                    }
                }
                delete_user_records(adapter.as_ref(), &user.id).await?;
                let cookies = delete_session_cookie(
                    &context.auth_cookies,
                    &request_cookie_header(&request).unwrap_or_default(),
                    false,
                );
                json_response(
                    StatusCode::OK,
                    &DeleteUserResponse {
                        success: true,
                        message: "User deleted",
                    },
                    cookies,
                )
            })
        },
    )
}

pub(super) fn delete_user_callback_endpoint(adapter: Arc<dyn DbAdapter>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/delete-user/callback",
        Method::GET,
        AuthEndpointOptions::new().openapi(delete_user_openapi(
            "deleteUserCallback",
            "Callback to complete user deletion with verification token",
        )),
        move |context, request| {
            let adapter = Arc::clone(&adapter);
            Box::pin(async move {
                if !context.options.user.delete_user.enabled {
                    return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Not found");
                }
                let Some(token) = query_param(&request, "token") else {
                    return invalid_token();
                };
                delete_user_by_token(adapter.as_ref(), context, &request, &token).await
            })
        },
    )
}

async fn delete_user_by_token(
    adapter: &dyn DbAdapter,
    context: &crate::context::AuthContext,
    request: &crate::api::ApiRequest,
    token: &str,
) -> Result<crate::api::ApiResponse, crate::error::OpenAuthError> {
    let Some((_, user, _)) = current_session(adapter, context, request).await? else {
        return super::shared::unauthorized();
    };
    let identifier = format!("delete-account-{token}");
    let verifications = DbVerificationStore::new(adapter);
    let Some(verification) = verifications.find_verification(&identifier).await? else {
        return invalid_token();
    };
    if verification.value != user.id {
        return invalid_token();
    }
    delete_user_records(adapter, &user.id).await?;
    verifications.delete_verification(&identifier).await?;
    let cookies = delete_session_cookie(
        &context.auth_cookies,
        &request_cookie_header(request).unwrap_or_default(),
        false,
    );
    json_response(
        StatusCode::OK,
        &DeleteUserResponse {
            success: true,
            message: "User deleted",
        },
        cookies,
    )
}

async fn verify_delete_password(
    adapter: &dyn DbAdapter,
    context: &crate::context::AuthContext,
    user_id: &str,
    password: &str,
) -> Result<bool, crate::error::OpenAuthError> {
    let Some(account) = DbUserStore::new(adapter)
        .find_credential_account(user_id)
        .await?
    else {
        return Ok(false);
    };
    let Some(password_hash) = account.password.as_deref() else {
        return Ok(false);
    };
    (context.password.verify)(password_hash, password)
}

async fn delete_user_records(
    adapter: &dyn DbAdapter,
    user_id: &str,
) -> Result<(), crate::error::OpenAuthError> {
    let users = DbUserStore::new(adapter);
    users.delete_user_accounts(user_id).await?;
    DbSessionStore::new(adapter)
        .delete_user_sessions(user_id)
        .await?;
    users.delete_user(user_id).await
}

fn invalid_token() -> Result<crate::api::ApiResponse, crate::error::OpenAuthError> {
    error_response(StatusCode::NOT_FOUND, "INVALID_TOKEN", "Invalid token")
}

fn delete_user_body_schema() -> BodySchema {
    BodySchema::object([
        BodyField::optional("callbackURL", JsonSchemaType::String)
            .description("The callback URL to redirect to after the user is deleted"),
        BodyField::optional("password", JsonSchemaType::String)
            .description("The user's password. Required if session is not fresh"),
        BodyField::optional("token", JsonSchemaType::String)
            .description("The deletion verification token"),
    ])
}

fn delete_user_openapi(operation_id: &str, description: &str) -> OpenApiOperation {
    OpenApiOperation::new(operation_id)
        .description(description)
        .response(
            "200",
            super::shared::json_openapi_response(
                "User deletion processed successfully",
                json!({
                    "type": "object",
                    "properties": {
                        "success": { "type": "boolean" },
                        "message": { "type": "string" },
                    },
                    "required": ["success", "message"],
                }),
            ),
        )
}
