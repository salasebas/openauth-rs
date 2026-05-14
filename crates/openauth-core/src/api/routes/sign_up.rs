use std::sync::Arc;

use http::{Method, StatusCode};
use serde::{Deserialize, Serialize};

use super::shared::{
    auth_flow_error_response, auth_session_cookies, email_password_config, json_response,
    message_openapi_response, password_validation_rejection_response,
    sign_up_email_openapi_response, RequestMetadata,
};
use crate::api::plugin_pipeline::run_password_validators;
use crate::api::{
    create_auth_endpoint, parse_request_body, AsyncAuthEndpoint, AuthEndpointOptions, BodyField,
    BodySchema, JsonSchemaType, OpenApiOperation,
};
use crate::auth::email_password::{EmailPasswordAuth, SignUpInput};
use crate::db::{DbAdapter, User};

#[derive(Debug, Deserialize)]
struct SignUpEmailBody {
    name: String,
    email: String,
    password: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default, alias = "rememberMe")]
    remember_me: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AuthTokenUserBody {
    token: String,
    user: User,
}

pub(super) fn sign_up_email_endpoint(adapter: Arc<dyn DbAdapter>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/sign-up/email",
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id("signUpWithEmailAndPassword")
            .allowed_media_types(["application/x-www-form-urlencoded", "application/json"])
            .body_schema(sign_up_email_body_schema())
            .openapi(
                OpenApiOperation::new("signUpWithEmailAndPassword")
                    .description("Sign up a user using email and password")
                    .response("200", sign_up_email_openapi_response())
                    .response(
                        "422",
                        message_openapi_response(
                            "Unprocessable Entity. User already exists or failed to create user.",
                        ),
                    ),
            ),
        move |context, request| {
            let adapter = Arc::clone(&adapter);
            Box::pin(async move {
                let body: SignUpEmailBody = parse_request_body(&request)?;
                let remember_me = body.remember_me.unwrap_or(true);
                let mut input =
                    SignUpInput::new(body.name, body.email, body.password).remember_me(remember_me);
                if let Some(image) = body.image {
                    input = input.image(image);
                }
                input = input.with_request_metadata(&request);
                if let Err(rejection) =
                    run_password_validators(context, "/sign-up/email", &input.password).await
                {
                    return password_validation_rejection_response(rejection);
                }

                let auth = EmailPasswordAuth::new(
                    adapter.as_ref(),
                    email_password_config(context),
                    context.password.hash,
                    context.password.verify,
                );
                let result = match auth.sign_up(input).await {
                    Ok(result) => result,
                    Err(error) => return auth_flow_error_response(error),
                };
                let cookies =
                    auth_session_cookies(context, &result.session, &result.user, !remember_me)?;
                json_response(
                    StatusCode::OK,
                    &AuthTokenUserBody {
                        token: result.session.token,
                        user: result.user,
                    },
                    cookies,
                )
            })
        },
    )
}

fn sign_up_email_body_schema() -> BodySchema {
    BodySchema::object([
        BodyField::new("name", JsonSchemaType::String).description("The name of the user"),
        BodyField::new("email", JsonSchemaType::String)
            .format("email")
            .description("The email of the user"),
        BodyField::new("password", JsonSchemaType::String).description("The password of the user"),
        BodyField::optional("image", JsonSchemaType::String)
            .description("The profile image URL of the user"),
        BodyField::optional("callbackURL", JsonSchemaType::String)
            .description("The URL to use for email verification callback"),
        BodyField::optional("rememberMe", JsonSchemaType::Boolean)
            .description("If false, the session will not be remembered"),
    ])
}
