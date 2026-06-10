use openauth_core::db::{auth_schema, AuthSchemaOptions, DbAdapter, RateLimitStorage};
use openauth_deadpool_postgres::DeadpoolPostgresStores;

#[tokio::main]
async fn main() -> Result<(), openauth_core::error::OpenAuthError> {
    let schema = auth_schema(AuthSchemaOptions {
        rate_limit_storage: RateLimitStorage::Database,
        ..AuthSchemaOptions::default()
    });
    let stores = DeadpoolPostgresStores::connect_with_schema(
        "postgres://user:password@localhost/openauth",
        schema.clone(),
    )
    .await?;

    stores.adapter_ref().run_migrations(&schema).await?;
    let _options = stores.apply_to_options(openauth_core::options::OpenAuthOptions::default());
    Ok(())
}
