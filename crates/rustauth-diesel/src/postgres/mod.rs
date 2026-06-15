mod errors;
mod row;
mod schema;
mod state;
mod support;

use std::fmt;
use std::sync::Arc;

use diesel::deserialize::QueryableByName;
use diesel::sql_types::BigInt;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl, SimpleAsyncConnection};
use rustauth_core::db::{
    auth_schema, rate_limit_consume_statements, validate_rate_limit_rule, AdapterCapabilities,
    AdapterFuture, AuthSchemaOptions, Count, Create, DbAdapter, DbField, DbFieldType, DbRecord,
    DbSchema, DbValue, Delete, DeleteMany, FindMany, FindOne, SchemaCreation, SchemaMigrationPlan,
    SqlDialect, SqlParam, TransactionCallback, Update, UpdateMany,
};
use rustauth_core::error::RustAuthError;
use rustauth_core::options::{
    RateLimitConsumeInput, RateLimitDecision, RateLimitFuture, RateLimitRecord, RateLimitStore,
};
use rustauth_core::plugin::{PluginMigration, PluginMigrationBody};
use tokio::sync::Mutex;

use self::errors::{diesel_error, inactive_transaction, pool_error};
use self::schema::{
    create_schema, execute_migration_plan_on_pool, plan_migrations as plan_schema_migrations,
};
use self::state::{DieselPostgresExecutor, DieselPostgresState};
use crate::{
    bind_postgres_params, consume_record, count_from_i64, count_to_i64, RateLimitSqlNames,
};

#[derive(Clone)]
pub struct DieselPostgresAdapter {
    pool: Pool<AsyncPgConnection>,
    schema: Arc<DbSchema>,
}

impl fmt::Debug for DieselPostgresAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DieselPostgresAdapter")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct DieselPostgresRateLimitStore {
    pool: Pool<AsyncPgConnection>,
    names: RateLimitSqlNames,
}

impl fmt::Debug for DieselPostgresRateLimitStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DieselPostgresRateLimitStore")
            .field("names", &self.names)
            .finish_non_exhaustive()
    }
}

impl DieselPostgresRateLimitStore {
    pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
        Self::with_table(pool, "rate_limits")
    }

    pub fn with_table(pool: Pool<AsyncPgConnection>, table: impl Into<String>) -> Self {
        Self {
            pool,
            names: RateLimitSqlNames::new(table),
        }
    }
}

impl From<&DieselPostgresAdapter> for DieselPostgresRateLimitStore {
    fn from(adapter: &DieselPostgresAdapter) -> Self {
        Self {
            pool: adapter.pool.clone(),
            names: RateLimitSqlNames::from_schema(&adapter.schema),
        }
    }
}

impl RateLimitStore for DieselPostgresRateLimitStore {
    fn consume<'a>(&'a self, input: RateLimitConsumeInput) -> RateLimitFuture<'a> {
        Box::pin(
            async move { consume_diesel_postgres_rate_limit(&self.pool, &self.names, input).await },
        )
    }
}

#[derive(QueryableByName)]
struct RateLimitRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
    #[diesel(sql_type = BigInt)]
    last_request: i64,
}

async fn consume_diesel_postgres_rate_limit(
    pool: &Pool<AsyncPgConnection>,
    names: &RateLimitSqlNames,
    input: RateLimitConsumeInput,
) -> Result<RateLimitDecision, RustAuthError> {
    validate_rate_limit_rule(&input.rule)?;
    let plan = rate_limit_consume_statements(
        SqlDialect::Postgres,
        &names.table,
        &names.key,
        &names.count,
        &names.last_request,
    )?;
    let mut pooled = pool.get().await.map_err(pool_error)?;
    let conn = &mut *pooled;
    SimpleAsyncConnection::batch_execute(conn, "BEGIN")
        .await
        .map_err(diesel_error)?;

    let insert_params = vec![
        rate_limit_param(&names.key, input.key.clone()),
        rate_limit_number_param(&names.last_request, input.now_ms),
    ];
    bind_postgres_params(&plan.insert_ignore.sql, &insert_params)?
        .execute(conn)
        .await
        .map_err(diesel_error)?;

    let select_params = vec![rate_limit_param(&names.key, input.key.clone())];
    let row = match bind_postgres_params(&plan.select.sql, &select_params)?
        .get_result::<RateLimitRow>(conn)
        .await
    {
        Ok(row) => row,
        Err(diesel::result::Error::NotFound) => {
            return Err(RustAuthError::Adapter("missing rate limit row".to_owned()));
        }
        Err(error) => return Err(diesel_error(error)),
    };

    let (decision, record, update) =
        consume_record(input, Some(postgres_record(row, &names.key)?))?;
    if decision.permitted && update {
        let update_params = vec![
            rate_limit_number_param(&names.count, count_to_i64(record.count)?),
            rate_limit_number_param(&names.last_request, record.last_request),
            rate_limit_param(&names.key, record.key.clone()),
        ];
        bind_postgres_params(&plan.update.sql, &update_params)?
            .execute(conn)
            .await
            .map_err(diesel_error)?;
    }

    SimpleAsyncConnection::batch_execute(conn, "COMMIT")
        .await
        .map_err(diesel_error)?;
    Ok(decision)
}

