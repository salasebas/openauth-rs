use crate::context::{AuthContext, SecretMaterial};
use crate::crypto::{symmetric_decrypt, symmetric_encrypt};
use crate::error::OpenAuthError;

pub fn set_token_util(
    token: Option<&str>,
    context: &AuthContext,
) -> Result<Option<String>, OpenAuthError> {
    let Some(token) = token else {
        return Ok(None);
    };
    if context.options.account.encrypt_oauth_tokens {
        encrypt_with_context(token, context).map(Some)
    } else {
        Ok(Some(token.to_owned()))
    }
}

pub fn decrypt_oauth_token(token: &str, context: &AuthContext) -> Result<String, OpenAuthError> {
    if token.is_empty() || !context.options.account.encrypt_oauth_tokens {
        return Ok(token.to_owned());
    }
    if !is_likely_encrypted(token) {
        return Ok(token.to_owned());
    }
    decrypt_with_context(token, context)
}

pub(crate) fn encrypt_with_context(
    data: &str,
    context: &AuthContext,
) -> Result<String, OpenAuthError> {
    match &context.secret_config {
        SecretMaterial::Single(secret) => symmetric_encrypt(secret.as_str(), data),
        SecretMaterial::Rotating(config) => symmetric_encrypt(config, data),
    }
}

pub(crate) fn decrypt_with_context(
    data: &str,
    context: &AuthContext,
) -> Result<String, OpenAuthError> {
    match &context.secret_config {
        SecretMaterial::Single(secret) => symmetric_decrypt(secret.as_str(), data),
        SecretMaterial::Rotating(config) => symmetric_decrypt(config, data),
    }
}

fn is_likely_encrypted(token: &str) -> bool {
    token.starts_with("$ba$")
        || (token.len() % 2 == 0 && token.chars().all(|character| character.is_ascii_hexdigit()))
}
