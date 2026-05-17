use openauth_core::context::create_auth_context;
use openauth_core::db::DbAdapter;
use openauth_core::options::OpenAuthOptions;
use openauth_passkey::{passkey, PasskeyOptions};
use openauth_sqlx::SqliteAdapter;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn sqlite_schema_migration_creates_passkeys_table_and_columns(
) -> Result<(), Box<dyn std::error::Error>> {
    let context = create_auth_context(OpenAuthOptions {
        plugins: vec![passkey(PasskeyOptions::default())],
        secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
        ..OpenAuthOptions::default()
    })?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    let adapter = SqliteAdapter::with_schema(pool.clone(), context.db_schema.clone());

    adapter.create_schema(&context.db_schema, None).await?;

    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'passkeys'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(table_count, 1);

    let columns = sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('passkeys')")
        .fetch_all(&pool)
        .await?;
    assert!(columns.iter().any(|column| column == "credential_id"));
    assert!(columns.iter().any(|column| column == "webauthn_credential"));
    assert!(columns
        .iter()
        .all(|column| !column.contains(char::is_uppercase)));

    Ok(())
}
