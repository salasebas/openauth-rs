use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

use super::error::OAuthError;

pub use super::tokens::{get_oauth2_tokens, get_primary_client_id};

pub fn generate_code_challenge(code_verifier: &str) -> Result<String, OAuthError> {
    let hash = Sha256::digest(code_verifier.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(hash))
}
