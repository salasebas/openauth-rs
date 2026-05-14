use std::sync::Arc;

use http::StatusCode;
use openauth_core::api::{parse_request_body, ApiRequest};
use openauth_core::context::AuthContext;
use openauth_core::cookies::{set_session_cookie, CookieOptions, SessionCookieOptions};
use openauth_core::crypto::password::hash_password;
use openauth_core::db::User;
use openauth_core::user::{CreateCredentialAccountInput, CreateUserInput, DbUserStore};
use openauth_core::verification::DbVerificationStore;
use serde::{Deserialize, Serialize};

use super::helpers::{
    adapter, authenticated_user, create_session, parse_type, resolve_otp, send_email,
    validated_email, verify_otp,
};
use super::otp;
use super::response;
use super::types::{EmailOtpOptions, EmailOtpType};

pub(super) const SEND_PATH: &str = "/email-otp/send-verification-otp";
pub(super) const CHECK_PATH: &str = "/email-otp/check-verification-otp";
pub(super) const VERIFY_EMAIL_PATH: &str = "/email-otp/verify-email";
pub(super) const SIGN_IN_PATH: &str = "/sign-in/email-otp";
pub(super) const RESET_PASSWORD_PATH: &str = "/email-otp/reset-password";
pub(super) const REQUEST_CHANGE_EMAIL_PATH: &str = "/email-otp/request-email-change";
pub(super) const CHANGE_EMAIL_PATH: &str = "/email-otp/change-email";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendOtpBody {
    email: String,
    #[serde(rename = "type")]
    otp_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckOtpBody {
    email: String,
    #[serde(rename = "type")]
    otp_type: String,
    otp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyEmailBody {
    email: String,
    otp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignInBody {
    email: String,
    otp: String,
    name: Option<String>,
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailBody {
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordBody {
    email: String,
    otp: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestEmailChangeBody {
    new_email: String,
    otp: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeEmailBody {
    new_email: String,
    otp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenUserResponse {
    token: String,
    user: User,
}

pub(super) fn send_otp<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: SendOtpBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let otp_type = match parse_type(&body.otp_type)? {
            Ok(otp_type) => otp_type,
            Err(response) => return Ok(response),
        };
        if otp_type == EmailOtpType::ChangeEmail {
            return response::error(
                StatusCode::BAD_REQUEST,
                "INVALID_OTP_TYPE",
                "Invalid OTP type",
            );
        }
        let adapter = adapter(context)?;
        let user_exists = DbUserStore::new(adapter.as_ref())
            .find_user_by_email(&email)
            .await?
            .is_some();
        let should_send = otp_type == EmailOtpType::SignIn && !options.disable_sign_up;
        let identifier = otp::identifier(otp_type, &email);
        let otp = resolve_otp(adapter.as_ref(), &options, &email, otp_type, &identifier).await?;

        if !user_exists && !should_send {
            DbVerificationStore::new(adapter.as_ref())
                .delete_verification(&identifier)
                .await?;
            return response::success();
        }
        if let Some(response) = send_email(&options, &email, otp, otp_type, Some(&request))? {
            return Ok(response);
        }
        response::success()
    })
}

pub(super) fn check_otp<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: CheckOtpBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let otp_type = match parse_type(&body.otp_type)? {
            Ok(otp_type) => otp_type,
            Err(response) => return Ok(response),
        };
        if otp_type == EmailOtpType::ChangeEmail {
            return response::error(
                StatusCode::BAD_REQUEST,
                "INVALID_OTP_TYPE",
                "Invalid OTP type",
            );
        }
        let adapter = adapter(context)?;
        if DbUserStore::new(adapter.as_ref())
            .find_user_by_email(&email)
            .await?
            .is_none()
        {
            return response::error(StatusCode::BAD_REQUEST, "USER_NOT_FOUND", "User not found");
        }
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &otp::identifier(otp_type, &email),
            &body.otp,
            false,
        )
        .await?
        {
            return Ok(response);
        }
        response::success()
    })
}

pub(super) fn verify_email<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: VerifyEmailBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let adapter = adapter(context)?;
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &otp::identifier(EmailOtpType::EmailVerification, &email),
            &body.otp,
            true,
        )
        .await?
        {
            return Ok(response);
        }
        let users = DbUserStore::new(adapter.as_ref());
        let Some(user) = users.find_user_by_email(&email).await? else {
            return response::error(StatusCode::BAD_REQUEST, "USER_NOT_FOUND", "User not found");
        };
        let user = users
            .update_user_email_verified(&user.id, true)
            .await?
            .unwrap_or(user);
        if context
            .options
            .email_verification
            .auto_sign_in_after_verification
        {
            let session = create_session(adapter.as_ref(), context, &user.id, &request).await?;
            let cookies = set_session_cookie(
                &context.auth_cookies,
                &context.secret,
                &session.token,
                SessionCookieOptions {
                    dont_remember: false,
                    overrides: CookieOptions::default(),
                },
            )?;
            return response::json(
                StatusCode::OK,
                &serde_json::json!({ "status": true, "token": session.token, "user": user }),
                cookies,
            );
        }
        response::json(
            StatusCode::OK,
            &serde_json::json!({ "status": true, "token": null, "user": user }),
            Vec::new(),
        )
    })
}

