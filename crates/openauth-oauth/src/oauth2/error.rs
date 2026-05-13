use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("missing OAuth provider option `{0}`")]
    MissingOption(&'static str),
    #[error("invalid OAuth URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("OAuth HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid OAuth response: {0}")]
    InvalidResponse(String),
    #[error("token verification failed: {0}")]
    TokenVerification(String),
    #[error("JOSE operation failed: {0}")]
    Jose(String),
}

impl From<josekit::JoseError> for OAuthError {
    fn from(error: josekit::JoseError) -> Self {
        Self::Jose(error.to_string())
    }
}
