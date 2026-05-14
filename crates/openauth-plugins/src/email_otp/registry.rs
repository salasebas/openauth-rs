use std::sync::Arc;

use http::Method;
use openauth_core::api::{
    create_auth_endpoint, ApiRequest, AsyncAuthEndpoint, AuthEndpointOptions,
};
use openauth_core::context::AuthContext;
use openauth_core::plugin::AuthPlugin;

use super::endpoints::{
    change_email, check_otp, request_email_change, request_password_reset, reset_password,
    send_otp, sign_in, verify_email, CHANGE_EMAIL_PATH, CHECK_PATH, REQUEST_CHANGE_EMAIL_PATH,
    RESET_PASSWORD_PATH, SEND_PATH, SIGN_IN_PATH, VERIFY_EMAIL_PATH,
};
use super::schema::common_schema;
use super::types::EmailOtpOptions;

const REQUEST_RESET_PATH: &str = "/email-otp/request-password-reset";
const FORGET_PASSWORD_PATH: &str = "/forget-password/email-otp";

type Handler =
    fn(&AuthContext, ApiRequest, Arc<EmailOtpOptions>) -> openauth_core::api::EndpointFuture<'_>;

pub fn paths() -> &'static [&'static str] {
    &[
        SEND_PATH,
        CHECK_PATH,
        VERIFY_EMAIL_PATH,
        SIGN_IN_PATH,
        REQUEST_RESET_PATH,
        FORGET_PASSWORD_PATH,
        RESET_PASSWORD_PATH,
        REQUEST_CHANGE_EMAIL_PATH,
        CHANGE_EMAIL_PATH,
    ]
}

pub fn register(plugin: AuthPlugin, options: EmailOtpOptions) -> AuthPlugin {
    let options = Arc::new(options);
    plugin
        .with_endpoint(endpoint(
            SEND_PATH,
            "sendEmailVerificationOTP",
            options.clone(),
            send_otp,
        ))
        .with_endpoint(endpoint(
            CHECK_PATH,
            "checkEmailVerificationOTP",
            options.clone(),
            check_otp,
        ))
        .with_endpoint(endpoint(
            VERIFY_EMAIL_PATH,
            "verifyEmailOTP",
            options.clone(),
            verify_email,
        ))
        .with_endpoint(endpoint(
            SIGN_IN_PATH,
            "signInEmailOTP",
            options.clone(),
            sign_in,
        ))
        .with_endpoint(endpoint(
            REQUEST_RESET_PATH,
            "requestPasswordResetEmailOTP",
            options.clone(),
            request_password_reset,
        ))
        .with_endpoint(endpoint(
            FORGET_PASSWORD_PATH,
            "forgetPasswordEmailOTP",
            options.clone(),
            request_password_reset,
        ))
        .with_endpoint(endpoint(
            RESET_PASSWORD_PATH,
            "resetPasswordEmailOTP",
            options.clone(),
            reset_password,
        ))
        .with_endpoint(endpoint(
            REQUEST_CHANGE_EMAIL_PATH,
            "requestEmailChangeEmailOTP",
            options.clone(),
            request_email_change,
        ))
        .with_endpoint(endpoint(
            CHANGE_EMAIL_PATH,
            "changeEmailEmailOTP",
            options,
            change_email,
        ))
}

fn endpoint(
    path: &'static str,
    operation_id: &'static str,
    options: Arc<EmailOtpOptions>,
    handler: Handler,
) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        path,
        Method::POST,
        AuthEndpointOptions::new()
            .operation_id(operation_id)
            .body_schema(common_schema(path)),
        move |context, request| handler(context, request, options.clone()),
    )
}