pub(super) fn sign_in<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: SignInBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let adapter = adapter(context)?;
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &otp::identifier(EmailOtpType::SignIn, &email),
            &body.otp,
            true,
        )
        .await?
        {
            return Ok(response);
        }
        let users = DbUserStore::new(adapter.as_ref());
        let user = if let Some(user) = users.find_user_by_email(&email).await? {
            if !user.email_verified {
                users
                    .update_user_email_verified(&user.id, true)
                    .await?
                    .unwrap_or(user)
            } else {
                user
            }
        } else {
            if options.disable_sign_up {
                return response::error(StatusCode::BAD_REQUEST, "INVALID_OTP", "Invalid OTP");
            }
            let mut input =
                CreateUserInput::new(body.name.unwrap_or_default(), &email).email_verified(true);
            if let Some(image) = body.image {
                input = input.image(image);
            }
            users.create_user(input).await?
        };
        let session = create_session(adapter.as_ref(), context, &user.id, &request).await?;
        let cookies = set_session_cookie(
            &context.auth_cookies,
            &context.secret,
            &session.token,
            SessionCookieOptions::default(),
        )?;
        response::json(
            StatusCode::OK,
            &TokenUserResponse {
                token: session.token,
                user,
            },
            cookies,
        )
    })
}

pub(super) fn request_password_reset<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: EmailBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let adapter = adapter(context)?;
        let identifier = otp::identifier(EmailOtpType::ForgetPassword, &email);
        let otp = resolve_otp(
            adapter.as_ref(),
            &options,
            &email,
            EmailOtpType::ForgetPassword,
            &identifier,
        )
        .await?;
        if DbUserStore::new(adapter.as_ref())
            .find_user_by_email(&email)
            .await?
            .is_none()
        {
            DbVerificationStore::new(adapter.as_ref())
                .delete_verification(&identifier)
                .await?;
            return response::success();
        }
        if let Some(response) = send_email(
            &options,
            &email,
            otp,
            EmailOtpType::ForgetPassword,
            Some(&request),
        )? {
            return Ok(response);
        }
        response::success()
    })
}

