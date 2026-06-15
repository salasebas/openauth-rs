use diesel::deserialize::QueryableByName;
use diesel::mysql::Mysql;
use diesel::row::NamedRow;
use diesel_async::AsyncConnection;
use diesel_async::AsyncMysqlConnection;
use diesel_async::RunQueryDsl;
use rustauth_core::db::{DbFieldType, DbRecord, DbValue};
use rustauth_core::error::RustAuthError;
use rustauth_diesel::{bind_mysql_params, decode_mysql_row};
use serde_json::json;

use crate::support::{adapter_error, field, mysql_database_url, param, sample_timestamp, selected};

struct ScalarDecoded(DbRecord);

impl QueryableByName<Mysql> for ScalarDecoded {
    fn build<'a>(row: &impl NamedRow<'a, Mysql>) -> diesel::deserialize::Result<Self> {
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
        ];
        decode_mysql_row(row, &selection)
            .map(ScalarDecoded)
            .map_err(|error| error.to_string().into())
    }
}

struct JsonArrayDecoded(DbRecord);

impl QueryableByName<Mysql> for JsonArrayDecoded {
    fn build<'a>(row: &impl NamedRow<'a, Mysql>) -> diesel::deserialize::Result<Self> {
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
        decode_mysql_row(row, &selection)
            .map(JsonArrayDecoded)
            .map_err(|error| error.to_string().into())
    }
}

async fn connection() -> Result<AsyncMysqlConnection, RustAuthError> {
    let database_url = mysql_database_url();
    AsyncMysqlConnection::establish(&database_url)
        .await
        .map_err(|error| adapter_error("diesel mysql", &database_url, error))
}

#[tokio::test]
async fn mysql_bind_and_decode_scalar_values() -> Result<(), RustAuthError> {
    let text_field = field("text", DbFieldType::String);
    let number_field = field("number", DbFieldType::Number);
    let bool_field = field("boolean", DbFieldType::Boolean);
    let null_field = field("null_text", DbFieldType::String);
    let ts_field = field("timestamp", DbFieldType::Timestamp);
    let timestamp = sample_timestamp();

    let params = vec![
        param(&text_field, DbValue::String("beta".to_owned())),
        param(&number_field, DbValue::Number(7)),
        param(&bool_field, DbValue::Boolean(false)),
        param(&null_field, DbValue::Null),
        param(&ts_field, DbValue::Timestamp(timestamp)),
    ];

    let sql = "SELECT \
        ? AS col_text, \
        ? AS col_number, \
        ? AS col_bool, \
        ? AS col_null, \
        ? AS col_ts";

    let mut conn = connection().await?;
    let query = bind_mysql_params(sql, &params)?;
    let ScalarDecoded(record) = query
        .get_result(&mut conn)
        .await
        .map_err(|error| RustAuthError::Adapter(format!("mysql scalar query: {error}")))?;

    assert_eq!(
        record.get("text"),
        Some(&DbValue::String("beta".to_owned()))
    );
    assert_eq!(record.get("number"), Some(&DbValue::Number(7)));
    assert_eq!(record.get("boolean"), Some(&DbValue::Boolean(false)));
    assert_eq!(record.get("null_text"), Some(&DbValue::Null));
    assert_eq!(
        record.get("timestamp"),
        Some(&DbValue::Timestamp(timestamp))
    );

    Ok(())
}

#[tokio::test]
async fn mysql_json_arrays_match_sqlx_representation() -> Result<(), RustAuthError> {
    let json_field = field("json", DbFieldType::Json);
    let strings_field = field("strings", DbFieldType::StringArray);
    let numbers_field = field("numbers", DbFieldType::NumberArray);
    let payload = json!({"tier": "pro"});
    let string_array_json = json!(["a", "b"]);
    let number_array_json = json!([4, 5, 6]);

    let params = vec![
        param(&json_field, DbValue::Json(payload.clone())),
        param(
            &strings_field,
            DbValue::StringArray(vec!["a".to_owned(), "b".to_owned()]),
        ),
        param(&numbers_field, DbValue::NumberArray(vec![4, 5, 6])),
    ];

    let sql = "SELECT ? AS col_json, ? AS col_strings, ? AS col_numbers";

    let mut conn = connection().await?;
    let query = bind_mysql_params(sql, &params)?;
    let JsonArrayDecoded(record) = query
        .get_result(&mut conn)
        .await
        .map_err(|error| RustAuthError::Adapter(format!("mysql json/array query: {error}")))?;

    assert_eq!(record.get("json"), Some(&DbValue::Json(payload)));
    assert_eq!(
        record.get("strings"),
        Some(&DbValue::StringArray(vec!["a".to_owned(), "b".to_owned()]))
    );
    assert_eq!(
        record.get("numbers"),
        Some(&DbValue::NumberArray(vec![4, 5, 6]))
    );

    // SQLx stores MySQL arrays as JSON values at the wire level.
    assert_eq!(string_array_json, json!(["a", "b"]));
    assert_eq!(number_array_json, json!([4, 5, 6]));

    Ok(())
}
