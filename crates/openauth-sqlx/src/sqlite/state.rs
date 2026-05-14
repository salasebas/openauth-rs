use openauth_core::db::{
    Count, Create, DbRecord, DbSchema, Delete, DeleteMany, FindMany, FindOne, SortDirection,
    Update, UpdateMany,
};
use openauth_core::error::OpenAuthError;
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::errors::{inactive_transaction, sql_error};
use super::joins::{
    base_alias, internal_base_selection, join_alias, join_field_alias, joined_rows,
    resolve_field_from_selection, resolve_native_joins,
};
use super::query::{bind_value, where_sql};
use super::row::{row_record, row_value_at};
use super::support::{
    quote_identifier, resolve_field, resolve_table, resolve_table_with_logical, select_fields,
    select_record,
};

pub(super) struct SqliteState<'a, 'tx> {
    pub(super) schema: &'a DbSchema,
    pub(super) executor: SqliteExecutor<'a, 'tx>,
}

pub(super) enum SqliteExecutor<'a, 'tx> {
    Pool(&'a SqlitePool),
    Transaction(tokio::sync::MutexGuard<'a, Option<Transaction<'tx, Sqlite>>>),
}

impl SqliteState<'_, '_> {
    pub(super) async fn create(mut self, query: Create) -> Result<DbRecord, OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut args = SqliteArguments::default();

