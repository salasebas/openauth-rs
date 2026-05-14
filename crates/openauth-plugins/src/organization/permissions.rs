use serde::{Deserialize, Serialize};

use super::options::OrganizationOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
}

impl OrganizationRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationPermission {
    OrganizationUpdate,
    OrganizationDelete,
    MemberCreate,
    MemberUpdate,
    MemberDelete,
    InvitationCreate,
    InvitationCancel,
}

pub fn has_permission(
    role: &str,
    options: &OrganizationOptions,
    permission: OrganizationPermission,
) -> bool {
    role.split(',').map(str::trim).any(|role| {
        if role == options.creator_role {
            return true;
        }
        match role {
            "owner" => true,
            "admin" => !matches!(permission, OrganizationPermission::OrganizationDelete),
            "member" => false,
            _ => false,
        }
    })
}

pub(crate) fn parse_roles(role: impl AsRef<str>) -> String {
    role.as_ref()
        .split(',')
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn is_known_static_role(role: &str, options: &OrganizationOptions) -> bool {
    role == options.creator_role || matches!(role, "owner" | "admin" | "member")
}
