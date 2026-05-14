use openauth_core::crypto::random::generate_random_string;
use openauth_core::db::{Count, Create, DbValue, Delete, FindMany, FindOne, Update, Where};
use openauth_core::error::OpenAuthError;
use time::OffsetDateTime;

use super::{id_where, OrganizationStore, ID_LENGTH};
use crate::organization::record::organization_role_from_record;
use crate::organization::OrganizationRoleRecord;

impl<'a> OrganizationStore<'a> {
    pub async fn create_organization_role(
        &self,
        organization_id: &str,
        role: &str,
        permission: serde_json::Value,
    ) -> Result<OrganizationRoleRecord, OpenAuthError> {
        let now = OffsetDateTime::now_utc();
        let record = self
            .adapter()
            .create(
                Create::new("organization_role")
                    .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
                    .data(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    )
                    .data("role", DbValue::String(role.to_owned()))
                    .data("permission", DbValue::Json(permission))
                    .data("created_at", DbValue::Timestamp(now))
                    .data("updated_at", DbValue::Timestamp(now))
                    .force_allow_id(),
            )
            .await?;
        organization_role_from_record(&record)
    }

    pub async fn organization_role_by_id(
        &self,
        id: &str,
    ) -> Result<Option<OrganizationRoleRecord>, OpenAuthError> {
        self.adapter()
            .find_one(FindOne::new("organization_role").where_clause(id_where(id)))
            .await?
            .map(|record| organization_role_from_record(&record))
            .transpose()
    }

    pub async fn organization_role_by_name(
        &self,
        organization_id: &str,
        role: &str,
    ) -> Result<Option<OrganizationRoleRecord>, OpenAuthError> {
        self.adapter()
            .find_one(
                FindOne::new("organization_role")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .where_clause(Where::new("role", DbValue::String(role.to_owned()))),
            )
            .await?
            .map(|record| organization_role_from_record(&record))
            .transpose()
    }

    pub async fn organization_roles(
        &self,
        organization_id: &str,
    ) -> Result<Vec<OrganizationRoleRecord>, OpenAuthError> {
        self.adapter()
            .find_many(FindMany::new("organization_role").where_clause(Where::new(
                "organization_id",
                DbValue::String(organization_id.to_owned()),
            )))
            .await?
            .iter()
            .map(organization_role_from_record)
            .collect()
    }

    pub async fn count_organization_roles(
        &self,
        organization_id: &str,
    ) -> Result<u64, OpenAuthError> {
        self.adapter()
            .count(Count::new("organization_role").where_clause(Where::new(
                "organization_id",
                DbValue::String(organization_id.to_owned()),
            )))
            .await
    }

    pub async fn update_organization_role(
        &self,
        id: &str,
        role: Option<&str>,
        permission: Option<serde_json::Value>,
    ) -> Result<Option<OrganizationRoleRecord>, OpenAuthError> {
        let mut update = Update::new("organization_role")
            .where_clause(id_where(id))
            .data("updated_at", DbValue::Timestamp(OffsetDateTime::now_utc()));
        if let Some(role) = role {
            update = update.data("role", DbValue::String(role.to_owned()));
        }
        if let Some(permission) = permission {
            update = update.data("permission", DbValue::Json(permission));
        }
        self.adapter()
            .update(update)
            .await?
            .map(|record| organization_role_from_record(&record))
            .transpose()
    }

    pub async fn delete_organization_role(&self, id: &str) -> Result<(), OpenAuthError> {
        self.adapter()
            .delete(Delete::new("organization_role").where_clause(id_where(id)))
            .await
    }
}
