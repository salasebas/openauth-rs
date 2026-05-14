use openauth_core::db::{
    Connector, DbField, DbFieldType, DbTable, DbValue, Where, WhereMode, WhereOperator,
};
use openauth_core::error::OpenAuthError;
use sqlx::mysql::MySqlArguments;
use sqlx::Arguments;
use time::OffsetDateTime;

use super::errors::argument_error;
use super::support::{quote_identifier, resolve_field};

pub(super) fn where_sql(
    table: &DbTable,
    clauses: &[Where],
    args: &mut MySqlArguments,
) -> Result<String, OpenAuthError> {
    if clauses.is_empty() {
        return Ok(String::new());
    }

    let mut sql = String::from(" WHERE ");
    for (index, clause) in clauses.iter().enumerate() {
        if index > 0 {
            sql.push(' ');
            sql.push_str(match clause.connector {
                Connector::And => "AND",
                Connector::Or => "OR",
            });
            sql.push(' ');
        }
        sql.push_str(&clause_sql(table, clause, args)?);
    }
    Ok(sql)
}

pub(super) fn clause_sql(
    table: &DbTable,
    clause: &Where,
    args: &mut MySqlArguments,
) -> Result<String, OpenAuthError> {
    let (_, field) = resolve_field(table, &clause.field)?;
    let column = quote_identifier(&field.name)?;
    if clause.value == DbValue::Null {
        return Ok(match clause.operator {
            WhereOperator::Eq => format!("{column} IS NULL"),
            WhereOperator::Ne => format!("{column} IS NOT NULL"),
            _ => {
                return Err(OpenAuthError::Adapter(
                    "null only supports Eq and Ne operators".to_owned(),
                ))
            }
        });
    }

    match clause.operator {
        WhereOperator::Eq
        | WhereOperator::Ne
        | WhereOperator::Lt
        | WhereOperator::Lte
        | WhereOperator::Gt
        | WhereOperator::Gte => {
            let operator = match clause.operator {
                WhereOperator::Eq => "=",
                WhereOperator::Ne => "!=",
                WhereOperator::Lt => "<",
                WhereOperator::Lte => "<=",
                WhereOperator::Gt => ">",
                WhereOperator::Gte => ">=",
                _ => unreachable!("operator matched by outer arm"),
            };
            bind_value(args, field, &clause.value)?;
            Ok(format!("{column} {operator} ?"))
        }
        WhereOperator::In | WhereOperator::NotIn => {
            let placeholders = bind_array_values(args, field, &clause.value)?;
            let operator = if clause.operator == WhereOperator::In {
                "IN"
            } else {
                "NOT IN"
            };
            Ok(format!("{column} {operator} ({})", placeholders.join(", ")))
        }
        WhereOperator::Contains | WhereOperator::StartsWith | WhereOperator::EndsWith => {
            let DbValue::String(value) = &clause.value else {
                return Err(OpenAuthError::Adapter(
                    "string pattern operators require string values".to_owned(),
                ));
            };
            let pattern = match clause.operator {
                WhereOperator::Contains => format!("%{value}%"),
                WhereOperator::StartsWith => format!("{value}%"),
                WhereOperator::EndsWith => format!("%{value}"),
                _ => unreachable!("operator matched by outer arm"),
            };
            args.add(pattern).map_err(argument_error)?;
            if clause.mode == WhereMode::Insensitive {
                Ok(format!("LOWER({column}) LIKE LOWER(?)"))
            } else {
                Ok(format!("{column} LIKE ?"))
            }
        }
    }
}

pub(super) fn bind_array_values(
    args: &mut MySqlArguments,
    field: &DbField,
    value: &DbValue,
) -> Result<Vec<String>, OpenAuthError> {
    match value {
        DbValue::StringArray(values) => {
            for value in values {
                bind_value(args, field, &DbValue::String(value.clone()))?;
            }
            Ok(vec!["?".to_owned(); values.len()])
        }
        DbValue::NumberArray(values) => {
            for value in values {
                bind_value(args, field, &DbValue::Number(*value))?;
            }
            Ok(vec!["?".to_owned(); values.len()])
        }
        _ => Err(OpenAuthError::Adapter(
            "IN and NOT IN require array values".to_owned(),
        )),
    }
}

pub(super) fn bind_value(
    args: &mut MySqlArguments,
    field: &DbField,
    value: &DbValue,
) -> Result<(), OpenAuthError> {
    match value {
        DbValue::String(value) => args.add(value.clone()).map_err(argument_error),
        DbValue::Number(value) => args.add(*value).map_err(argument_error),
        DbValue::Boolean(value) => args.add(*value).map_err(argument_error),
        DbValue::Timestamp(value) => args.add(*value).map_err(argument_error),
        DbValue::Json(value) => args.add(value.clone()).map_err(argument_error),
        DbValue::StringArray(value) => args
            .add(serde_json::Value::Array(
                value
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ))
            .map_err(argument_error),
        DbValue::NumberArray(value) => args
            .add(serde_json::Value::Array(
                value.iter().copied().map(serde_json::Value::from).collect(),
            ))
            .map_err(argument_error),
        DbValue::Record(_) | DbValue::RecordArray(_) => Err(OpenAuthError::Adapter(
            "joined records cannot be bound as SQL values".to_owned(),
        )),
        DbValue::Null => match field.field_type {
            DbFieldType::String => args.add(Option::<String>::None).map_err(argument_error),
            DbFieldType::Number => args.add(Option::<i64>::None).map_err(argument_error),
            DbFieldType::Boolean => args.add(Option::<bool>::None).map_err(argument_error),
            DbFieldType::Timestamp => args
                .add(Option::<OffsetDateTime>::None)
                .map_err(argument_error),
            DbFieldType::Json | DbFieldType::StringArray | DbFieldType::NumberArray => args
                .add(Option::<serde_json::Value>::None)
                .map_err(argument_error),
        },
    }
}
