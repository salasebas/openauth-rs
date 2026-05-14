mod invitations;
mod members;
mod org;
mod permissions;

use openauth_core::api::AsyncAuthEndpoint;

use super::options::OrganizationOptions;

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    let mut endpoints = Vec::new();
    endpoints.extend(org::endpoints(options.clone()));
    endpoints.extend(members::endpoints(options.clone()));
    endpoints.extend(invitations::endpoints(options.clone()));
    endpoints.extend(permissions::endpoints(options));
    endpoints
}

fn resolve_organization_id(explicit: Option<String>, active: Option<&str>) -> Option<String> {
    explicit.or_else(|| active.map(str::to_owned))
}
