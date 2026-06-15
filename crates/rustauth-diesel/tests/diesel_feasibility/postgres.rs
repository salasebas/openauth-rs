use diesel::deserialize::QueryableByName;
use diesel::pg::Pg;
use diesel::row::NamedRow;
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use rustauth_core::db::{DbFieldType, DbRecord, DbValue};
use rustauth_core::error::RustAuthError;
use rustauth_diesel::{bind_postgres_params, decode_postgres_row, RowDecodeStrategy};
use serde_json::json;
use uuid::Uuid;

use crate::support::{
    adapter_error, field, param, postgres_database_url, sample_timestamp, selected, uuid_field,
};

struct ScalarDecoded(DbRecord);

impl QueryableByName<Pg> for ScalarDecoded {
    fn build<'a>(row: &impl NamedRow<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let selection = [
            selected("text", field("text", DbFieldType::String), "col_text"),
            selected("number", field("number", DbFieldType::Number), "col_number"),
            selected(
                "boolean",
                field("boolean", DbFieldType::Boolean),
                "col_bool",
            ),
            selected(
                "null_text",
                field("null_text", DbFieldType::String),
                "col_null",
            ),
            selected(
                "timestamp",
                field("timestamp", DbFieldType::Timestamp),
                "col_ts",
            ),
            selected("uuid", uuid_field("uuid"), "col_uuid"),
        ];
        decode_postgres_row(row, &selection)
            .map(ScalarDecoded)
            .map_err(|error| error.to_string().into())
    }
}

struct JsonArrayDecoded(DbRecord);

impl QueryableByName<Pg> for JsonArrayDecoded {
    fn build<'a>(row: &impl NamedRow<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let selection = [
            selected("json", field("json", DbFieldType::Json), "col_json"),
            selected(
                "strings",
                field("strings", DbFieldType::StringArray),
                "col_strings",
            ),
            selected(
                "numbers",
                field("numbers", DbFieldType::NumberArray),
                "col_numbers",
            ),
        ];
        decode_postgres_row(row, &selection)
            .map(JsonArrayDecoded)
            .map_err(|error| error.to_string().into())
    }
}

struct JoinAliasDecoded(DbRecord);

impl QueryableByName<Pg> for JoinAliasDecoded {
    fn build<'a>(row: &impl NamedRow<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let selection = [
            selected(
                "userName",
                field("user_name", DbFieldType::String),
                "user_name",
            ),
            selected(
                "orgName",
                field("org_name", DbFieldType::String),
                "org_name",
            ),
        ];
        decode_postgres_row(row, &selection)
            .map(JoinAliasDecoded)
            .map_err(|error| error.to_string().into())
    }
}

async fn connection() -> Result<AsyncPgConnection, RustAuthError> {
    let database_url = postgres_database_url();
    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(|error| adapter_error("diesel postgres", &database_url, error))
}

#[tokio::test]
async fn postgres_row_decode_strategy_is_direct_alias() {
    assert_eq!(RowDecodeStrategy::SELECTED, RowDecodeStrategy::DirectAlias);
}

#[tokio::test]
async fn postgres_bind_and_decode_scalar_values() -> Result<(), RustAuthError> {
    let text_field = field("text", DbFieldType::String);
    let number_field = field("number", DbFieldType::Number);
    let bool_field = field("boolean", DbFieldType::Boolean);
    let null_field = field("null_text", DbFieldType::String);
    let ts_field = field("timestamp", DbFieldType::Timestamp);
    let uuid_id = uuid_field("uuid");
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")
        .map_err(|error| RustAuthError::Adapter(format!("invalid test uuid: {error}")))?;
    let timestamp = sample_timestamp();

    let params = vec![
        param(&text_field, DbValue::String("alpha".to_owned())),
        param(&number_field, DbValue::Number(42)),
        param(&bool_field, DbValue::Boolean(true)),
        param(&null_field, DbValue::Null),
        param(&ts_field, DbValue::Timestamp(timestamp)),
        param(&uuid_id, DbValue::String(uuid.to_string())),
    ];

    let sql = "SELECT \
        $1::text AS col_text, \
        $2::bigint AS col_number, \
        $3::boolean AS col_bool, \
        $4::text AS col_null, \
        $5::timestamptz AS col_ts, \
        $6::uuid AS col_uuid";

    let mut conn = connection().await?;
    let query = bind_postgres_params(sql, &params)?;
    let ScalarDecoded(record) = query
        .get_result(&mut conn)
        .await
        .map_err(|error| RustAuthError::Adapter(format!("postgres scalar query: {error}")))?;

    assert_eq!(
        record.get("text"),
        Some(&DbValue::String("alpha".to_owned()))
    );
    assert_eq!(record.get("number"), Some(&DbValue::Number(42)));
    assert_eq!(record.get("boolean"), Some(&DbValue::Boolean(true)));
    assert_eq!(record.get("null_text"), Some(&DbValue::Null));
    assert_eq!(
        record.get("timestamp"),
        Some(&DbValue::Timestamp(timestamp))
    );
    assert_eq!(record.get("uuid"), Some(&DbValue::String(uuid.to_string())));

    Ok(())
}

#[tokio::test]
async fn postgres_bind_and_decode_json_and_arrays() -> Result<(), RustAuthError> {
    let json_field = field("json", DbFieldType::Json);
    let strings_field = field("strings", DbFieldType::StringArray);
    let numbers_field = field("numbers", DbFieldType::NumberArray);
    let payload = json!({"role": "admin", "active": true});

    let params = vec![
        param(&json_field, DbValue::Json(payload.clone())),
        param(
            &strings_field,
            DbValue::StringArray(vec!["read".to_owned(), "write".to_owned()]),
        ),
        param(&numbers_field, DbValue::NumberArray(vec![1, 2, 3])),
    ];

    let sql =
        "SELECT $1::jsonb AS col_json, $2::text[] AS col_strings, $3::bigint[] AS col_numbers";

    let mut conn = connection().await?;
    let query = bind_postgres_params(sql, &params)?;
    let JsonArrayDecoded(record) = query
        .get_result(&mut conn)
        .await
        .map_err(|error| RustAuthError::Adapter(format!("postgres json/array query: {error}")))?;

    assert_eq!(record.get("json"), Some(&DbValue::Json(payload)));
    assert_eq!(
        record.get("strings"),
        Some(&DbValue::StringArray(vec![
            "read".to_owned(),
            "write".to_owned()
        ]))
    );
    assert_eq!(
        record.get("numbers"),
        Some(&DbValue::NumberArray(vec![1, 2, 3]))
    );

    Ok(())
}

#[tokio::test]
async fn postgres_joined_aliases_preserve_logical_names() -> Result<(), RustAuthError> {
    let user_field = field("user_name", DbFieldType::String);
    let org_field = field("org_name", DbFieldType::String);
    let params = vec![
        param(&user_field, DbValue::String("Ada".to_owned())),
        param(&org_field, DbValue::String("RustAuth".to_owned())),
    ];

    let sql = "SELECT $1::text AS user_name, $2::text AS org_name";

    let mut conn = connection().await?;
    let query = bind_postgres_params(sql, &params)?;
    let JoinAliasDecoded(record) = query
        .get_result(&mut conn)
        .await
        .map_err(|error| RustAuthError::Adapter(format!("postgres join alias query: {error}")))?;

    assert_eq!(
        record.get("userName"),
        Some(&DbValue::String("Ada".to_owned()))
    );
    assert_eq!(
        record.get("orgName"),
        Some(&DbValue::String("RustAuth".to_owned()))
    );

    Ok(())
}
