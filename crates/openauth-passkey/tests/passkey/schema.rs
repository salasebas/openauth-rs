use openauth_core::context::create_auth_context;
use openauth_core::db::DbFieldType;
use openauth_core::options::OpenAuthOptions;
use openauth_passkey::{passkey, PasskeyOptions};

#[test]
fn passkey_plugin_registers_snake_case_plural_schema() -> Result<(), Box<dyn std::error::Error>> {
    let context = create_auth_context(OpenAuthOptions {
        plugins: vec![passkey(PasskeyOptions::default())],
        secret: Some("secret-a-at-least-32-chars-long!!".to_owned()),
        ..OpenAuthOptions::default()
    })?;

    let table = context
        .db_schema
        .table("passkey")
        .ok_or("missing passkey table")?;
    assert_eq!(table.name, "passkeys");

    let public_key = context.db_schema.field("passkey", "public_key")?;
    assert_eq!(public_key.name, "public_key");
    assert_eq!(public_key.field_type, DbFieldType::String);
    assert!(public_key.required);

    let credential_id = context.db_schema.field("passkey", "credential_id")?;
    assert_eq!(credential_id.name, "credential_id");
    assert!(credential_id.index);

    let user_id = context.db_schema.field("passkey", "user_id")?;
    assert_eq!(user_id.name, "user_id");
    assert!(user_id.index);
    assert!(user_id.foreign_key.is_some());

    let credential = context.db_schema.field("passkey", "webauthn_credential")?;
    assert_eq!(credential.field_type, DbFieldType::Json);
    assert!(!credential.returned);

    assert_eq!(
        context
            .plugin_error_codes
            .get("CHALLENGE_NOT_FOUND")
            .map(|code| code.message.as_str()),
        Some("Challenge not found")
    );

    Ok(())
}
