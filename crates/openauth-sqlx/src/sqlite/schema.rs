use super::errors::{inactive_transaction, sql_error};
use super::state::SqliteExecutor;
use super::support::{quote_identifier, sanitize_identifier};
use openauth_core::db::{DbField, DbFieldType, DbSchema, OnDelete};
use openauth_core::error::OpenAuthError;

pub(super) async fn create_schema(
    mut executor: SqliteExecutor<'_, '_>,
    schema: &DbSchema,
) -> Result<(), OpenAuthError> {
    let mut tables = schema.tables().collect::<Vec<_>>();
    tables.sort_by_key(|(_, table)| table.order.unwrap_or(u16::MAX));

    for (_, table) in &tables {
        let mut columns = Vec::new();
        for (logical_name, field) in &table.fields {
            columns.push(column_definition(logical_name, field)?);
        }
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            quote_identifier(&table.name)?,
            columns.join(", ")
        );
        execute_schema_sql(&mut executor, &sql).await?;
    }

    for (_, table) in tables {
        for (logical_name, field) in &table.fields {
            if field.index && !field.unique {
                let index_name = format!("idx_{}_{}", table.name, logical_name);
                let sql = format!(
                    "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
                    quote_identifier(&sanitize_identifier(&index_name)?)?,
                    quote_identifier(&table.name)?,
                    quote_identifier(&field.name)?,
                );
                execute_schema_sql(&mut executor, &sql).await?;
            }
        }
    }

    Ok(())
}

pub(super) async fn execute_schema_sql(
    executor: &mut SqliteExecutor<'_, '_>,
    sql: &str,
) -> Result<(), OpenAuthError> {
    match executor {
        SqliteExecutor::Pool(pool) => {
            sqlx::query(sql).execute(*pool).await.map_err(sql_error)?;
        }
        SqliteExecutor::Transaction(tx) => {
            let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
            sqlx::query(sql)
                .execute(&mut **tx)
                .await
                .map_err(sql_error)?;
        }
    }
    Ok(())
}

pub(super) fn column_definition(
    logical_name: &str,
    field: &DbField,
) -> Result<String, OpenAuthError> {
    let mut parts = vec![quote_identifier(&field.name)?, sqlite_type(field)];
    if logical_name == "id" || field.name == "id" {
        parts.push("PRIMARY KEY".to_owned());
    } else {
        if field.required {
            parts.push("NOT NULL".to_owned());
        }
        if field.unique {
            parts.push("UNIQUE".to_owned());
        }
    }
    if let Some(foreign_key) = &field.foreign_key {
        parts.push(format!(
            "REFERENCES {} ({})",
            quote_identifier(&foreign_key.table)?,
            quote_identifier(&foreign_key.field)?
        ));
        parts.push(on_delete_sql(foreign_key.on_delete).to_owned());
    }
    Ok(parts.join(" "))
}

pub(super) fn sqlite_type(field: &DbField) -> String {
    match field.field_type {
        DbFieldType::String
        | DbFieldType::Timestamp
        | DbFieldType::Json
        | DbFieldType::StringArray
        | DbFieldType::NumberArray => "TEXT".to_owned(),
        DbFieldType::Number | DbFieldType::Boolean => "INTEGER".to_owned(),
    }
}

pub(super) fn on_delete_sql(on_delete: OnDelete) -> &'static str {
    match on_delete {
        OnDelete::NoAction => "ON DELETE NO ACTION",
        OnDelete::Restrict => "ON DELETE RESTRICT",
        OnDelete::Cascade => "ON DELETE CASCADE",
        OnDelete::SetNull => "ON DELETE SET NULL",
        OnDelete::SetDefault => "ON DELETE SET DEFAULT",
    }
}