fn postgres_record(row: RateLimitRow, key: &str) -> Result<RateLimitRecord, RustAuthError> {
    Ok(RateLimitRecord {
        key: key.to_owned(),
        count: count_from_i64(row.count)?,
        last_request: row.last_request,
    })
}

fn rate_limit_param(name: &str, value: String) -> SqlParam {
    SqlParam::new(
        &DbField::new(name, DbFieldType::String),
        DbValue::String(value),
    )
}

fn rate_limit_number_param(name: &str, value: i64) -> SqlParam {
    SqlParam::new(
        &DbField::new(name, DbFieldType::Number),
        DbValue::Number(value),
    )
}

impl DieselPostgresAdapter {
    pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
        Self::with_schema(pool, auth_schema(AuthSchemaOptions::default()))
    }

    pub fn with_schema(pool: Pool<AsyncPgConnection>, schema: DbSchema) -> Self {
        Self {
            pool,
            schema: Arc::new(schema),
        }
    }

    pub async fn connect(database_url: &str) -> Result<Self, RustAuthError> {
        Self::connect_with_schema(database_url, auth_schema(AuthSchemaOptions::default())).await
    }

    pub async fn connect_with_schema(
        database_url: &str,
        schema: DbSchema,
    ) -> Result<Self, RustAuthError> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let pool = Pool::builder(manager)
            .build()
            .map_err(|error| RustAuthError::Adapter(format!("diesel postgres pool: {error}")))?;
        Ok(Self::with_schema(pool, schema))
    }

    pub fn pool(&self) -> &Pool<AsyncPgConnection> {
        &self.pool
    }

    pub async fn plan_migrations(
        &self,
        schema: &DbSchema,
    ) -> Result<SchemaMigrationPlan, RustAuthError> {
        plan_schema_migrations(DieselPostgresExecutor::Pool(&self.pool), schema).await
    }

    pub async fn compile_migrations(&self, schema: &DbSchema) -> Result<String, RustAuthError> {
        Ok(self.plan_migrations(schema).await?.compile())
    }

    #[doc(hidden)]
    pub async fn apply_migration_plan(
        &self,
        plan: &SchemaMigrationPlan,
    ) -> Result<(), RustAuthError> {
        crate::migration::ensure_executable(plan)?;
        execute_migration_plan_on_pool(&self.pool, plan).await
    }

    fn state(&self) -> DieselPostgresState<'_> {
        DieselPostgresState {
            schema: &self.schema,
            executor: DieselPostgresExecutor::Pool(&self.pool),
        }
    }
}

impl DbAdapter for DieselPostgresAdapter {
    fn id(&self) -> &str {
        "diesel-postgres"
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities::new(self.id())
            .named("Diesel Postgres")
            .with_uuid_ids()
            .with_json()
            .with_arrays()
            .with_native_joins()
            .with_transactions()
    }

