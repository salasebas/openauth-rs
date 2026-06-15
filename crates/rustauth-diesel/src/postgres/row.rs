use diesel::deserialize::QueryableByName;
use diesel::pg::Pg;
use diesel::row::{Field, NamedRow, Row};
use indexmap::IndexMap;
use rustauth_core::db::{DbField, DbFieldType, DbValue, IdGeneration};
use rustauth_core::error::RustAuthError;
use time::OffsetDateTime;
use tokio_postgres::types::{FromSql, Type};

#[derive(Debug, Clone)]
pub(super) struct StoredColumn {
    bytes: Option<Vec<u8>>,
    type_oid: u32,
}

/// Dynamic Postgres row captured from any `sql_query` result.
#[derive(Debug, Clone, Default)]
pub(super) struct DieselPostgresRow {
    columns: IndexMap<String, StoredColumn>,
}

impl QueryableByName<Pg> for DieselPostgresRow {
    fn build<'a>(row: &impl NamedRow<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let mut columns = IndexMap::new();
        for index in 0..Row::field_count(row) {
            let field = Row::get(row, index)
                .ok_or_else(|| "missing diesel postgres row field".to_string())?;
            let name = field
                .field_name()
                .ok_or_else(|| "diesel postgres row field has no name".to_string())?
                .to_owned();
            let (bytes, type_oid) = match field.value() {
                Some(value) => {
                    let pg_value: diesel::pg::PgValue<'_> = value;
                    (Some(pg_value.as_bytes().to_vec()), pg_value.get_oid().get())
                }
                None => (None, 0),
            };
            columns.insert(name, StoredColumn { bytes, type_oid });
        }
        Ok(Self { columns })
    }
}

pub(super) fn row_value_at(
    row: &DieselPostgresRow,
    field: &DbField,
    column: &str,
) -> Result<DbValue, RustAuthError> {
    let stored = row.columns.get(column).ok_or_else(|| {
        RustAuthError::Adapter(format!("diesel postgres row missing column `{column}`"))
    })?;
    decode_field(field, stored)
}

fn decode_field(field: &DbField, stored: &StoredColumn) -> Result<DbValue, RustAuthError> {
    let Some(bytes) = stored.bytes.as_ref() else {
        return Ok(DbValue::Null);
    };
    let ty = Type::from_oid(stored.type_oid).unwrap_or(Type::TEXT);
    match field.field_type {
        DbFieldType::String if field.generated_id == Some(IdGeneration::Uuid) => {
            let value = uuid::Uuid::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::String(value.to_string()))
        }
        DbFieldType::String => {
            let value = String::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::String(value))
        }
        DbFieldType::Number => {
            let value = i64::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::Number(value))
        }
        DbFieldType::Boolean => {
            let value = bool::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::Boolean(value))
        }
        DbFieldType::Timestamp => {
            let value = OffsetDateTime::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::Timestamp(value))
        }
        DbFieldType::Json => {
            let value = serde_json::Value::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::Json(value))
        }
        DbFieldType::StringArray => {
            let value = Vec::<String>::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::StringArray(value))
        }
        DbFieldType::NumberArray => {
            let value = Vec::<i64>::from_sql(&ty, bytes).map_err(decode_error)?;
            Ok(DbValue::NumberArray(value))
        }
    }
}

fn decode_error(error: Box<dyn std::error::Error + Send + Sync>) -> RustAuthError {
    RustAuthError::Adapter(format!("diesel postgres row decode: {error}"))
}
