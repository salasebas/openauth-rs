use indexmap::IndexMap;
use openauth_core::db::{DbField, DbFieldType, DbTable, ForeignKey, OnDelete};
use openauth_core::plugin::PluginSchemaContribution;

pub fn oauth_application_schema() -> PluginSchemaContribution {
    PluginSchemaContribution::table(
        "oauthApplication",
        table(
            "oauth_applications",
            30,
            [
                ("name", field("name", DbFieldType::String)),
                ("icon", field("icon", DbFieldType::String).optional()),
                (
                    "metadata",
                    field("metadata", DbFieldType::String).optional(),
                ),
                ("clientId", field("clientId", DbFieldType::String).unique()),
                (
                    "clientSecret",
                    field("clientSecret", DbFieldType::String).optional(),
                ),
                ("redirectUrls", field("redirectUrls", DbFieldType::String)),
                ("type", field("type", DbFieldType::String)),
                (
                    "authenticationScheme",
                    field("authenticationScheme", DbFieldType::String),
                ),
                (
                    "disabled",
                    field("disabled", DbFieldType::Boolean).optional(),
                ),
                (
                    "userId",
                    field("userId", DbFieldType::String)
                        .optional()
                        .indexed()
                        .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                ),
                ("createdAt", field("createdAt", DbFieldType::Timestamp)),
                ("updatedAt", field("updatedAt", DbFieldType::Timestamp)),
            ],
        ),
    )
}

pub fn oauth_access_token_schema() -> PluginSchemaContribution {
    PluginSchemaContribution::table(
        "oauthAccessToken",
        table(
            "oauth_access_tokens",
            31,
            [
                (
                    "accessToken",
                    field("accessToken", DbFieldType::String).unique(),
                ),
                (
                    "refreshToken",
                    field("refreshToken", DbFieldType::String).unique(),
                ),
                (
                    "accessTokenExpiresAt",
                    field("accessTokenExpiresAt", DbFieldType::Timestamp),
                ),
                (
                    "refreshTokenExpiresAt",
                    field("refreshTokenExpiresAt", DbFieldType::Timestamp),
                ),
                (
                    "clientId",
                    field("clientId", DbFieldType::String)
                        .indexed()
                        .references(ForeignKey::new(
                            "oauth_applications",
                            "clientId",
                            OnDelete::Cascade,
                        )),
                ),
                (
                    "userId",
                    field("userId", DbFieldType::String)
                        .optional()
                        .indexed()
                        .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                ),
                ("scopes", field("scopes", DbFieldType::String)),
                ("createdAt", field("createdAt", DbFieldType::Timestamp)),
                ("updatedAt", field("updatedAt", DbFieldType::Timestamp)),
            ],
        ),
    )
}

pub fn oauth_consent_schema() -> PluginSchemaContribution {
    PluginSchemaContribution::table(
        "oauthConsent",
        table(
            "oauth_consents",
            32,
            [
                (
                    "clientId",
                    field("clientId", DbFieldType::String)
                        .indexed()
                        .references(ForeignKey::new(
                            "oauth_applications",
                            "clientId",
                            OnDelete::Cascade,
                        )),
                ),
                (
                    "userId",
                    field("userId", DbFieldType::String)
                        .indexed()
                        .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                ),
                ("scopes", field("scopes", DbFieldType::String)),
                ("createdAt", field("createdAt", DbFieldType::Timestamp)),
                ("updatedAt", field("updatedAt", DbFieldType::Timestamp)),
                ("consentGiven", field("consentGiven", DbFieldType::Boolean)),
            ],
        ),
    )
}

fn table<const N: usize>(name: &str, order: u16, fields: [(&str, DbField); N]) -> DbTable {
    DbTable {
        name: name.to_owned(),
        order: Some(order),
        fields: fields
            .into_iter()
            .map(|(name, field)| (name.to_owned(), field))
            .collect::<IndexMap<_, _>>(),
    }
}

fn field(name: &str, field_type: DbFieldType) -> DbField {
    DbField::new(name, field_type)
}
