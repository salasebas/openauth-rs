use rustauth_core::db::{DbField, DbFieldType, DbValue, IdGeneration, SqlParam, SqlSelectedField};
use rustauth_core::env::env_var;
use rustauth_core::error::RustAuthError;
use time::OffsetDateTime;

pub const DEFAULT_POSTGRES_URL: &str = "postgres://user:password@localhost:5432/rustauth";

#[cfg(feature = "mysql")]
pub const DEFAULT_MYSQL_URL: &str = "mysql://user:password@localhost:3306/rustauth";

#[cfg(feature = "postgres")]
pub fn postgres_database_url() -> String {
    env_var("TEST_POSTGRES_URL")
        .or_else(|| env_var("RUSTAUTH_TEST_POSTGRES_URL"))
        .unwrap_or_else(|| DEFAULT_POSTGRES_URL.to_owned())
}

#[cfg(feature = "mysql")]
pub fn mysql_database_url() -> String {
    env_var("TEST_MYSQL_URL")
        .or_else(|| env_var("RUSTAUTH_TEST_MYSQL_URL"))
        .unwrap_or_else(|| DEFAULT_MYSQL_URL.to_owned())
}

pub fn field(name: &str, field_type: DbFieldType) -> DbField {
    DbField {
        name: name.to_owned(),
        field_type,
        required: false,
        unique: false,
        index: false,
        returned: true,
        input: true,
        foreign_key: None,
        generated_id: None,
    }
}

#[cfg(feature = "postgres")]
pub fn uuid_field(name: &str) -> DbField {
    DbField {
        generated_id: Some(IdGeneration::Uuid),
        ..field(name, DbFieldType::String)
    }
}

pub fn selected(logical_name: &str, field: DbField, alias: &str) -> SqlSelectedField {
    SqlSelectedField {
        logical_name: logical_name.to_owned(),
        field,
        alias: alias.to_owned(),
    }
}

pub fn param(field: &DbField, value: DbValue) -> SqlParam {
    SqlParam::new(field, value)
}

pub fn sample_timestamp() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

pub fn adapter_error(
    context: &str,
    database_url: &str,
    error: impl std::fmt::Display,
) -> RustAuthError {
    RustAuthError::Adapter(format!(
        "{context} test database preflight failed for `{database_url}`: {error}. \
         Start services with `./scripts/ensure-test-services.sh` and override with \
         TEST_POSTGRES_URL / TEST_MYSQL_URL when needed."
    ))
}
