use serde_json::{json, Value};

use super::hooks::OrganizationHooks;

#[derive(Clone)]
pub struct OrganizationOptions {
    pub allow_user_to_create_organization: bool,
    pub organization_limit: Option<usize>,
    pub creator_role: String,
    pub membership_limit: usize,
    pub invitation_expires_in: i64,
    pub invitation_limit: usize,
    pub cancel_pending_invitations_on_re_invite: bool,
    pub require_email_verification_on_invitation: bool,
    pub disable_organization_deletion: bool,
    pub hooks: OrganizationHooks,
}

impl Default for OrganizationOptions {
    fn default() -> Self {
        Self {
            allow_user_to_create_organization: true,
            organization_limit: None,
            creator_role: "owner".to_owned(),
            membership_limit: 100,
            invitation_expires_in: 60 * 60 * 48,
            invitation_limit: 100,
            cancel_pending_invitations_on_re_invite: false,
            require_email_verification_on_invitation: false,
            disable_organization_deletion: false,
            hooks: OrganizationHooks::default(),
        }
    }
}

impl OrganizationOptions {
    pub fn builder() -> OrganizationOptionsBuilder {
        OrganizationOptionsBuilder::default()
    }

    pub(crate) fn to_metadata(&self) -> Value {
        json!({
            "allowUserToCreateOrganization": self.allow_user_to_create_organization,
            "organizationLimit": self.organization_limit,
            "creatorRole": self.creator_role,
            "membershipLimit": self.membership_limit,
            "invitationExpiresIn": self.invitation_expires_in,
            "invitationLimit": self.invitation_limit,
            "cancelPendingInvitationsOnReInvite": self.cancel_pending_invitations_on_re_invite,
            "requireEmailVerificationOnInvitation": self.require_email_verification_on_invitation,
            "disableOrganizationDeletion": self.disable_organization_deletion,
        })
    }
}

#[derive(Clone, Default)]
pub struct OrganizationOptionsBuilder {
    options: OrganizationOptions,
}

impl OrganizationOptionsBuilder {
    pub fn allow_user_to_create_organization(mut self, allow: bool) -> Self {
        self.options.allow_user_to_create_organization = allow;
        self
    }

    pub fn organization_limit(mut self, limit: usize) -> Self {
        self.options.organization_limit = Some(limit);
        self
    }

    pub fn creator_role(mut self, role: impl Into<String>) -> Self {
        self.options.creator_role = role.into();
        self
    }

    pub fn membership_limit(mut self, limit: usize) -> Self {
        self.options.membership_limit = limit;
        self
    }

    pub fn invitation_expires_in(mut self, seconds: i64) -> Self {
        self.options.invitation_expires_in = seconds;
        self
    }

    pub fn invitation_limit(mut self, limit: usize) -> Self {
        self.options.invitation_limit = limit;
        self
    }

    pub fn cancel_pending_invitations_on_re_invite(mut self, cancel: bool) -> Self {
        self.options.cancel_pending_invitations_on_re_invite = cancel;
        self
    }

    pub fn require_email_verification_on_invitation(mut self, require: bool) -> Self {
        self.options.require_email_verification_on_invitation = require;
        self
    }

    pub fn disable_organization_deletion(mut self, disable: bool) -> Self {
        self.options.disable_organization_deletion = disable;
        self
    }

    pub fn hooks(mut self, hooks: OrganizationHooks) -> Self {
        self.options.hooks = hooks;
        self
    }

    pub fn build(self) -> OrganizationOptions {
        self.options
    }
}
