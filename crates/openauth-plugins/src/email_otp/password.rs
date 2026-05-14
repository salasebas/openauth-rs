use std::sync::Arc;

use http::StatusCode;
use openauth_core::api::{parse_request_body, ApiRequest};
use openauth_core::context::AuthContext;
use openauth_core::crypto::password::hash_password;
use openauth_core::db::DbAdapter;
use openauth_core::user::{CreateCredentialAccountInput, DbUserStore};
use openauth_core::verification::DbVerificationStore;
use serde::Deserialize;

use super::helpers::{resolve_otp, send_email, validated_email, verify_otp};
use super::otp;
use super::response;
use super::types::{EmailOtpOptions, EmailOtpType};

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

pub(super) fn request_password_reset<'a>(
    context: &'a AuthContext,
    request: ApiRequest,
    adapter: Arc<dyn DbAdapter>,
    options: Arc<EmailOtpOptions>,
) -> openauth_core::api::EndpointFuture<'a> {
    Box::pin(async move {
        let body: EmailBody = parse_request_body(&request)?;
        let email = match validated_email(&body.email)? {
            Ok(email) => email,
            Err(response) => return Ok(response),
        };
        let identifier = otp::identifier(EmailOtpType::ForgetPassword, &email);
        let otp = resolve_otp(
            adapter.as_ref(),
            &options,
            &context.secret,
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
    adapter: Arc<dyn DbAdapter>,
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
        if let Some(response) = verify_otp(
            adapter.as_ref(),
            &options,
            &context.secret,
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
