use super::{
    transform_count_query_with_capabilities, transform_create_query_with_capabilities,
    transform_delete_many_query_with_capabilities, transform_delete_query_with_capabilities,
    transform_find_many_query_with_capabilities, transform_find_one_query_with_capabilities,
    transform_update_many_query_with_capabilities, transform_update_query_with_capabilities,
    AdapterCapabilities, AdapterFuture, Count, Create, DbAdapter, DbRecord, DbSchema, Delete,
    DeleteMany, FindMany, FindOne, SchemaCreation, TransactionCallback, Update, UpdateMany,
};

/// Adapter wrapper that maps OpenAuth logical schema names to database names.
#[derive(Debug, Clone)]
pub struct SchemaAdapter<A> {
    schema: DbSchema,
    inner: A,
}

impl<A> SchemaAdapter<A> {
    pub fn new(schema: DbSchema, inner: A) -> Self {
        Self { schema, inner }
    }

    pub fn schema(&self) -> &DbSchema {
        &self.schema
    }

    pub fn inner(&self) -> &A {
        &self.inner
    }
}

impl<A> DbAdapter for SchemaAdapter<A>
where
    A: DbAdapter,
{
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.inner.capabilities()
    }

    fn create<'a>(&'a self, query: Create) -> AdapterFuture<'a, DbRecord> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_create_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.create(query).await
        })
    }

    fn find_one<'a>(&'a self, query: FindOne) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_find_one_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.find_one(query).await
        })
    }

    fn find_many<'a>(&'a self, query: FindMany) -> AdapterFuture<'a, Vec<DbRecord>> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_find_many_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.find_many(query).await
        })
    }

    fn count<'a>(&'a self, query: Count) -> AdapterFuture<'a, u64> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_count_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.count(query).await
        })
    }

    fn update<'a>(&'a self, query: Update) -> AdapterFuture<'a, Option<DbRecord>> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_update_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.update(query).await
        })
    }

    fn update_many<'a>(&'a self, query: UpdateMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_update_many_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.update_many(query).await
        })
    }

    fn delete<'a>(&'a self, query: Delete) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_delete_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.delete(query).await
        })
    }

    fn delete_many<'a>(&'a self, query: DeleteMany) -> AdapterFuture<'a, u64> {
        Box::pin(async move {
            let capabilities = self.inner.capabilities();
            let query =
                transform_delete_many_query_with_capabilities(&self.schema, &capabilities, query)?;
            self.inner.delete_many(query).await
        })
    }

    fn transaction<'a>(&'a self, callback: TransactionCallback<'a>) -> AdapterFuture<'a, ()> {
        callback(self)
    }

    fn create_schema<'a>(
        &'a self,
        _schema: &'a DbSchema,
        file: Option<&'a str>,
    ) -> AdapterFuture<'a, Option<SchemaCreation>> {
        self.inner.create_schema(&self.schema, file)
    }
}
