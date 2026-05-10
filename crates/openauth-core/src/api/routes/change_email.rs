use std::sync::Arc;

use http::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::email_verification::create_email_verification_token;
use super::shared::{auth_session_cookies, current_session, error_response, json_response};
use crate::api::{
    create_auth_endpoint, parse_request_body, AsyncAuthEndpoint, AuthEndpointOptions, BodyField,
    BodySchema, JsonSchemaType, OpenApiOperation,
};
use crate::db::{DbAdapter, User};
use crate::options::VerificationEmail;
use crate::user::DbUserStore;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeEmailBody {
    new_email: String,
    #[serde(default, alias = "callbackURL")]
    callback_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChangeEmailResponse {
    status: bool,
    message: &'static str,
    user: Option<User>,
}

pub(super) fn change_email_endpoint(adapter: Arc<dyn DbAdapter>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/change-email",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("changeEmail")
            .body_schema(change_email_body_schema())
            .openapi(
                OpenApiOperation::new("changeEmail")
                    .description("Change the current user's email")
                    .response(
                        "200",
                        super::shared::json_openapi_response(
                            "Email change request processed successfully",
                            json!({
                                "type": "object",
                                "properties": {
                                    "status": { "type": "boolean" },
                                    "message": { "type": "string", "nullable": true },
                                    "user": {
                                        "oneOf": [
                                            { "$ref": "#/components/schemas/User" },
                                            { "type": "null" }
                                        ],
                                    },
                                },
                                "required": ["status"],
                            }),
                        ),
                    ),
            ),
        move |context, request| {
            let adapter = Arc::clone(&adapter);
            Box::pin(async move {
                if !context.options.user.change_email.enabled {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "CHANGE_EMAIL_DISABLED",
                        "Change email is disabled",
                    );
                }
                let Some((session, user, _cookies)) =
                    current_session(adapter.as_ref(), context, &request).await?
                else {
                    return super::shared::unauthorized();
                };
                let body: ChangeEmailBody = parse_request_body(&request)?;
                let new_email = body.new_email.to_lowercase();
                if new_email == user.email {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "EMAIL_IS_SAME",
                        "Email is the same",
                    );
                }

                let users = DbUserStore::new(adapter.as_ref());
                if users.find_user_by_email(&new_email).await?.is_some() {
                    create_email_verification_token(context, &user.email, Some(&new_email), None)?;
                    return json_response(
                        StatusCode::OK,
                        &ChangeEmailResponse {
                            status: true,
                            message: "Verification email sent",
                            user: None,
                        },
                        Vec::new(),
                    );
                }

                if !user.email_verified
                    && context
                        .options
                        .user
                        .change_email
                        .update_email_without_verification
                {
                    let updated = users
                        .update_user_email(&user.id, &new_email, false)
                        .await?
                        .unwrap_or(user);
                    let cookies = auth_session_cookies(context, &session, &updated, false)?;
                    return json_response(
                        StatusCode::OK,
                        &ChangeEmailResponse {
                            status: true,
                            message: "Email updated",
                            user: Some(updated),
                        },
                        cookies,
                    );
                }

                let Some(sender) = context
                    .options
                    .email_verification
                    .send_verification_email
                    .clone()
                else {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "VERIFICATION_EMAIL_NOT_ENABLED",
                        "Verification email isn't enabled",
                    );
                };
                let token = create_email_verification_token(
                    context,
                    &user.email,
                    Some(&new_email),
                    Some("change-email-verification"),
                )?;
                let callback_url = body.callback_url.unwrap_or_else(|| "/".to_owned());
                let url = format!(
                    "{}/verify-email?token={token}&callbackURL={}",
                    context.base_url,
                    super::shared::percent_encode(&callback_url)
                );
                sender.send_verification_email(
                    VerificationEmail {
                        user: User {
                            email: new_email,
                            ..user
                        },
                        url,
                        token,
                    },
                    Some(&request),
                )?;
                json_response(
                    StatusCode::OK,
                    &ChangeEmailResponse {
                        status: true,
                        message: "Verification email sent",
                        user: None,
                    },
                    Vec::new(),
                )
            })
        },
    )
}

fn change_email_body_schema() -> BodySchema {
    BodySchema::object([
        BodyField::new("newEmail", JsonSchemaType::String)
            .description("The new email address to set"),
        BodyField::optional("callbackURL", JsonSchemaType::String)
            .description("The URL to redirect to after email verification"),
    ])
}