        for (field, value) in &query.data {
            let (_, metadata) = resolve_field(table, field)?;
            columns.push(quote_identifier(&metadata.name)?);
            values.push("?".to_owned());
            bind_value(&mut args, metadata, value)?;
        }

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_identifier(&table.name)?,
            columns.join(", "),
            values.join(", ")
        );
        self.execute(sql, args).await?;
        Ok(select_record(query.data, &query.select))
    }

    pub(super) async fn find_one(
        self,
        mut query: FindOne,
    ) -> Result<Option<DbRecord>, OpenAuthError> {
        let mut find_many = FindMany::new(query.model);
        find_many.where_clauses = std::mem::take(&mut query.where_clauses);
        find_many.limit = Some(1);
        find_many.select = query.select;
        find_many.joins = query.joins;
        Ok(self.find_many(find_many).await?.into_iter().next())
    }

    pub(super) async fn find_many(
        mut self,
        query: FindMany,
    ) -> Result<Vec<DbRecord>, OpenAuthError> {
        if !query.joins.is_empty() {
            return self.find_many_with_joins(query).await;
        }
        let table = resolve_table(self.schema, &query.model)?;
        let selection = select_fields(table, &query.select)?;
        let mut args = SqliteArguments::default();
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let mut sql = format!(
            "SELECT {} FROM {}{}",
            selection
                .iter()
                .map(|(_, field)| quote_identifier(&field.name))
                .collect::<Result<Vec<_>, _>>()?
                .join(", "),
            quote_identifier(&table.name)?,
            where_sql
        );

        if let Some(sort) = query.sort_by {
            let (_, field) = resolve_field(table, &sort.field)?;
            let direction = match sort.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            sql.push_str(" ORDER BY ");
            sql.push_str(&quote_identifier(&field.name)?);
            sql.push(' ');
            sql.push_str(direction);
        }
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&limit.to_string());
        }
        if let Some(offset) = query.offset {
            sql.push_str(" OFFSET ");
            sql.push_str(&offset.to_string());
        }

        let rows = self.fetch_all(sql, args).await?;
        rows.iter()
            .map(|row| row_record(row, &selection))
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_many_with_joins(
        mut self,
        query: FindMany,
    ) -> Result<Vec<DbRecord>, OpenAuthError> {
        let (base_logical, table) = resolve_table_with_logical(self.schema, &query.model)?;
        let joins = resolve_native_joins(self.schema, base_logical, table, &query.joins, 100)?;
        let base_selection = internal_base_selection(table, &query.select, &joins)?;
        let base_id_alias = "__base_id";
        let mut args = SqliteArguments::default();
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let base_columns = base_selection
            .iter()
            .map(|(_, field)| quote_identifier(&field.name))
            .collect::<Result<Vec<_>, _>>()?;
        let mut base_sql = format!(
            "SELECT {} FROM {}{}",
            base_columns.join(", "),
            quote_identifier(&table.name)?,
            where_sql
        );

        if let Some(sort) = &query.sort_by {
            let (_, field) = resolve_field(table, &sort.field)?;
            let direction = match sort.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            base_sql.push_str(" ORDER BY ");
            base_sql.push_str(&quote_identifier(&field.name)?);
            base_sql.push(' ');
            base_sql.push_str(direction);
        }
        if let Some(limit) = query.limit {
            base_sql.push_str(" LIMIT ");
            base_sql.push_str(&limit.to_string());
        }
        if let Some(offset) = query.offset {
            base_sql.push_str(" OFFSET ");
            base_sql.push_str(&offset.to_string());
        }

        let mut selects = vec![format!(
            "{}.{} AS {}",
            quote_identifier("base")?,
            quote_identifier(&resolve_field_from_selection(&base_selection, "id")?.name)?,
            quote_identifier(base_id_alias)?
        )];
        for (index, (_, field)) in base_selection.iter().enumerate() {
            selects.push(format!(
                "{}.{} AS {}",
                quote_identifier("base")?,
                quote_identifier(&field.name)?,
                quote_identifier(&base_alias(index))?
            ));
        }
        for (join_index, join) in joins.iter().enumerate() {
            for (field_index, (_, field)) in join.selection.iter().enumerate() {
                selects.push(format!(
                    "{}.{} AS {}",
                    quote_identifier(&join_alias(join_index))?,
                    quote_identifier(&field.name)?,
                    quote_identifier(&join_field_alias(join_index, field_index))?
                ));
            }
        }

        let mut sql = format!(
            "SELECT {} FROM ({}) AS {}",
            selects.join(", "),
            base_sql,
            quote_identifier("base")?
        );
        for (index, join) in joins.iter().enumerate() {
            sql.push_str(" LEFT JOIN ");
            sql.push_str(&quote_identifier(&join.table.name)?);
            sql.push_str(" AS ");
            sql.push_str(&quote_identifier(&join_alias(index))?);
            sql.push_str(" ON ");
            sql.push_str(&quote_identifier(&join_alias(index))?);
            sql.push('.');
            sql.push_str(&quote_identifier(&join.to)?);
            sql.push_str(" = ");
            sql.push_str(&quote_identifier("base")?);
            sql.push('.');
            sql.push_str(&quote_identifier(&join.from)?);
        }

        let rows = self.fetch_all(sql, args).await?;
        joined_rows(&rows, &base_selection, &query.select, &joins, row_value_at)
    }

    pub(super) async fn count(mut self, query: Count) -> Result<u64, OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        let mut args = SqliteArguments::default();
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let sql = format!(
            "SELECT COUNT(*) FROM {}{}",
            quote_identifier(&table.name)?,
            where_sql
        );
        let count: i64 = self.fetch_scalar(sql, args).await?;
        u64::try_from(count)
            .map_err(|_| OpenAuthError::Adapter("sqlite returned a negative count".to_owned()))
    }

    pub(super) async fn update(mut self, query: Update) -> Result<Option<DbRecord>, OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        if query.data.is_empty() {
            return Ok(None);
        }
        let selection = select_fields(table, &[])?;
        let mut args = SqliteArguments::default();
        let mut assignments = Vec::new();
        for (field, value) in &query.data {
            let (_, metadata) = resolve_field(table, field)?;
            assignments.push(format!("{} = ?", quote_identifier(&metadata.name)?));
            bind_value(&mut args, metadata, value)?;
        }
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let sql = format!(
            "UPDATE {} SET {} WHERE rowid IN (SELECT rowid FROM {}{} LIMIT 1) RETURNING {}",
            quote_identifier(&table.name)?,
            assignments.join(", "),
            quote_identifier(&table.name)?,
            where_sql,
            selection
                .iter()
                .map(|(_, field)| quote_identifier(&field.name))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        );
        let row = self.fetch_optional(sql, args).await?;
        row.as_ref()
            .map(|row| row_record(row, &selection))
            .transpose()
    }

    pub(super) async fn update_many(mut self, query: UpdateMany) -> Result<u64, OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        if query.data.is_empty() {
            return Ok(0);
        }
        let mut args = SqliteArguments::default();
        let mut assignments = Vec::new();
        for (field, value) in &query.data {
            let (_, metadata) = resolve_field(table, field)?;
            assignments.push(format!("{} = ?", quote_identifier(&metadata.name)?));
            bind_value(&mut args, metadata, value)?;
        }
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let sql = format!(
            "UPDATE {} SET {}{}",
            quote_identifier(&table.name)?,
            assignments.join(", "),
            where_sql
        );
        self.execute(sql, args).await
    }

    pub(super) async fn delete(mut self, query: Delete) -> Result<(), OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        let mut args = SqliteArguments::default();
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let sql = format!(
            "DELETE FROM {} WHERE rowid IN (SELECT rowid FROM {}{} LIMIT 1)",
            quote_identifier(&table.name)?,
            quote_identifier(&table.name)?,
            where_sql
        );
        self.execute(sql, args).await?;
        Ok(())
    }

    pub(super) async fn delete_many(mut self, query: DeleteMany) -> Result<u64, OpenAuthError> {
        let table = resolve_table(self.schema, &query.model)?;
        let mut args = SqliteArguments::default();
        let where_sql = where_sql(table, &query.where_clauses, &mut args)?;
        let sql = format!(
            "DELETE FROM {}{}",
            quote_identifier(&table.name)?,
            where_sql
        );
        self.execute(sql, args).await
    }

    async fn execute(
        &mut self,
        sql: String,
        args: SqliteArguments<'_>,
    ) -> Result<u64, OpenAuthError> {
        match &mut self.executor {
            SqliteExecutor::Pool(pool) => sqlx::query_with(&sql, args)
                .execute(*pool)
                .await
                .map(|result| result.rows_affected())
                .map_err(sql_error),
            SqliteExecutor::Transaction(tx) => {
                let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
                sqlx::query_with(&sql, args)
                    .execute(&mut **tx)
                    .await
                    .map(|result| result.rows_affected())
                    .map_err(sql_error)
            }
        }
    }

    async fn fetch_all(
        &mut self,
        sql: String,
        args: SqliteArguments<'_>,
    ) -> Result<Vec<SqliteRow>, OpenAuthError> {
        match &mut self.executor {
            SqliteExecutor::Pool(pool) => sqlx::query_with(&sql, args)
                .fetch_all(*pool)
                .await
                .map_err(sql_error),
            SqliteExecutor::Transaction(tx) => {
                let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
                sqlx::query_with(&sql, args)
                    .fetch_all(&mut **tx)
                    .await
                    .map_err(sql_error)
            }
        }
    }

    async fn fetch_optional(
        &mut self,
        sql: String,
        args: SqliteArguments<'_>,
    ) -> Result<Option<SqliteRow>, OpenAuthError> {
        match &mut self.executor {
            SqliteExecutor::Pool(pool) => sqlx::query_with(&sql, args)
                .fetch_optional(*pool)
                .await
                .map_err(sql_error),
            SqliteExecutor::Transaction(tx) => {
                let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
                sqlx::query_with(&sql, args)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(sql_error)
            }
        }
    }

    async fn fetch_scalar(
        &mut self,
        sql: String,
        args: SqliteArguments<'_>,
    ) -> Result<i64, OpenAuthError> {
        match &mut self.executor {
            SqliteExecutor::Pool(pool) => sqlx::query_scalar_with(&sql, args)
                .fetch_one(*pool)
                .await
                .map_err(sql_error),
            SqliteExecutor::Transaction(tx) => {
                let tx = tx.as_mut().ok_or_else(inactive_transaction)?;
                sqlx::query_scalar_with(&sql, args)
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(sql_error)
            }
        }
    }
}
