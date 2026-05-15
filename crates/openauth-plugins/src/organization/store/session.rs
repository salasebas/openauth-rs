use openauth_core::db::{DbValue, FindOne, Update, Where};
use openauth_core::error::OpenAuthError;

use super::OrganizationStore;

impl<'a> OrganizationStore<'a> {
    pub async fn set_active_organization(
        &self,
        token: &str,
        organization_id: Option<&str>,
    ) -> Result<(), OpenAuthError> {
        self.adapter()
            .update(
                Update::new("session")
                    .where_clause(Where::new("token", DbValue::String(token.to_owned())))
                    .data(
                        "activeOrganizationId",
                        organization_id
                            .map(|value| DbValue::String(value.to_owned()))
                            .unwrap_or(DbValue::Null),
                    ),
            )
            .await?;
        Ok(())
    }

    pub async fn active_organization_id(
        &self,
        token: &str,
    ) -> Result<Option<String>, OpenAuthError> {
        let Some(record) = self
            .adapter()
            .find_one(
                FindOne::new("session")
                    .where_clause(Where::new("token", DbValue::String(token.to_owned()))),
            )
            .await?
        else {
            return Ok(None);
        };
        crate::organization::models::optional_string(&record, "activeOrganizationId").and_then(
            |value| match value {
                Some(value) => Ok(Some(value)),
                None => {
                    crate::organization::models::optional_string(&record, "active_organization_id")
                }
            },
        )
    }
}