pub(super) fn reset_password<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: ResetPasswordBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        if body.password.len() < context.password.config.min_password_length {
            return response::error(
                StatusCode::BAD_REQUEST,
                "PASSWORD_TOO_SHORT",
                "Password too short",
            );
        }
        if body.password.len() > context.password.config.max_password_length {
            return response::error(
                StatusCode::BAD_REQUEST,
                "PASSWORD_TOO_LONG",
                "Password too long",
            );
        }
        let adapter = adapter(context)?;
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &otp::identifier(EmailOtpType::ForgetPassword, &email),
            &body.otp,
            true,
        )
        .await?
        {
            return Ok(response);
        }
        let users = DbUserStore::new(adapter.as_ref());
        let Some(user) = users.find_user_by_email(&email).await? else {
            return response::error(StatusCode::BAD_REQUEST, "USER_NOT_FOUND", "User not found");
        };
        let password_hash = hash_password(&body.password)?;
        if users.find_credential_account(&user.id).await?.is_some() {
            users
                .update_credential_password(&user.id, &password_hash)
                .await?;
        } else {
            users
                .create_credential_account(CreateCredentialAccountInput::new(
                    &user.id,
                    password_hash,
                ))
                .await?;
        }
        if !user.email_verified {
            users.update_user_email_verified(&user.id, true).await?;
        }
        response::success()
    })
}

pub(super) fn request_email_change<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        if !options.change_email.enabled {
            return response::error(
                StatusCode::BAD_REQUEST,
                "CHANGE_EMAIL_DISABLED",
                "Change email with OTP is disabled",
            );
        }
        let body: RequestEmailChangeBody = parse_request_body(&request)?;
        let new_email = match validated_email(&body.new_email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let adapter = adapter(context)?;
        let user = match authenticated_user(adapter.as_ref(), context, &request).await? {
            Ok(user) => user,
            Err(response) => return Ok(response),
        };
        let current_email = otp::normalize_email(&user.email);
        if new_email == current_email {
            return response::error(
                StatusCode::BAD_REQUEST,
                "EMAIL_IS_THE_SAME",
                "Email is the same",
            );
        }
        if options.change_email.verify_current_email {
            let Some(current_otp) = body.otp else {
                return response::error(
                    StatusCode::BAD_REQUEST,
                    "OTP_REQUIRED",
                    "OTP is required to verify current email",
                );
            };
            if let Some(response) = verify_otp(
                adapter.as_ref(),
                &options,
                &otp::identifier(EmailOtpType::EmailVerification, &current_email),
                &current_otp,
                true,
            )
            .await?
            {
                return Ok(response);
            }
        }
        let identifier = otp::change_email_identifier(&current_email, &new_email);
        let generated = resolve_otp(
            adapter.as_ref(),
            &options,
            &new_email,
            EmailOtpType::ChangeEmail,
            &identifier,
        )
        .await?;
        if DbUserStore::new(adapter.as_ref())
            .find_user_by_email(&new_email)
            .await?
            .is_some()
        {
            DbVerificationStore::new(adapter.as_ref())
                .delete_verification(&identifier)
                .await?;
            return response::success();
        }
        if let Some(response) = send_email(
            &options,
            &new_email,
            generated,
            EmailOtpType::ChangeEmail,
            Some(&request),
        )? {
            return Ok(response);
        }
        response::success()
    })
}

pub(super) fn change_email<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        if !options.change_email.enabled {
            return response::error(
                StatusCode::BAD_REQUEST,
                "CHANGE_EMAIL_DISABLED",
                "Change email with OTP is disabled",
            );
        }
        let body: ChangeEmailBody = parse_request_body(&request)?;
        let new_email = match validated_email(&body.new_email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let adapter = adapter(context)?;
        let user = match authenticated_user(adapter.as_ref(), context, &request).await? {
            Ok(user) => user,
            Err(response) => return Ok(response),
        };
        let current_email = otp::normalize_email(&user.email);
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &otp::change_email_identifier(&current_email, &new_email),
            &body.otp,
            true,
        )
        .await?
        {
            return Ok(response);
        }
        let updated = DbUserStore::new(adapter.as_ref())
            .update_user_email(&user.id, &new_email, true)
            .await?
            .unwrap_or(user);
        response::json(
            StatusCode::OK,
            &serde_json::json!({ "success": true, "user": updated }),
            Vec::new(),
        )
    })
}
