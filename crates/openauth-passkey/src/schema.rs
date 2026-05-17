use indexmap::IndexMap;
use openauth_core::db::{DbField, DbFieldType, DbTable, ForeignKey, OnDelete};
use openauth_core::plugin::PluginSchemaContribution;

pub fn contributions(table_name: &str) -> Vec<PluginSchemaContribution> {
    vec![PluginSchemaContribution::table(
        "passkey",
        passkey_table(table_name),
    )]
}

fn passkey_table(name: &str) -> DbTable {
    let mut fields = IndexMap::new();
    fields.insert("id".to_owned(), DbField::new("id", DbFieldType::String));
    fields.insert(
        "name".to_owned(),
        DbField::new("name", DbFieldType::String).optional(),
    );
    fields.insert(
        "public_key".to_owned(),
        DbField::new("public_key", DbFieldType::String),
    );
    fields.insert(
        "user_id".to_owned(),
        DbField::new("user_id", DbFieldType::String)
            .indexed()
            .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
    );
    fields.insert(
        "credential_id".to_owned(),
        DbField::new("credential_id", DbFieldType::String).indexed(),
    );
    fields.insert(
        "counter".to_owned(),
        DbField::new("counter", DbFieldType::Number),
    );
    fields.insert(
        "device_type".to_owned(),
        DbField::new("device_type", DbFieldType::String),
    );
    fields.insert(
        "backed_up".to_owned(),
        DbField::new("backed_up", DbFieldType::Boolean),
    );
    fields.insert(
        "transports".to_owned(),
        DbField::new("transports", DbFieldType::String).optional(),
    );
    fields.insert(
        "created_at".to_owned(),
        DbField::new("created_at", DbFieldType::Timestamp)
            .optional()
            .generated(),
    );
    fields.insert(
        "aaguid".to_owned(),
        DbField::new("aaguid", DbFieldType::String).optional(),
    );
    fields.insert(
        "webauthn_credential".to_owned(),
        DbField::new("webauthn_credential", DbFieldType::Json).hidden(),
    );

    DbTable {
        name: name.to_owned(),
        fields,
        order: Some(20),
    }
}
