use std::collections::{HashMap, HashSet};

use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Bool, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl, SimpleAsyncConnection};
use rustauth_core::db::{
    plan_schema_migration, DbSchema, ForeignKey, IdGeneration, OnDelete, SchemaMigrationPlan,
    SqlColumnSnapshot, SqlDialect, SqlSchemaSnapshot,
};
use rustauth_core::error::RustAuthError;

use super::errors::{diesel_error, inactive_transaction, pool_error};
use super::state::DieselPostgresExecutor;
use super::support::sanitize_identifier;
use crate::bind_postgres_params;

pub(super) async fn plan_migrations(
    mut executor: DieselPostgresExecutor<'_>,
    schema: &DbSchema,
) -> Result<SchemaMigrationPlan, RustAuthError> {
    build_migration_plan(&mut executor, schema).await
}

pub(super) async fn create_schema(
    mut executor: DieselPostgresExecutor<'_>,
    schema: &DbSchema,
) -> Result<(), RustAuthError> {
    let plan = build_migration_plan(&mut executor, schema).await?;
    crate::migration::ensure_executable(&plan)?;
    match executor {
        DieselPostgresExecutor::Pool(pool) => execute_migration_plan_on_pool(pool, &plan).await,
        DieselPostgresExecutor::Transaction(guard) => {
            execute_migration_plan(&mut DieselPostgresExecutor::Transaction(guard), &plan).await
        }
    }
}

async fn build_migration_plan(
    executor: &mut DieselPostgresExecutor<'_>,
    schema: &DbSchema,
) -> Result<SchemaMigrationPlan, RustAuthError> {
    let snapshot = load_schema_snapshot(executor, schema).await?;
    plan_schema_migration(SqlDialect::Postgres, schema, &snapshot)
}

async fn load_schema_snapshot(
    executor: &mut DieselPostgresExecutor<'_>,
    schema: &DbSchema,
) -> Result<SqlSchemaSnapshot, RustAuthError> {
    let mut tables = schema.tables().collect::<Vec<_>>();
    tables.sort_by_key(|(_, table)| table.order.unwrap_or(u16::MAX));

    let mut groups: Vec<(Option<&str>, Vec<String>)> = Vec::new();
    for (_, table) in &tables {
        let table_ref = PgTableRef::new(&table.name);
        match groups
            .iter_mut()
            .find(|(schema, _)| *schema == table_ref.schema)
        {
            Some((_, names)) => names.push(table_ref.name.to_owned()),
            None => groups.push((table_ref.schema, vec![table_ref.name.to_owned()])),
        }
    }

    let mut catalogs: HashMap<Option<&str>, SchemaCatalog> = HashMap::new();
    for (schema_name, names) in &groups {
        let catalog = SchemaCatalog::load(executor, *schema_name, names).await?;
        catalogs.insert(*schema_name, catalog);
    }

    let mut snapshot = SqlSchemaSnapshot::default();
    for (_, table) in &tables {
        let table_ref = PgTableRef::new(&table.name);
        let Some(catalog) = catalogs.get(&table_ref.schema) else {
            continue;
        };

        if catalog.tables.contains(table_ref.name) {
            snapshot = snapshot.with_table(&table.name);
            for (_, field) in &table.fields {
                if let Some(column) = catalog.column_snapshot(table_ref, &field.name) {
                    snapshot = snapshot.with_column(&table.name, column);
                }
                if field.unique
                    && catalog
                        .unique_columns
                        .contains(&(table_ref.name.to_owned(), field.name.clone()))
                {
                    snapshot = snapshot.with_unique_column(&table.name, &field.name);
                }
            }
        }

        for (logical_name, field) in &table.fields {
            if field.index || field.unique {
                let prefix = if field.unique { "uidx" } else { "idx" };
                let index_name = format!("{prefix}_{}_{}", table.name, logical_name);
                let index_name = sanitize_identifier(&index_name)?;
                if catalog.indexes.contains(&index_name) {
                    snapshot = snapshot.with_index(&table.name, index_name);
                }
            }
        }
    }

    Ok(snapshot)
}

