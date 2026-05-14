use openauth_core::db::{DbField, DbRecord, DbSchema, DbTable};
use openauth_core::error::OpenAuthError;

pub(super) fn select_fields<'a>(
    table: &'a DbTable,
    select: &'a [String],
) -> Result<Vec<(&'a str, &'a DbField)>, OpenAuthError> {
    if select.is_empty() {
        return Ok(table
            .fields
            .iter()
            .map(|(logical_name, field)| (logical_name.as_str(), field))
            .collect());
    }

    select
        .iter()
        .map(|field| resolve_field(table, field))
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn select_record(record: DbRecord, select: &[String]) -> DbRecord {
    if select.is_empty() {
        return record;
    }
    select
        .iter()
        .filter_map(|field| {
            record
                .get(field)
                .cloned()
                .map(|value| (field.clone(), value))
        })
        .collect()
}

pub(super) fn resolve_table<'a>(
    schema: &'a DbSchema,
    model: &str,
) -> Result<&'a DbTable, OpenAuthError> {
    schema
        .tables()
        .find_map(|(logical_name, table)| {
            (logical_name == model || table.name == model).then_some(table)
        })
        .ok_or_else(|| OpenAuthError::TableNotFound {
            table: model.to_owned(),
        })
}

pub(super) fn resolve_table_with_logical<'a>(
    schema: &'a DbSchema,
    model: &str,
) -> Result<(&'a str, &'a DbTable), OpenAuthError> {
    schema
        .tables()
        .find(|(logical_name, table)| *logical_name == model || table.name == model)
        .ok_or_else(|| OpenAuthError::TableNotFound {
            table: model.to_owned(),
        })
}

pub(super) fn resolve_field<'a>(
    table: &'a DbTable,
    field: &str,
) -> Result<(&'a str, &'a DbField), OpenAuthError> {
    table
        .fields
        .iter()
        .find_map(|(logical_name, metadata)| {
            (logical_name == field || metadata.name == field)
                .then_some((logical_name.as_str(), metadata))
        })
        .ok_or_else(|| OpenAuthError::FieldNotFound {
            table: table.name.clone(),
            field: field.to_owned(),
        })
}

pub(super) fn quote_identifier(identifier: &str) -> Result<String, OpenAuthError> {
    validate_identifier(identifier)?;
    Ok(format!("\"{identifier}\""))
}

pub(super) fn sanitize_identifier(identifier: &str) -> Result<String, OpenAuthError> {
    let sanitized = identifier
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    validate_identifier(&sanitized)?;
    Ok(sanitized)
}

pub(super) fn validate_identifier(identifier: &str) -> Result<(), OpenAuthError> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(OpenAuthError::Adapter(
            "postgres identifier cannot be empty".to_owned(),
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(invalid_identifier(identifier));
    }
    if chars.any(|character| !(character.is_ascii_alphanumeric() || character == '_')) {
        return Err(invalid_identifier(identifier));
    }
    Ok(())
}

pub(super) fn invalid_identifier(identifier: &str) -> OpenAuthError {
    OpenAuthError::Adapter(format!("invalid postgres identifier `{identifier}`"))
}
