use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};

use crate::organization::additional_fields;
use crate::organization::http;
use crate::organization::models::Member;
use crate::organization::options::OrganizationOptions;
use crate::organization::store::OrganizationStore;

use super::validation::require_session;

pub(super) fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    vec![
        get_active_member(options.clone()),
        list_members(options),
        get_active_member_role(),
    ]
}

fn get_active_member(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-active-member",
        Method::GET,
        AuthEndpointOptions::new().operation_id("organizationGetActiveMember"),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let Some(organization_id) = session.active_organization_id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                match store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                {
                    Some(mut member) => {
                        retain_returned_member_fields(&mut member, &options);
                        http::json(StatusCode::OK, &member)
                    }
                    None => http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND"),
                }
            })
        },
    )
}

fn list_members(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-members",
        Method::GET,
        AuthEndpointOptions::new().operation_id("organizationListMembers"),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let Some(organization_id) = session.active_organization_id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                if store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                    .is_none()
                {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                }
                let mut members = store.members(&organization_id).await?;
                for member in &mut members {
                    retain_returned_member_fields(member, &options);
                }
                http::json(StatusCode::OK, &serde_json::json!({ "members": members }))
            })
        },
    )
}

fn get_active_member_role() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-active-member-role",
        Method::GET,
        AuthEndpointOptions::new().operation_id("organizationGetActiveMemberRole"),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let Some(organization_id) = session.active_organization_id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                match store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                {
                    Some(member) => {
                        http::json(StatusCode::OK, &serde_json::json!({ "role": member.role }))
                    }
                    None => http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND"),
                }
            })
        },
    )
}

fn retain_returned_member_fields(member: &mut Member, options: &OrganizationOptions) {
    additional_fields::retain_returned(
        &mut member.additional_fields,
        &options.schema.member.additional_fields,
    );
}
