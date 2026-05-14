//! Organization plugin.

mod errors;
mod hooks;
mod http;
mod models;
mod options;
mod permissions;
mod record;
mod routes;
mod schema;
mod store;

pub use errors::ORGANIZATION_ERROR_CODES;
pub use hooks::{
    AfterAddMember, AfterCreateInvitation, AfterCreateOrganization, BeforeAddMember,
    BeforeCreateInvitation, BeforeCreateOrganization, OrganizationHooks,
};
pub use models::{
    Invitation, InvitationStatus, Member, Organization, OrganizationRoleRecord, Team, TeamMember,
};
pub use options::{
    DynamicAccessControlOptions, InvitationEmail, OrganizationOptions, OrganizationOptionsBuilder,
    SendInvitationEmailHook, TeamOptions,
};
pub use permissions::{has_permission, OrganizationPermission, OrganizationRole};

use openauth_core::plugin::AuthPlugin;

pub const UPSTREAM_PLUGIN_ID: &str = "organization";

pub fn organization() -> AuthPlugin {
    organization_with_options(OrganizationOptions::default())
}

pub fn organization_with_options(options: OrganizationOptions) -> AuthPlugin {
    let mut plugin = AuthPlugin::new(UPSTREAM_PLUGIN_ID)
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_options(options.to_metadata());

    for contribution in schema::schema_contributions(&options) {
        plugin = plugin.with_schema(contribution);
    }
    for error_code in errors::error_codes() {
        plugin = plugin.with_error_code(error_code);
    }
    for endpoint in routes::endpoints(options) {
        plugin = plugin.with_endpoint(endpoint);
    }
    plugin
}