#[derive(Debug, Clone)]
struct ColumnInfo {
    data_type: String,
    nullable: bool,
    column_default: Option<String>,
    is_identity: bool,
}

#[derive(Debug, Default)]
struct SchemaCatalog {
    tables: HashSet<String>,
    columns: HashMap<(String, String), ColumnInfo>,
    primary_key_columns: HashSet<(String, String)>,
    unique_columns: HashSet<(String, String)>,
    foreign_keys: HashMap<(String, String), (String, String, String, OnDelete)>,
    indexes: HashSet<String>,
}

impl SchemaCatalog {
    async fn load(
        executor: &mut DieselPostgresExecutor<'_>,
        schema_name: Option<&str>,
        table_names: &[String],
    ) -> Result<Self, RustAuthError> {
        let mut catalog = Self::default();

        let table_rows: Vec<TableNameRow> = fetch_catalog_rows(
            executor,
            "SELECT table_name::text AS table_name FROM information_schema.tables \
             WHERE table_schema = COALESCE($1, current_schema()) \
               AND table_type = 'BASE TABLE' AND table_name = ANY($2)",
            schema_name,
            table_names,
        )
        .await?;
        catalog.tables = table_rows.into_iter().map(|row| row.table_name).collect();

        let column_rows: Vec<ColumnInfoRow> = fetch_catalog_rows(
            executor,
            "SELECT table_name::text AS table_name, column_name::text AS column_name, \
                    CASE WHEN data_type = 'ARRAY' THEN udt_name::text ELSE data_type::text END AS data_type, \
                    (is_nullable = 'YES') AS nullable, \
                    column_default::text AS column_default, \
                    (is_identity = 'YES') AS is_identity \
             FROM information_schema.columns \
             WHERE table_schema = COALESCE($1, current_schema()) AND table_name = ANY($2)",
            schema_name,
            table_names,
        )
        .await?;
        for row in column_rows {
            catalog.columns.insert(
                (row.table_name, row.column_name),
                ColumnInfo {
                    data_type: row.data_type,
                    nullable: row.nullable,
                    column_default: row.column_default,
                    is_identity: row.is_identity,
                },
            );
        }

        let index_column_rows: Vec<IndexColumnRow> = fetch_catalog_rows(
            executor,
            "SELECT tbl.relname::text AS table_name, attr.attname::text AS column_name, \
                    bool_or(i.indisprimary) AS is_primary, bool_or(i.indisunique) AS is_unique \
             FROM pg_index i \
             JOIN pg_class tbl ON tbl.oid = i.indrelid \
             JOIN pg_namespace ns ON ns.oid = tbl.relnamespace \
             JOIN pg_attribute attr ON attr.attrelid = tbl.oid AND attr.attnum = ANY(i.indkey) \
             WHERE ns.nspname = COALESCE($1, current_schema()) \
               AND tbl.relname = ANY($2) \
               AND NOT attr.attisdropped \
             GROUP BY tbl.relname, attr.attname",
            schema_name,
            table_names,
        )
        .await?;
        for row in index_column_rows {
            if row.is_primary {
                catalog
                    .primary_key_columns
                    .insert((row.table_name.clone(), row.column_name.clone()));
            }
            if row.is_unique {
                catalog
                    .unique_columns
                    .insert((row.table_name.clone(), row.column_name));
            }
        }

        let foreign_key_rows: Vec<ForeignKeyRow> = fetch_catalog_rows(
            executor,
            "SELECT src.relname::text AS table_name, src_attr.attname::text AS column_name, \
                    ref_ns.nspname::text AS ref_schema, ref.relname::text AS ref_table, \
                    ref_attr.attname::text AS ref_column, con.confdeltype::text AS delete_rule \
             FROM pg_constraint con \
             JOIN pg_class src ON src.oid = con.conrelid \
             JOIN pg_namespace src_ns ON src_ns.oid = src.relnamespace \
             JOIN pg_class ref ON ref.oid = con.confrelid \
             JOIN pg_namespace ref_ns ON ref_ns.oid = ref.relnamespace \
             CROSS JOIN LATERAL unnest(con.conkey, con.confkey) AS cols(src_attnum, ref_attnum) \
             JOIN pg_attribute src_attr \
               ON src_attr.attrelid = src.oid AND src_attr.attnum = cols.src_attnum \
             JOIN pg_attribute ref_attr \
               ON ref_attr.attrelid = ref.oid AND ref_attr.attnum = cols.ref_attnum \
             WHERE con.contype = 'f' \
               AND src_ns.nspname = COALESCE($1, current_schema()) \
               AND src.relname = ANY($2)",
            schema_name,
            table_names,
        )
        .await?;
        for row in foreign_key_rows {
            catalog
                .foreign_keys
                .entry((row.table_name, row.column_name))
                .or_insert((
                    row.ref_schema,
                    row.ref_table,
                    row.ref_column,
                    parse_on_delete(&row.delete_rule),
                ));
        }

        let index_rows: Vec<IndexNameRow> = fetch_catalog_rows(
            executor,
            "SELECT indexname::text AS index_name FROM pg_indexes \
             WHERE schemaname = COALESCE($1, current_schema()) AND tablename = ANY($2)",
            schema_name,
            table_names,
        )
        .await?;
        catalog.indexes = index_rows.into_iter().map(|row| row.index_name).collect();

        Ok(catalog)
    }

