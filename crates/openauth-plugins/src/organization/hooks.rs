//! Organization hook callbacks.

use std::sync::Arc;

use openauth_core::db::User;
use openauth_core::error::OpenAuthError;

use super::{Invitation, Member, Organization};

#[derive(Clone, Default)]
pub struct OrganizationHooks {
    pub before_create_organization: Option<BeforeCreateOrganizationHook>,
    pub after_create_organization: Option<AfterCreateOrganizationHook>,
    pub before_add_member: Option<BeforeAddMemberHook>,
    pub after_add_member: Option<AfterAddMemberHook>,
    pub before_create_invitation: Option<BeforeCreateInvitationHook>,
    pub after_create_invitation: Option<AfterCreateInvitationHook>,
}

pub type BeforeCreateOrganizationHook =
    Arc<dyn Fn(&BeforeCreateOrganization) -> Result<(), OpenAuthError> + Send + Sync>;
pub type AfterCreateOrganizationHook =
    Arc<dyn Fn(&AfterCreateOrganization) -> Result<(), OpenAuthError> + Send + Sync>;
pub type BeforeAddMemberHook =
    Arc<dyn Fn(&BeforeAddMember) -> Result<(), OpenAuthError> + Send + Sync>;
pub type AfterAddMemberHook =
    Arc<dyn Fn(&AfterAddMember) -> Result<(), OpenAuthError> + Send + Sync>;
pub type BeforeCreateInvitationHook =
    Arc<dyn Fn(&BeforeCreateInvitation) -> Result<(), OpenAuthError> + Send + Sync>;
pub type AfterCreateInvitationHook =
    Arc<dyn Fn(&AfterCreateInvitation) -> Result<(), OpenAuthError> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct BeforeCreateOrganization {
    pub name: String,
    pub slug: String,
    pub user: User,
}

#[derive(Debug, Clone)]
pub struct AfterCreateOrganization {
    pub organization: Organization,
    pub member: Member,
    pub user: User,
}

#[derive(Debug, Clone)]
pub struct BeforeAddMember {
    pub organization: Organization,
    pub user: User,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct AfterAddMember {
    pub organization: Organization,
    pub member: Member,
    pub user: User,
}

#[derive(Debug, Clone)]
pub struct BeforeCreateInvitation {
    pub organization: Organization,
    pub inviter: User,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct AfterCreateInvitation {
    pub organization: Organization,
    pub inviter: User,
    pub invitation: Invitation,
}
