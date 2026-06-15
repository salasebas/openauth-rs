use diesel::deserialize::QueryableByName;
use diesel::sql_types::BigInt;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use rustauth_core::db::{
    AdapterFuture, Count, Create, DbField, DbRecord, DbSchema, DbValue, Delete, DeleteMany,
    FindMany, FindOne, SqlAdapterRunner, SqlDialect, SqlExecutor, SqlParam, SqlRowReader,
    SqlStatement, Update, UpdateMany,
};
use rustauth_core::error::RustAuthError;

use super::errors::{diesel_error_with_context, inactive_transaction, pool_error};
use super::row::{row_value_at, DieselPostgresRow};
use crate::bind_postgres_params;

pub(super) struct DieselPostgresState<'a> {
    pub(super) schema: &'a DbSchema,
    pub(super) executor: DieselPostgresExecutor<'a>,
}

pub(super) enum DieselPostgresExecutor<'a> {
    Pool(&'a Pool<AsyncPgConnection>),
    Transaction(tokio::sync::MutexGuard<'a, Option<Object<AsyncPgConnection>>>),
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

impl DieselPostgresState<'_> {
    pub(super) async fn create(self, query: Create) -> Result<DbRecord, RustAuthError> {
        runner(self).create(query).await
    }

    pub(super) async fn find_one(self, query: FindOne) -> Result<Option<DbRecord>, RustAuthError> {
        runner(self).find_one(query).await
    }

    pub(super) async fn find_many(self, query: FindMany) -> Result<Vec<DbRecord>, RustAuthError> {
        runner(self).find_many(query).await
    }

    pub(super) async fn count(self, query: Count) -> Result<u64, RustAuthError> {
        runner(self).count(query).await
    }

    pub(super) async fn update(self, query: Update) -> Result<Option<DbRecord>, RustAuthError> {
        runner(self).update(query).await
    }

    pub(super) async fn update_many(self, query: UpdateMany) -> Result<u64, RustAuthError> {
        runner(self).update_many(query).await
    }

    pub(super) async fn delete(self, query: Delete) -> Result<(), RustAuthError> {
        runner(self).delete(query).await
    }

    pub(super) async fn delete_many(self, query: DeleteMany) -> Result<u64, RustAuthError> {
        runner(self).delete_many(query).await
    }

    async fn execute_sql(
        &mut self,
        sql: String,
        args: Vec<SqlParam>,
        params: usize,
    ) -> Result<u64, RustAuthError> {
        let query = bind_postgres_params(&sql, &args)?;
        match &mut self.executor {
            DieselPostgresExecutor::Pool(pool) => {
                let mut pooled = pool.get().await.map_err(pool_error)?;
                let conn = &mut *pooled;
                query
                    .execute(conn)
                    .await
                    .map(|count| count as u64)
                    .map_err(|error| diesel_error_with_context("execute", &sql, params, error))
            }
            DieselPostgresExecutor::Transaction(conn) => {
                let conn = conn.as_mut().ok_or_else(inactive_transaction)?.as_mut();
                query
                    .execute(conn)
                    .await
                    .map(|count| count as u64)
                    .map_err(|error| diesel_error_with_context("execute", &sql, params, error))
            }
        }
    }

    async fn fetch_all_sql(
        &mut self,
        sql: String,
        args: Vec<SqlParam>,
        params: usize,
    ) -> Result<Vec<DieselPostgresRow>, RustAuthError> {
        let query = bind_postgres_params(&sql, &args)?;
        match &mut self.executor {
            DieselPostgresExecutor::Pool(pool) => {
                let mut pooled = pool.get().await.map_err(pool_error)?;
                let conn = &mut *pooled;
                query
                    .get_results(conn)
                    .await
                    .map_err(|error| diesel_error_with_context("fetch_all", &sql, params, error))
            }
            DieselPostgresExecutor::Transaction(conn) => {
                let conn = conn.as_mut().ok_or_else(inactive_transaction)?.as_mut();
                query
                    .get_results(conn)
                    .await
                    .map_err(|error| diesel_error_with_context("fetch_all", &sql, params, error))
            }
        }
    }

    async fn fetch_optional_sql(
        &mut self,
        sql: String,
        args: Vec<SqlParam>,
        params: usize,
    ) -> Result<Option<DieselPostgresRow>, RustAuthError> {
        let query = bind_postgres_params(&sql, &args)?;
        match &mut self.executor {
            DieselPostgresExecutor::Pool(pool) => {
                let mut pooled = pool.get().await.map_err(pool_error)?;
                let conn = &mut *pooled;
                query.get_result(conn).await.map(Some).or_else(|error| {
                    if matches!(error, diesel::result::Error::NotFound) {
                        Ok(None)
                    } else {
                        Err(diesel_error_with_context(
                            "fetch_optional",
                            &sql,
                            params,
                            error,
                        ))
                    }
                })
            }
            DieselPostgresExecutor::Transaction(conn) => {
                let conn = conn.as_mut().ok_or_else(inactive_transaction)?.as_mut();
                query.get_result(conn).await.map(Some).or_else(|error| {
                    if matches!(error, diesel::result::Error::NotFound) {
                        Ok(None)
                    } else {
                        Err(diesel_error_with_context(
                            "fetch_optional",
                            &sql,
                            params,
                            error,
                        ))
                    }
                })
            }
        }
    }

    async fn fetch_scalar_sql(
        &mut self,
        sql: String,
        args: Vec<SqlParam>,
        params: usize,
    ) -> Result<i64, RustAuthError> {
        let query = bind_postgres_params(&sql, &args)?;
        match &mut self.executor {
            DieselPostgresExecutor::Pool(pool) => {
                let mut conn = pool.get().await.map_err(pool_error)?;
                query
                    .get_result::<CountRow>(&mut conn)
                    .await
                    .map(|row| row.count)
                    .map_err(|error| diesel_error_with_context("fetch_scalar", &sql, params, error))
            }
            DieselPostgresExecutor::Transaction(conn) => {
                let conn = conn.as_mut().ok_or_else(inactive_transaction)?;
                query
                    .get_result::<CountRow>(conn)
                    .await
                    .map(|row| row.count)
                    .map_err(|error| diesel_error_with_context("fetch_scalar", &sql, params, error))
            }
        }
    }
}

impl SqlExecutor for DieselPostgresState<'_> {
    type Row = DieselPostgresRow;

    fn execute<'a>(&'a mut self, statement: SqlStatement) -> AdapterFuture<'a, u64> {
        Box::pin(async move {
            let params = statement.params.len();
            self.execute_sql(statement.sql, statement.params, params)
                .await
        })
    }

    fn fetch_all<'a>(&'a mut self, statement: SqlStatement) -> AdapterFuture<'a, Vec<Self::Row>> {
        Box::pin(async move {
            let params = statement.params.len();
            self.fetch_all_sql(statement.sql, statement.params, params)
                .await
        })
    }

    fn fetch_optional<'a>(
        &'a mut self,
        statement: SqlStatement,
    ) -> AdapterFuture<'a, Option<Self::Row>> {
        Box::pin(async move {
            let params = statement.params.len();
            self.fetch_optional_sql(statement.sql, statement.params, params)
                .await
        })
    }

    fn fetch_scalar_i64<'a>(&'a mut self, statement: SqlStatement) -> AdapterFuture<'a, i64> {
        Box::pin(async move {
            let params = statement.params.len();
            self.fetch_scalar_sql(statement.sql, statement.params, params)
                .await
        })
    }
}

struct DieselPostgresRowReader;

impl SqlRowReader<DieselPostgresRow> for DieselPostgresRowReader {
    fn value_at(
        &self,
        row: &DieselPostgresRow,
        field: &DbField,
        alias: &str,
    ) -> Result<DbValue, RustAuthError> {
        row_value_at(row, field, alias)
    }
}

fn runner<'a>(
    state: DieselPostgresState<'a>,
) -> SqlAdapterRunner<'a, DieselPostgresState<'a>, DieselPostgresRowReader> {
    SqlAdapterRunner::new(
        SqlDialect::Postgres,
        state.schema,
        state,
        DieselPostgresRowReader,
    )
}
