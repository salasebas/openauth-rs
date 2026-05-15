use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};

use crate::organization::hooks::{AfterRemoveMember, BeforeRemoveMember};
use crate::organization::http;
use crate::organization::options::OrganizationOptions;
use crate::organization::store::OrganizationStore;

use super::validation::{is_last_owner, require_session};

pub(super) fn leave(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/leave",
        Method::POST,
        AuthEndpointOptions::new().operation_id("organizationLeave"),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let Some(organization_id) = session.active_organization_id.clone() else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let Some(member) = store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if is_last_owner(&store, &organization_id, &member, &options).await? {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "YOU_CANNOT_LEAVE_THE_ORGANIZATION_AS_THE_ONLY_OWNER",
                    );
                }
                let Some(organization) = store.organization_by_id(&organization_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                if let Some(hook) = &options.hooks.before_remove_member {
                    hook(&BeforeRemoveMember {
                        organization: organization.clone(),
                        member: member.clone(),
                        user: session.user.clone(),
                    })?;
                }
                if options.teams.enabled {
                    store
                        .delete_team_members_for_user(&organization_id, &session.user.id)
                        .await?;
                }
                store.delete_member(&member.id).await?;
                if let Some(hook) = &options.hooks.after_remove_member {
                    hook(&AfterRemoveMember {
                        organization,
                        member: member.clone(),
                        user: session.user.clone(),
                    })?;
                }
                store
                    .set_active_organization(&session.session.token, None)
                    .await?;
                if options.teams.enabled {
                    store.set_active_team(&session.session.token, None).await?;
                }
                http::json_with_cookies(
                    StatusCode::OK,
                    &serde_json::json!({ "member": member }),
                    http::refreshed_session_cookies(context, &session.session, &session.user)?,
                )
            })
        },
    )
}
