use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use openauth_core::crypto::buffer::constant_time_equal;
use openauth_core::error::OpenAuthError;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

use super::types::{EmailOtpOptions, EmailOtpType, OtpStorage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOtpParts {
    pub value: String,
    pub attempts: u32,
}

pub fn identifier(otp_type: EmailOtpType, email: &str) -> String {
    format!("{}-otp-{}", otp_type.as_str(), normalize_email(email))
}

pub fn change_email_identifier(current_email: &str, new_email: &str) -> String {
    identifier(
        EmailOtpType::ChangeEmail,
        &format!(
            "{}-{}",
            normalize_email(current_email),
            normalize_email(new_email)
        ),
    )
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn valid_email(email: &str) -> bool {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

pub fn generate(options: &EmailOtpOptions, email: &str, otp_type: EmailOtpType) -> String {
    if let Some(generator) = &options.generator {
        return generator.generate_otp(email, otp_type, options.otp_length);
    }
    generate_numeric(options.otp_length)
}

pub fn store(options: &EmailOtpOptions, otp: &str) -> String {
    match options.store_otp {
        OtpStorage::Plain => otp.to_owned(),
        OtpStorage::Hashed => hash_otp(otp),
    }
}

pub fn verify(options: &EmailOtpOptions, stored: &str, provided: &str) -> bool {
    match options.store_otp {
        OtpStorage::Plain => constant_time_equal(stored.as_bytes(), provided.as_bytes()),
        OtpStorage::Hashed => constant_time_equal(hash_otp(provided).as_bytes(), stored.as_bytes()),
    }
}

pub fn reusable_plain_otp(options: &EmailOtpOptions, parts: &StoredOtpParts) -> Option<String> {
    (options.store_otp == OtpStorage::Plain).then(|| parts.value.clone())
}

pub fn encode_value(stored_otp: &str, attempts: u32) -> String {
    format!("{stored_otp}:{attempts}")
}

pub fn split_value(value: &str) -> StoredOtpParts {
    let Some((otp, attempts)) = value.rsplit_once(':') else {
        return StoredOtpParts {
            value: value.to_owned(),
            attempts: 0,
        };
    };
    StoredOtpParts {
        value: otp.to_owned(),
        attempts: attempts.parse().unwrap_or(0),
    }
}

pub fn seconds_to_duration(seconds: u64) -> Result<time::Duration, OpenAuthError> {
    let seconds = i64::try_from(seconds)
        .map_err(|_| OpenAuthError::InvalidConfig("email OTP expiry is too large".to_owned()))?;
    Ok(time::Duration::seconds(seconds))
}

fn generate_numeric(length: usize) -> String {
    let mut output = String::with_capacity(length);
    let mut random = vec![0_u8; length];
    OsRng.fill_bytes(&mut random);
    for byte in random {
        output.push(char::from(b'0' + (byte % 10)));
    }
    output
}

fn hash_otp(otp: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(otp.as_bytes()))
}
