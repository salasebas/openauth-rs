use indexmap::IndexMap;
use openauth_core::db::{DbField, DbFieldType, DbTable, ForeignKey, OnDelete};
use openauth_core::plugin::PluginSchemaContribution;

use super::OrganizationOptions;

pub fn schema_contributions(options: &OrganizationOptions) -> Vec<PluginSchemaContribution> {
    let mut contributions = vec![
        PluginSchemaContribution::table(
            "organization",
            table(
                "organizations",
                Some(20),
                [
                    ("id", DbField::new("id", DbFieldType::String)),
                    ("name", DbField::new("name", DbFieldType::String)),
                    (
                        "slug",
                        DbField::new("slug", DbFieldType::String).unique().indexed(),
                    ),
                    ("logo", DbField::new("logo", DbFieldType::String).optional()),
                    (
                        "metadata",
                        DbField::new("metadata", DbFieldType::Json).optional(),
                    ),
                    (
                        "created_at",
                        DbField::new("created_at", DbFieldType::Timestamp),
                    ),
                    (
                        "updated_at",
                        DbField::new("updated_at", DbFieldType::Timestamp).optional(),
                    ),
                ],
            ),
        ),
        PluginSchemaContribution::table(
            "member",
            table(
                "members",
                Some(21),
                [
                    ("id", DbField::new("id", DbFieldType::String)),
                    (
                        "organization_id",
                        DbField::new("organization_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("organizations", "id", OnDelete::Cascade)),
                    ),
                    (
                        "user_id",
                        DbField::new("user_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                    ),
                    ("role", DbField::new("role", DbFieldType::String)),
                    (
                        "created_at",
                        DbField::new("created_at", DbFieldType::Timestamp),
                    ),
                ],
            ),
        ),
        PluginSchemaContribution::table(
            "invitation",
            table(
                "invitations",
                Some(22),
                [
                    ("id", DbField::new("id", DbFieldType::String)),
                    (
                        "organization_id",
                        DbField::new("organization_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("organizations", "id", OnDelete::Cascade)),
                    ),
                    (
                        "email",
                        DbField::new("email", DbFieldType::String).indexed(),
                    ),
                    ("role", DbField::new("role", DbFieldType::String)),
                    (
                        "status",
                        DbField::new("status", DbFieldType::String).indexed(),
                    ),
                    (
                        "expires_at",
                        DbField::new("expires_at", DbFieldType::Timestamp),
                    ),
                    (
                        "created_at",
                        DbField::new("created_at", DbFieldType::Timestamp),
                    ),
                    (
                        "inviter_id",
                        DbField::new("inviter_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                    ),
                    (
                        "team_id",
                        DbField::new("team_id", DbFieldType::String).optional(),
                    ),
                ],
            ),
        ),
        PluginSchemaContribution::field(
            "session",
            "active_organization_id",
            DbField::new("active_organization_id", DbFieldType::String).optional(),
        ),
    ];
    if options.teams.enabled {
        contributions.extend(team_schema_contributions());
    }
    if options.dynamic_access_control.enabled {
        contributions.push(organization_role_schema_contribution());
    }
    contributions
}

fn team_schema_contributions() -> Vec<PluginSchemaContribution> {
    vec![
        PluginSchemaContribution::table(
            "team",
            table(
                "teams",
                Some(23),
                [
                    ("id", DbField::new("id", DbFieldType::String)),
                    ("name", DbField::new("name", DbFieldType::String)),
                    (
                        "organization_id",
                        DbField::new("organization_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("organizations", "id", OnDelete::Cascade)),
                    ),
                    (
                        "created_at",
                        DbField::new("created_at", DbFieldType::Timestamp),
                    ),
                    (
                        "updated_at",
                        DbField::new("updated_at", DbFieldType::Timestamp).optional(),
                    ),
                ],
            ),
        ),
        PluginSchemaContribution::table(
            "team_member",
            table(
                "team_members",
                Some(24),
                [
                    ("id", DbField::new("id", DbFieldType::String)),
                    (
                        "team_id",
                        DbField::new("team_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("teams", "id", OnDelete::Cascade)),
                    ),
                    (
                        "user_id",
                        DbField::new("user_id", DbFieldType::String)
                            .indexed()
                            .references(ForeignKey::new("users", "id", OnDelete::Cascade)),
                    ),
                    (
                        "created_at",
                        DbField::new("created_at", DbFieldType::Timestamp),
                    ),
                ],
            ),
        ),
        PluginSchemaContribution::field(
            "session",
            "active_team_id",
            DbField::new("active_team_id", DbFieldType::String).optional(),
        ),
    ]
}

fn organization_role_schema_contribution() -> PluginSchemaContribution {
    PluginSchemaContribution::table(
        "organization_role",
        table(
            "organization_roles",
            Some(25),
            [
                ("id", DbField::new("id", DbFieldType::String)),
                (
                    "organization_id",
                    DbField::new("organization_id", DbFieldType::String)
                        .indexed()
                        .references(ForeignKey::new("organizations", "id", OnDelete::Cascade)),
                ),
                ("role", DbField::new("role", DbFieldType::String).indexed()),
                ("permission", DbField::new("permission", DbFieldType::Json)),
                (
                    "created_at",
                    DbField::new("created_at", DbFieldType::Timestamp),
                ),
                (
                    "updated_at",
                    DbField::new("updated_at", DbFieldType::Timestamp).optional(),
                ),
            ],
        ),
    )
}

fn table<const N: usize>(name: &str, order: Option<u16>, fields: [(&str, DbField); N]) -> DbTable {
    DbTable {
        name: name.to_owned(),
        fields: fields
            .into_iter()
            .map(|(logical_name, field)| (logical_name.to_owned(), field))
            .collect::<IndexMap<_, _>>(),
        order,
    }
}
