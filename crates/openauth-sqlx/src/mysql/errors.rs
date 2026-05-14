use openauth_core::error::OpenAuthError;

pub(super) fn inactive_transaction() -> OpenAuthError {
    OpenAuthError::Adapter("mysql transaction is no longer active".to_owned())
}

pub(super) fn sql_error(error: sqlx::Error) -> OpenAuthError {
    OpenAuthError::Adapter(error.to_string())
}

pub(super) fn argument_error(error: Box<dyn std::error::Error + Send + Sync>) -> OpenAuthError {
    OpenAuthError::Adapter(error.to_string())
}

pub(super) fn json_error(error: serde_json::Error) -> OpenAuthError {
    OpenAuthError::Adapter(error.to_string())
}
