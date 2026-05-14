use openauth_core::crypto::random::generate_random_string;
use openauth_core::db::{
    Count, Create, DbValue, Delete, DeleteMany, FindMany, FindOne, Sort, SortDirection, Update,
    Where,
};
use openauth_core::error::OpenAuthError;
use time::OffsetDateTime;

use super::{id_where, OrganizationStore, ID_LENGTH};
use crate::organization::record::{team_from_record, team_member_from_record};
use crate::organization::{Team, TeamMember};

impl<'a> OrganizationStore<'a> {
    pub async fn create_team(
        &self,
        organization_id: &str,
        name: &str,
    ) -> Result<Team, OpenAuthError> {
        let now = OffsetDateTime::now_utc();
        let record = self
            .adapter()
            .create(
                Create::new("team")
                    .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
                    .data("name", DbValue::String(name.to_owned()))
                    .data(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    )
                    .data("created_at", DbValue::Timestamp(now))
                    .data("updated_at", DbValue::Timestamp(now))
                    .force_allow_id(),
            )
            .await?;
        team_from_record(&record)
    }

    pub async fn update_team(
        &self,
        team_id: &str,
        name: &str,
    ) -> Result<Option<Team>, OpenAuthError> {
        self.adapter()
            .update(
                Update::new("team")
                    .where_clause(id_where(team_id))
                    .data("name", DbValue::String(name.to_owned()))
                    .data("updated_at", DbValue::Timestamp(OffsetDateTime::now_utc())),
            )
            .await?
            .map(|record| team_from_record(&record))
            .transpose()
    }

    pub async fn delete_team(&self, team_id: &str) -> Result<(), OpenAuthError> {
        self.adapter()
            .delete_many(
                DeleteMany::new("team_member")
                    .where_clause(Where::new("team_id", DbValue::String(team_id.to_owned()))),
            )
            .await?;
        self.adapter()
            .delete(Delete::new("team").where_clause(id_where(team_id)))
            .await
    }

    pub async fn team_by_id(&self, team_id: &str) -> Result<Option<Team>, OpenAuthError> {
        self.adapter()
            .find_one(FindOne::new("team").where_clause(id_where(team_id)))
            .await?
            .map(|record| team_from_record(&record))
            .transpose()
    }

    pub async fn teams_for_organization(
        &self,
        organization_id: &str,
    ) -> Result<Vec<Team>, OpenAuthError> {
        self.adapter()
            .find_many(
                FindMany::new("team")
                    .where_clause(Where::new(
                        "organization_id",
                        DbValue::String(organization_id.to_owned()),
                    ))
                    .sort_by(Sort::new("created_at", SortDirection::Asc)),
            )
            .await?
            .iter()
            .map(team_from_record)
            .collect()
    }

    pub async fn create_team_member(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<TeamMember, OpenAuthError> {
        let record = self
            .adapter()
            .create(
                Create::new("team_member")
                    .data("id", DbValue::String(generate_random_string(ID_LENGTH)))
                    .data("team_id", DbValue::String(team_id.to_owned()))
                    .data("user_id", DbValue::String(user_id.to_owned()))
                    .data("created_at", DbValue::Timestamp(OffsetDateTime::now_utc()))
                    .force_allow_id(),
            )
            .await?;
        team_member_from_record(&record)
    }

    pub async fn team_member(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<Option<TeamMember>, OpenAuthError> {
        self.adapter()
            .find_one(
                FindOne::new("team_member")
                    .where_clause(Where::new("team_id", DbValue::String(team_id.to_owned())))
                    .where_clause(Where::new("user_id", DbValue::String(user_id.to_owned()))),
            )
            .await?
            .map(|record| team_member_from_record(&record))
            .transpose()
    }

    pub async fn team_members(&self, team_id: &str) -> Result<Vec<TeamMember>, OpenAuthError> {
        self.adapter()
            .find_many(
                FindMany::new("team_member")
                    .where_clause(Where::new("team_id", DbValue::String(team_id.to_owned())))
                    .sort_by(Sort::new("created_at", SortDirection::Asc)),
            )
            .await?
            .iter()
            .map(team_member_from_record)
            .collect()
    }

    pub async fn count_team_members(&self, team_id: &str) -> Result<u64, OpenAuthError> {
        self.adapter()
            .count(
                Count::new("team_member")
                    .where_clause(Where::new("team_id", DbValue::String(team_id.to_owned()))),
            )
            .await
    }

    pub async fn delete_team_member(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<(), OpenAuthError> {
        self.adapter()
            .delete_many(
                DeleteMany::new("team_member")
                    .where_clause(Where::new("team_id", DbValue::String(team_id.to_owned())))
                    .where_clause(Where::new("user_id", DbValue::String(user_id.to_owned()))),
            )
            .await?;
        Ok(())
    }

    pub async fn delete_team_members_for_user(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<(), OpenAuthError> {
        for team in self.teams_for_organization(organization_id).await? {
            self.delete_team_member(&team.id, user_id).await?;
        }
        Ok(())
    }

    pub async fn set_active_team(
        &self,
        token: &str,
        team_id: Option<&str>,
    ) -> Result<(), OpenAuthError> {
        self.adapter()
            .update(
                Update::new("session")
                    .where_clause(Where::new("token", DbValue::String(token.to_owned())))
                    .data(
                        "active_team_id",
                        team_id
                            .map(|value| DbValue::String(value.to_owned()))
                            .unwrap_or(DbValue::Null),
                    ),
            )
            .await?;
        Ok(())
    }
}
