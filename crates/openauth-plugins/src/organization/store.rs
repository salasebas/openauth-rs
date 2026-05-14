use openauth_core::crypto::random::generate_random_string;
use openauth_core::db::{
    Count, Create, DbAdapter, DbValue, Delete, DeleteMany, FindMany, FindOne, Sort, SortDirection,
    Update, Where,
};
use openauth_core::error::OpenAuthError;
use time::OffsetDateTime;

use super::models::{
    optional_string, required_string, Invitation, InvitationStatus, Member, Organization,
};
use super::record::{
    invitation_from_record, member_from_record, organization_from_record, user_from_record,
};

const ID_LENGTH: usize = 32;

pub struct OrganizationStore<'a> {
    adapter: &'a dyn DbAdapter,
}

impl<'a> OrganizationStore<'a> {
    pub fn new(adapter: &'a dyn DbAdapter) -> Self {
        Self { adapter }
    }

    pub fn adapter(&self) -> &'a dyn DbAdapter {
        self.adapter
    }

    pub async fn create_organization(
        &self,
        name: String,
        slug: String,
        logo: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Organization, OpenAuthError> {
        let now = OffsetDateTime::now_utc();
        let mut query = Create::new("organization")
            .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
            .data("name", DbValue::String(name))
            .data("slug", DbValue::String(slug))
            .data("created_at", DbValue::Timestamp(now))
            .data("updated_at", DbValue::Null)
            .force_allow_id();
        query = query.data("logo", option_string(logo));
        query = query.data(
            "metadata",
            metadata.map(DbValue::Json).unwrap_or(DbValue::Null),
        );
        organization_from_record(&self.adapter.create(query).await?)
    }

    pub async fn organization_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<Organization>, OpenAuthError> {
        self.adapter
            .find_one(
                FindOne::new("organization")
                    .where_clause(Where::new("slug", DbValue::String(slug.to_owned()))),
            )
            .await?
            .map(|record| organization_from_record(&record))
            .transpose()
    }

    pub async fn organization_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Organization>, OpenAuthError> {
        self.adapter
            .find_one(FindOne::new("organization").where_clause(id_where(id)))
            .await?
            .map(|record| organization_from_record(&record))
            .transpose()
    }

    pub async fn update_organization(
        &self,
        id: &str,
        data: OrganizationUpdate,
    ) -> Result<Option<Organization>, OpenAuthError> {
        let mut query = Update::new("organization")
            .where_clause(id_where(id))
            .data("updated_at", DbValue::Timestamp(OffsetDateTime::now_utc()));
        if let Some(name) = data.name {
            query = query.data("name", DbValue::String(name));
        }
        if let Some(slug) = data.slug {
            query = query.data("slug", DbValue::String(slug));
        }
        if data.logo_set {
            query = query.data("logo", option_string(data.logo));
        }
        if data.metadata_set {
            query = query.data(
                "metadata",
                data.metadata.map(DbValue::Json).unwrap_or(DbValue::Null),
            );
        }
        self.adapter
            .update(query)
            .await?
            .map(|record| organization_from_record(&record))
            .transpose()
    }

    pub async fn delete_organization(&self, id: &str) -> Result<(), OpenAuthError> {
        self.adapter
            .delete_many(DeleteMany::new("member").where_clause(Where::new(
                "organization_id",
                DbValue::String(id.to_owned()),
            )))
            .await?;
        self.adapter
            .delete_many(DeleteMany::new("invitation").where_clause(Where::new(
                "organization_id",
                DbValue::String(id.to_owned()),
            )))
            .await?;
        self.adapter
            .delete(Delete::new("organization").where_clause(id_where(id)))
            .await
    }

    pub async fn create_member(
        &self,
        organization_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<Member, OpenAuthError> {
        let record = self
            .adapter
            .create(
                Create::new("member")
                    .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
                    .data(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    )
                    .data("user_id", DbValue::String(user_id.to_owned()))
                    .data("role", DbValue::String(role.to_owned()))
                    .data("created_at", DbValue::Timestamp(OffsetDateTime::now_utc()))
                    .force_allow_id(),
            )
            .await?;
        member_from_record(&record)
    }

    pub async fn member_by_org_user(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<Option<Member>, OpenAuthError> {
        self.adapter
            .find_one(
                FindOne::new("member")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .where_clause(Where::new("user_id", DbValue::String(user_id.to_owned()))),
            )
            .await?
            .map(|record| member_from_record(&record))
            .transpose()
    }

    pub async fn member_by_id(&self, id: &str) -> Result<Option<Member>, OpenAuthError> {
        self.adapter
            .find_one(FindOne::new("member").where_clause(id_where(id)))
            .await?
            .map(|record| member_from_record(&record))
            .transpose()
    }

    pub async fn member_by_email(
        &self,
        organization_id: &str,
        email: &str,
    ) -> Result<Option<Member>, OpenAuthError> {
        let Some(user) = self.user_by_email(email).await? else {
            return Ok(None);
        };
        self.member_by_org_user(organization_id, &user.id).await
    }

    pub async fn members(&self, organization_id: &str) -> Result<Vec<Member>, OpenAuthError> {
        self.adapter
            .find_many(
                FindMany::new("member")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .sort_by(Sort::new("created_at", SortDirection::Asc)),
            )
            .await?
            .iter()
            .map(member_from_record)
            .collect()
    }

    pub async fn count_members(&self, organization_id: &str) -> Result<u64, OpenAuthError> {
        self.adapter
            .count(Count::new("member").where_clause(Where::new(
                "organization_id",
                DbValue::String(organization_id.to_owned()),
            )))
            .await
    }

    pub async fn update_member_role(
        &self,
        member_id: &str,
        role: &str,
    ) -> Result<Option<Member>, OpenAuthError> {
        self.adapter
            .update(
                Update::new("member")
                    .where_clause(id_where(member_id))
                    .data("role", DbValue::String(role.to_owned())),
            )
            .await?
            .map(|record| member_from_record(&record))
            .transpose()
    }

    pub async fn delete_member(&self, member_id: &str) -> Result<(), OpenAuthError> {
        self.adapter
            .delete(Delete::new("member").where_clause(id_where(member_id)))
            .await
    }

    pub async fn user_by_id(
        &self,
        id: &str,
    ) -> Result<Option<openauth_core::db::User>, OpenAuthError> {
        self.adapter
            .find_one(FindOne::new("user").where_clause(id_where(id)))
            .await?
            .map(|record| user_from_record(&record))
            .transpose()
    }

    pub async fn user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<openauth_core::db::User>, OpenAuthError> {
        self.adapter
            .find_one(
                FindOne::new("user").where_clause(
                    Where::new("email", DbValue::String(email.to_owned())).insensitive(),
                ),
            )
            .await?
            .map(|record| user_from_record(&record))
            .transpose()
    }

    pub async fn organizations_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<Organization>, OpenAuthError> {
        let members = self
            .adapter
            .find_many(
                FindMany::new("member")
                    .where_clause(Where::new("user_id", DbValue::String(user_id.to_owned()))),
            )
            .await?;
        let mut organizations = Vec::new();
        for member in members {
            let organization_id = required_string(&member, "organization_id")?;
            if let Some(organization) = self.organization_by_id(&organization_id).await? {
                organizations.push(organization);
            }
        }
        Ok(organizations)
    }

    pub async fn set_active_organization(
        &self,
        token: &str,
        organization_id: Option<&str>,
    ) -> Result<(), OpenAuthError> {
        self.adapter
            .update(
                Update::new("session")
                    .where_clause(Where::new("token", DbValue::String(token.to_owned())))
                    .data(
                        "active_organization_id",
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
            .adapter
            .find_one(
                FindOne::new("session")
                    .where_clause(Where::new("token", DbValue::String(token.to_owned()))),
            )
            .await?
        else {
            return Ok(None);
        };
        optional_string(&record, "active_organization_id")
    }

    pub async fn create_invitation(
        &self,
        organization_id: &str,
        email: &str,
        role: &str,
        inviter_id: &str,
        expires_at: OffsetDateTime,
    ) -> Result<Invitation, OpenAuthError> {
        let record = self
            .adapter
            .create(
                Create::new("invitation")
                    .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
                    .data(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    )
                    .data("email", DbValue::String(email.to_owned()))
                    .data("role", DbValue::String(role.to_owned()))
                    .data(
                        "status",
                        DbValue::String(InvitationStatus::Pending.as_str().to_owned()),
                    )
                    .data("expires_at", DbValue::Timestamp(expires_at))
                    .data("created_at", DbValue::Timestamp(OffsetDateTime::now_utc()))
                    .data("inviter_id", DbValue::String(inviter_id.to_owned()))
                    .force_allow_id(),
            )
            .await?;
        invitation_from_record(&record)
    }

    pub async fn invitation_by_id(&self, id: &str) -> Result<Option<Invitation>, OpenAuthError> {
        self.adapter
            .find_one(FindOne::new("invitation").where_clause(id_where(id)))
            .await?
            .map(|record| invitation_from_record(&record))
            .transpose()
    }

    pub async fn pending_invitations(
        &self,
        organization_id: &str,
    ) -> Result<Vec<Invitation>, OpenAuthError> {
        self.adapter
            .find_many(
                FindMany::new("invitation")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .where_clause(Where::new(
                        "status",
                        DbValue::String(InvitationStatus::Pending.as_str().to_owned()),
                    )),
            )
            .await?
            .iter()
            .map(invitation_from_record)
            .collect()
    }

    pub async fn pending_invitation_by_email(
        &self,
        organization_id: &str,
        email: &str,
    ) -> Result<Option<Invitation>, OpenAuthError> {
        self.adapter
            .find_one(
                FindOne::new("invitation")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .where_clause(Where::new("email", DbValue::String(email.to_owned())))
                    .where_clause(Where::new(
                        "status",
                        DbValue::String(InvitationStatus::Pending.as_str().to_owned()),
                    )),
            )
            .await?
            .map(|record| invitation_from_record(&record))
            .transpose()
    }

    pub async fn update_invitation_status(
        &self,
        id: &str,
        status: InvitationStatus,
    ) -> Result<Option<Invitation>, OpenAuthError> {
        self.adapter
            .update(
                Update::new("invitation")
                    .where_clause(id_where(id))
                    .data("status", DbValue::String(status.as_str().to_owned())),
            )
            .await?
            .map(|record| invitation_from_record(&record))
            .transpose()
    }

    pub async fn extend_invitation(
        &self,
        id: &str,
        expires_at: OffsetDateTime,
    ) -> Result<Option<Invitation>, OpenAuthError> {
        self.adapter
            .update(
                Update::new("invitation")
                    .where_clause(id_where(id))
                    .data("expires_at", DbValue::Timestamp(expires_at)),
            )
            .await?
            .map(|record| invitation_from_record(&record))
            .transpose()
    }

    pub async fn invitations_for_organization(
        &self,
        organization_id: &str,
    ) -> Result<Vec<Invitation>, OpenAuthError> {
        self.adapter
            .find_many(FindMany::new("invitation").where_clause(Where::new(
                "organization_id",
                DbValue::String(organization_id.to_owned()),
            )))
            .await?
            .iter()
            .map(invitation_from_record)
            .collect()
    }

    pub async fn invitations_for_email(
        &self,
        email: &str,
    ) -> Result<Vec<Invitation>, OpenAuthError> {
        self.adapter
            .find_many(
                FindMany::new("invitation")
                    .where_clause(Where::new("email", DbValue::String(email.to_owned())))
                    .where_clause(Where::new(
                        "status",
                        DbValue::String(InvitationStatus::Pending.as_str().to_owned()),
                    )),
            )
            .await?
            .iter()
            .map(invitation_from_record)
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct OrganizationUpdate {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub logo: Option<String>,
    pub logo_set: bool,
    pub metadata: Option<serde_json::Value>,
    pub metadata_set: bool,
}

fn id_where(id: &str) -> Where {
    Where::new("id", DbValue::String(id.to_owned()))
}

fn option_string(value: Option<String>) -> DbValue {
    value.map(DbValue::String).unwrap_or(DbValue::Null)
}