    fn column_snapshot(
        &self,
        table_ref: PgTableRef<'_>,
        column: &str,
    ) -> Option<SqlColumnSnapshot> {
        let key = (table_ref.name.to_owned(), column.to_owned());
        let info = self.columns.get(&key)?;
        let generated_id = if info.is_identity {
            Some(IdGeneration::Serial)
        } else if info
            .column_default
            .as_deref()
            .is_some_and(|default| default.contains("gen_random_uuid"))
        {
            Some(IdGeneration::Uuid)
        } else {
            None
        };
        let mut snapshot = SqlColumnSnapshot::new(column, &info.data_type)
            .nullable(info.nullable)
            .primary_key(self.primary_key_columns.contains(&key))
            .generated_id(generated_id);
        if let Some((ref_schema, ref_table, ref_column, on_delete)) = self.foreign_keys.get(&key) {
            let target_table = match table_ref.schema {
                Some(_) => format!("{ref_schema}.{ref_table}"),
                None => ref_table.clone(),
            };
            snapshot = snapshot.references(ForeignKey::new(target_table, ref_column, *on_delete));
        }
        Some(snapshot)
    }
}

#[derive(QueryableByName)]
struct TableNameRow {
    #[diesel(sql_type = Text)]
    table_name: String,
}

#[derive(QueryableByName)]
struct ColumnInfoRow {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    column_name: String,
    #[diesel(sql_type = Text)]
    data_type: String,
    #[diesel(sql_type = Bool)]
    nullable: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    column_default: Option<String>,
    #[diesel(sql_type = Bool)]
    is_identity: bool,
}

#[derive(QueryableByName)]
struct IndexColumnRow {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    column_name: String,
    #[diesel(sql_type = Bool)]
    is_primary: bool,
    #[diesel(sql_type = Bool)]
    is_unique: bool,
}

#[derive(QueryableByName)]
struct ForeignKeyRow {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    column_name: String,
    #[diesel(sql_type = Text)]
    ref_schema: String,
    #[diesel(sql_type = Text)]
    ref_table: String,
    #[diesel(sql_type = Text)]
    ref_column: String,
    #[diesel(sql_type = Text)]
    delete_rule: String,
}

#[derive(QueryableByName)]
struct IndexNameRow {
    #[diesel(sql_type = Text)]
    index_name: String,
}

