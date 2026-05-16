use super::errors::{inactive_transaction, sql_error};
use super::state::SqliteExecutor;
use super::support::{quote_identifier, sanitize_identifier};
use crate::migration::{
    ColumnToAdd, IndexToCreate, MigrationStatement, MigrationStatementKind, SchemaMigrationPlan,
    SchemaMigrationWarning, TableToCreate,
};
use openauth_core::db::{DbField, DbFieldType, DbSchema, OnDelete};
use openauth_core::error::OpenAuthError;

pub(super) async fn plan_migrations(
    mut executor: SqliteExecutor<'_, '_>,
    schema: &DbSchema,
) -> Result<SchemaMigrationPlan, OpenAuthError> {
    build_migration_plan(&mut executor, schema).await
}

pub(super) async fn create_schema(
    mut executor: SqliteExecutor<'_, '_>,
    schema: &DbSchema,
) -> Result<(), OpenAuthError> {
    let plan = build_migration_plan(&mut executor, schema).await?;
    execute_migration_plan(&mut executor, &plan).await
}

async fn build_migration_plan(
    executor: &mut SqliteExecutor<'_, '_>,
    schema: &DbSchema,
) -> Result<SchemaMigrationPlan, OpenAuthError> {
    let mut plan = SchemaMigrationPlan::default();
    let mut tables = schema.tables().collect::<Vec<_>>();
    tables.sort_by_key(|(_, table)| table.order.unwrap_or(u16::MAX));

    for (table_logical_name, table) in &tables {
        if table_exists(executor, &table.name).await? {
            for (logical_name, field) in &table.fields {
                if let Some(actual_type) = column_type(executor, &table.name, &field.name).await? {
                    if !sqlite_type_matches(&actual_type, field) {
                        plan.warnings
                            .push(SchemaMigrationWarning::ColumnTypeMismatch {
                                table_name: table.name.clone(),
                                column_name: field.name.clone(),
                                expected: sqlite_type(field),
                                actual: actual_type,
                            });
                    }
                } else {
                    plan.to_be_added.push(ColumnToAdd {
                        table_logical_name: (*table_logical_name).to_owned(),
                        table_name: table.name.clone(),
                        field_logical_name: logical_name.clone(),
                        column_name: field.name.clone(),
                    });
                    plan.statements.push(MigrationStatement {
                        kind: MigrationStatementKind::AddColumn,
                        sql: add_column_statement(&table.name, logical_name, field)?,
                    });
                }
            }
        } else {
            plan.to_be_created.push(TableToCreate {
                logical_name: (*table_logical_name).to_owned(),
                table_name: table.name.clone(),
            });
            plan.statements.push(MigrationStatement {
                kind: MigrationStatementKind::CreateTable,
                sql: create_table_statement(table)?,
            });
        }
    }

    for (table_logical_name, table) in tables {
        for (logical_name, field) in &table.fields {
            if field.index && !field.unique {
                let index_name = format!("idx_{}_{}", table.name, logical_name);
                let index_name = sanitize_identifier(&index_name)?;
                if !index_exists(executor, &index_name).await? {
                    plan.indexes_to_be_created.push(IndexToCreate {
                        table_logical_name: table_logical_name.to_owned(),
                        table_name: table.name.clone(),
                        field_logical_name: logical_name.clone(),
                        column_name: field.name.clone(),
                        index_name: index_name.clone(),
                    });
                    plan.statements.push(MigrationStatement {
                        kind: MigrationStatementKind::CreateIndex,
                        sql: create_index_statement(&table.name, &field.name, &index_name)?,
                    });
                }
            }
        }
    }

    Ok(plan)
}

pub(super) async fn execute_migration_plan(
    executor: &mut SqliteExecutor<'_, '_>,
    plan: &SchemaMigrationPlan,
) -> Result<(), OpenAuthError> {
    for statement in &plan.statements {
        execute_schema_sql(executor, &statement.sql).await?;
    }
    Ok(())
}

async fn table_exists(
    executor: &mut SqliteExecutor<'_, '_>,
    table: &str,
) -> Result<bool, OpenAuthError> {
    let count = match executor {
        SqliteExecutor::Pool(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table)
        .fetch_one(*pool)
        .await
        .map_err(sql_error)?,
        SqliteExecutor::Transaction(tx) => {
            let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&mut **tx)
            .await
            .map_err(sql_error)?
        }
    };
    Ok(count > 0)
}

async fn column_type(
    executor: &mut SqliteExecutor<'_, '_>,
    table: &str,
    column: &str,
) -> Result<Option<String>, OpenAuthError> {
    let sql = format!(
        "SELECT type FROM pragma_table_info({}) WHERE name = ?",
        sql_string_literal(table),
    );
    let column_type = match executor {
        SqliteExecutor::Pool(pool) => sqlx::query_scalar::<_, String>(&sql)
            .bind(column)
            .fetch_optional(*pool)
            .await
            .map_err(sql_error)?,
        SqliteExecutor::Transaction(tx) => {
            let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
            sqlx::query_scalar::<_, String>(&sql)
                .bind(column)
                .fetch_optional(&mut **tx)
                .await
                .map_err(sql_error)?
        }
    };
    Ok(column_type)
}

async fn index_exists(
    executor: &mut SqliteExecutor<'_, '_>,
    index: &str,
) -> Result<bool, OpenAuthError> {
    let count = match executor {
        SqliteExecutor::Pool(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?",
        )
        .bind(index)
        .fetch_one(*pool)
        .await
        .map_err(sql_error)?,
        SqliteExecutor::Transaction(tx) => {
            let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?",
            )
            .bind(index)
            .fetch_one(&mut **tx)
            .await
            .map_err(sql_error)?
        }
    };
    Ok(count > 0)
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
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

fn create_table_statement(table: &openauth_core::db::DbTable) -> Result<String, OpenAuthError> {
    let mut columns = Vec::new();
    for (logical_name, field) in &table.fields {
        columns.push(column_definition(logical_name, field)?);
    }
    Ok(format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        quote_identifier(&table.name)?,
        columns.join(", ")
    ))
}

fn add_column_statement(
    table: &str,
    logical_name: &str,
    field: &DbField,
) -> Result<String, OpenAuthError> {
    Ok(format!(
        "ALTER TABLE {} ADD COLUMN {}",
        quote_identifier(table)?,
        column_definition(logical_name, field)?,
    ))
}

fn create_index_statement(table: &str, column: &str, index: &str) -> Result<String, OpenAuthError> {
    Ok(format!(
        "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
        quote_identifier(index)?,
        quote_identifier(table)?,
        quote_identifier(column)?,
    ))
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

fn sqlite_type_matches(actual: &str, field: &DbField) -> bool {
    let actual = normalized_type(actual);
    match field.field_type {
        DbFieldType::String
        | DbFieldType::Timestamp
        | DbFieldType::Json
        | DbFieldType::StringArray
        | DbFieldType::NumberArray => matches!(
            actual.as_str(),
            "text" | "varchar" | "character varying" | "nvarchar" | "clob"
        ),
        DbFieldType::Number => matches!(
            actual.as_str(),
            "integer" | "int" | "bigint" | "smallint" | "tinyint" | "numeric" | "real" | "double"
        ),
        DbFieldType::Boolean => matches!(
            actual.as_str(),
            "integer" | "int" | "bigint" | "smallint" | "tinyint" | "boolean" | "bool"
        ),
    }
}

fn normalized_type(value: &str) -> String {
    value
        .trim()
        .split_once('(')
        .map(|(prefix, _)| prefix)
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
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