    fn create<'a>(&'a self, query: Create) -> AdapterFuture<'a, DbRecord> {
        Box::pin(async move { self.state().create(query).await })
    }

    fn find_one<'a>(&'a self, query: FindOne) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move { self.state().find_one(query).await })
    }

    fn find_many<'a>(&'a self, query: FindMany) -> AdapterFuture<'a, Vec<DbRecord>> {
        Box::pin(async move { self.state().find_many(query).await })
    }

    fn count<'a>(&'a self, query: Count) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().count(query).await })
    }

    fn update<'a>(&'a self, query: Update) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move { self.state().update(query).await })
    }

    fn update_many<'a>(&'a self, query: UpdateMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().update_many(query).await })
    }

    fn delete<'a>(&'a self, query: Delete) -> AdapterFuture<'a, ()> {
        Box::pin(async move { self.state().delete(query).await })
    }

    fn delete_many<'a>(&'a self, query: DeleteMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().delete_many(query).await })
    }

    fn transaction<'a>(&'a self, callback: TransactionCallback<'a>) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            let mut pooled = self.pool.get().await.map_err(pool_error)?;
            {
                let conn = &mut *pooled;
                SimpleAsyncConnection::batch_execute(conn, "BEGIN")
                    .await
                    .map_err(diesel_error)?;
            }
            let adapter = Arc::new(DieselPostgresTxAdapter {
                schema: Arc::clone(&self.schema),
                conn: Mutex::new(Some(pooled)),
            });
            let result = callback(Box::new(Arc::clone(&adapter))).await;
            let mut guard = adapter.conn.lock().await;
            let Some(mut pooled) = guard.take() else {
                return Err(RustAuthError::Adapter(
                    "diesel postgres transaction was already completed".to_owned(),
                ));
            };
            drop(guard);
            let conn = &mut *pooled;
            match result {
                Ok(()) => SimpleAsyncConnection::batch_execute(conn, "COMMIT")
                    .await
                    .map_err(diesel_error),
                Err(error) => {
                    let _rollback_result =
                        SimpleAsyncConnection::batch_execute(conn, "ROLLBACK").await;
                    Err(error)
                }
            }
        })
    }

    fn create_schema<'a>(
        &'a self,
        schema: &'a DbSchema,
        file: Option<&'a str>,
    ) -> AdapterFuture<'a, Option<SchemaCreation>> {
        Box::pin(async move {
            let code = if file.is_some() {
                Some(self.compile_migrations(schema).await?)
            } else {
                None
            };
            create_schema(DieselPostgresExecutor::Pool(&self.pool), schema).await?;
            match (file, code) {
                (Some(path), Some(code)) => {
                    Ok(Some(crate::migration::write_schema_file(path, code).await?))
                }
                _ => Ok(None),
            }
        })
    }

    fn run_migrations<'a>(&'a self, schema: &'a DbSchema) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            let plan =
                plan_schema_migrations(DieselPostgresExecutor::Pool(&self.pool), schema).await?;
            crate::migration::ensure_executable(&plan)?;
            execute_migration_plan_on_pool(&self.pool, &plan).await
        })
    }

    fn run_plugin_migrations<'a>(
        &'a self,
        migrations: &'a [PluginMigration],
    ) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            for migration in migrations {
                execute_plugin_migration_postgres(&self.pool, migration).await?;
            }
            Ok(())
        })
    }
}

async fn execute_plugin_migration_postgres(
    pool: &Pool<AsyncPgConnection>,
    migration: &PluginMigration,
) -> Result<(), RustAuthError> {
    match &migration.body {
        Some(PluginMigrationBody::Sql(sql)) => {
            let mut pooled = pool.get().await.map_err(pool_error)?;
            let conn = &mut *pooled;
            diesel_async::RunQueryDsl::execute(diesel::sql_query(sql), conn)
                .await
                .map_err(diesel_error)?;
        }
        Some(PluginMigrationBody::Plan(steps)) => {
            let mut pooled = pool.get().await.map_err(pool_error)?;
            let conn = &mut *pooled;
            for step in steps {
                if let Some(sql) = &step.sql {
                    diesel_async::RunQueryDsl::execute(diesel::sql_query(sql), conn)
                        .await
                        .map_err(diesel_error)?;
                }
            }
        }
        None => {}
    }
    Ok(())
}

struct DieselPostgresTxAdapter {
    schema: Arc<DbSchema>,
    conn: Mutex<Option<Object<AsyncPgConnection>>>,
}

impl fmt::Debug for DieselPostgresTxAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DieselPostgresTxAdapter")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

impl DbAdapter for DieselPostgresTxAdapter {
    fn id(&self) -> &str {
        "diesel-postgres"
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities::new(self.id())
            .named("Diesel Postgres")
            .with_uuid_ids()
            .with_json()
            .with_arrays()
            .with_native_joins()
            .with_transactions()
    }

    fn create<'a>(&'a self, query: Create) -> AdapterFuture<'a, DbRecord> {
        Box::pin(async move { self.state().await?.create(query).await })
    }

    fn find_one<'a>(&'a self, query: FindOne) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move { self.state().await?.find_one(query).await })
    }

    fn find_many<'a>(&'a self, query: FindMany) -> AdapterFuture<'a, Vec<DbRecord>> {
        Box::pin(async move { self.state().await?.find_many(query).await })
    }

    fn count<'a>(&'a self, query: Count) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().await?.count(query).await })
    }

    fn update<'a>(&'a self, query: Update) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move { self.state().await?.update(query).await })
    }

    fn update_many<'a>(&'a self, query: UpdateMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().await?.update_many(query).await })
    }

    fn delete<'a>(&'a self, query: Delete) -> AdapterFuture<'a, ()> {
        Box::pin(async move { self.state().await?.delete(query).await })
    }

    fn delete_many<'a>(&'a self, query: DeleteMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move { self.state().await?.delete_many(query).await })
    }

    fn transaction<'a>(&'a self, callback: TransactionCallback<'a>) -> AdapterFuture<'a, ()> {
        callback(Box::new(self))
    }
}

impl DieselPostgresTxAdapter {
    async fn state(&self) -> Result<DieselPostgresState<'_>, RustAuthError> {
        let guard = self.conn.lock().await;
        if guard.is_none() {
            return Err(inactive_transaction());
        }
        Ok(DieselPostgresState {
            schema: &self.schema,
            executor: DieselPostgresExecutor::Transaction(guard),
        })
    }
}