async fn fetch_catalog_rows<T>(
    executor: &mut DieselPostgresExecutor<'_>,
    sql: &str,
    schema_name: Option<&str>,
    table_names: &[String],
) -> Result<Vec<T>, RustAuthError>
where
    T: QueryableByName<diesel::pg::Pg> + Send + 'static,
{
    let params = catalog_params(schema_name, table_names)?;
    match executor {
        DieselPostgresExecutor::Pool(pool) => {
            let mut pooled = pool.get().await.map_err(pool_error)?;
            let conn = &mut *pooled;
            let query = bind_postgres_params(sql, &params)?;
            query.get_results(conn).await.map_err(diesel_error)
        }
        DieselPostgresExecutor::Transaction(conn) => {
            let conn = conn.as_mut().ok_or_else(inactive_transaction)?.as_mut();
            let query = bind_postgres_params(sql, &params)?;
            query.get_results(conn).await.map_err(diesel_error)
        }
    }
}

fn catalog_params(
    schema_name: Option<&str>,
    table_names: &[String],
) -> Result<Vec<rustauth_core::db::SqlParam>, RustAuthError> {
    use rustauth_core::db::{DbField, DbFieldType, DbValue, SqlParam};
    let schema_field = DbField::new("schema", DbFieldType::String).optional();
    let tables_field = DbField::new("tables", DbFieldType::StringArray);
    Ok(vec![
        SqlParam::new(
            &schema_field,
            schema_name
                .map(ToOwned::to_owned)
                .map(DbValue::String)
                .unwrap_or(DbValue::Null),
        ),
        SqlParam::new(&tables_field, DbValue::StringArray(table_names.to_vec())),
    ])
}

pub(super) async fn execute_migration_plan(
    executor: &mut DieselPostgresExecutor<'_>,
    plan: &SchemaMigrationPlan,
) -> Result<(), RustAuthError> {
    for statement in &plan.statements {
        execute_schema_sql(executor, &statement.sql).await?;
    }
    Ok(())
}

pub(super) async fn execute_migration_plan_on_pool(
    pool: &diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>,
    plan: &SchemaMigrationPlan,
) -> Result<(), RustAuthError> {
    let mut pooled = pool.get().await.map_err(pool_error)?;
    let conn = &mut *pooled;
    SimpleAsyncConnection::batch_execute(conn, "BEGIN")
        .await
        .map_err(diesel_error)?;
    for statement in &plan.statements {
        diesel_async::RunQueryDsl::execute(diesel::sql_query(statement.sql.clone()), conn)
            .await
            .map_err(diesel_error)?;
    }
    SimpleAsyncConnection::batch_execute(conn, "COMMIT")
        .await
        .map_err(diesel_error)?;
    Ok(())
}

#[derive(Clone, Copy)]
struct PgTableRef<'a> {
    schema: Option<&'a str>,
    name: &'a str,
}

impl<'a> PgTableRef<'a> {
    fn new(table: &'a str) -> Self {
        match table.split_once('.') {
            Some((schema, name)) => Self {
                schema: Some(schema),
                name,
            },
            None => Self {
                schema: None,
                name: table,
            },
        }
    }
}

fn parse_on_delete(value: &str) -> OnDelete {
    match value {
        "r" => OnDelete::Restrict,
        "c" => OnDelete::Cascade,
        "n" => OnDelete::SetNull,
        "d" => OnDelete::SetDefault,
        _ => OnDelete::NoAction,
    }
}

pub(super) async fn execute_schema_sql(
    executor: &mut DieselPostgresExecutor<'_>,
    sql: &str,
) -> Result<(), RustAuthError> {
    match executor {
        DieselPostgresExecutor::Pool(pool) => {
            let mut pooled = pool.get().await.map_err(pool_error)?;
            let conn = &mut *pooled;
            diesel_async::RunQueryDsl::execute(diesel::sql_query(sql), conn)
                .await
                .map_err(diesel_error)?;
        }
        DieselPostgresExecutor::Transaction(conn) => {
            let conn = conn.as_mut().ok_or_else(inactive_transaction)?.as_mut();
            diesel_async::RunQueryDsl::execute(diesel::sql_query(sql), conn)
                .await
                .map_err(diesel_error)?;
        }
    }
    Ok(())
}
