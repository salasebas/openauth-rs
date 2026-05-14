use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::organization::hooks::{AfterCreateInvitation, BeforeCreateInvitation};
use crate::organization::http;
use crate::organization::models::InvitationStatus;
use crate::organization::options::{InvitationEmail, OrganizationOptions};
use crate::organization::permissions::{
    has_permission, is_known_static_role, OrganizationPermission,
};
use crate::organization::store::OrganizationStore;

use super::input::RoleInput;

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    vec![
        create_invitation(options.clone()),
        accept_invitation(options.clone()),
        reject_invitation(options.clone()),
        cancel_invitation(options.clone()),
        get_invitation(),
        list_invitations(),
        list_user_invitations(),
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InviteBody {
    email: String,
    role: RoleInput,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    resend: bool,
}

fn create_invitation(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/invite-member",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: InviteBody = http::body(&request)?;
                let Some(organization_id) = super::resolve_organization_id(
                    input.organization_id,
                    session.active_organization_id.as_deref(),
                ) else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                let email = input.email.trim().to_lowercase();
                if !email.contains('@') {
                    return http::error(StatusCode::BAD_REQUEST, "INVALID_EMAIL", "Invalid email");
                }
                let Some(actor_member) = store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if !has_permission(
                    &actor_member.role,
                    &options,
                    OrganizationPermission::InvitationCreate,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_INVITE_USERS_TO_THIS_ORGANIZATION",
                    );
                }
                let role = input.role.normalized();
                for role in role
                    .split(',')
                    .map(str::trim)
                    .filter(|role| !role.is_empty())
                {
                    if !is_known_static_role(role, &options) {
                        return http::error(
                            StatusCode::BAD_REQUEST,
                            "ROLE_NOT_FOUND",
                            &format!(
                                "{}: {role}",
                                crate::organization::errors::message("ROLE_NOT_FOUND")
                            ),
                        );
                    }
                    if role == options.creator_role
                        && !actor_member
                            .role
                            .split(',')
                            .any(|actor_role| actor_role.trim() == options.creator_role)
                    {
                        return http::organization_error(
                            StatusCode::FORBIDDEN,
                            "YOU_ARE_NOT_ALLOWED_TO_INVITE_USER_WITH_THIS_ROLE",
                        );
                    }
                }
                if store
                    .member_by_email(&organization_id, &email)
                    .await?
                    .is_some()
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_ALREADY_A_MEMBER_OF_THIS_ORGANIZATION",
                    );
                }
                let expires_at =
                    OffsetDateTime::now_utc() + Duration::seconds(options.invitation_expires_in);
                if let Some(existing) = store
                    .pending_invitation_by_email(&organization_id, &email)
                    .await?
                {
                    if input.resend {
                        let invitation = store.extend_invitation(&existing.id, expires_at).await?;
                        return http::json(StatusCode::OK, &invitation);
                    }
                    if options.cancel_pending_invitations_on_re_invite {
                        store
                            .update_invitation_status(&existing.id, InvitationStatus::Canceled)
                            .await?;
                    } else {
                        return http::organization_error(
                            StatusCode::BAD_REQUEST,
                            "USER_IS_ALREADY_INVITED_TO_THIS_ORGANIZATION",
                        );
                    }
                }
                if store.pending_invitations(&organization_id).await?.len()
                    >= options.invitation_limit
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "INVITATION_LIMIT_REACHED",
                    );
                }
                let Some(organization) = store.organization_by_id(&organization_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                if let Some(hook) = &options.hooks.before_create_invitation {
                    hook(&BeforeCreateInvitation {
                        organization: organization.clone(),
                        inviter: session.user.clone(),
                        email: email.clone(),
                        role: role.clone(),
                    })?;
                }
                let invitation = store
                    .create_invitation(
                        &organization_id,
                        &email,
                        &role,
                        input.team_id.as_deref(),
                        &session.user.id,
                        expires_at,
                    )
                    .await?;
                if let Some(send_email) = &options.send_invitation_email {
                    send_email(&InvitationEmail {
                        id: invitation.id.clone(),
                        role: invitation.role.clone(),
                        email: invitation.email.clone(),
                        organization: organization.clone(),
                        invitation: invitation.clone(),
                        inviter: actor_member.clone(),
                    })?;
                }
                if let Some(hook) = &options.hooks.after_create_invitation {
                    hook(&AfterCreateInvitation {
                        organization,
                        inviter: session.user,
                        invitation: invitation.clone(),
                    })?;
                }
                http::json(StatusCode::OK, &invitation)
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvitationIdBody {
    invitation_id: String,
}

fn accept_invitation(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/accept-invitation",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: InvitationIdBody = http::body(&request)?;
                let Some(invitation) = store.invitation_by_id(&input.invitation_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "INVITATION_NOT_FOUND",
                    );
                };
                if invitation.status != InvitationStatus::Pending
                    || invitation.expires_at < OffsetDateTime::now_utc()
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "INVITATION_NOT_FOUND",
                    );
                }
                if invitation.email.to_lowercase() != session.user.email.to_lowercase() {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_THE_RECIPIENT_OF_THE_INVITATION",
                    );
                }
                if options.require_email_verification_on_invitation && !session.user.email_verified
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "EMAIL_VERIFICATION_REQUIRED_BEFORE_ACCEPTING_OR_REJECTING_INVITATION",
                    );
                }
                if store.count_members(&invitation.organization_id).await? as usize
                    >= options.membership_limit
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "ORGANIZATION_MEMBERSHIP_LIMIT_REACHED",
                    );
                }
                if store
                    .member_by_org_user(&invitation.organization_id, &session.user.id)
                    .await?
                    .is_some()
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_ALREADY_A_MEMBER_OF_THIS_ORGANIZATION",
                    );
                }
                let accepted = store
                    .update_invitation_status(&invitation.id, InvitationStatus::Accepted)
                    .await?;
                let member = store
                    .create_member(
                        &invitation.organization_id,
                        &session.user.id,
                        &invitation.role,
                    )
                    .await?;
                if options.teams.enabled {
                    if let Some(team_ids) = invitation.team_id.as_deref() {
                        for team_id in team_ids
                            .split(',')
                            .map(str::trim)
                            .filter(|id| !id.is_empty())
                        {
                            if store
                                .team_member(team_id, &session.user.id)
                                .await?
                                .is_none()
                            {
                                store.create_team_member(team_id, &session.user.id).await?;
                            }
                        }
                    }
                }
                store
                    .set_active_organization(
                        &session.session.token,
                        Some(&invitation.organization_id),
                    )
                    .await?;
                http::json(
                    StatusCode::OK,
                    &serde_json::json!({ "invitation": accepted, "member": member }),
                )
            })
        },
    )
}

fn reject_invitation(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/reject-invitation",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: InvitationIdBody = http::body(&request)?;
                let Some(invitation) = store.invitation_by_id(&input.invitation_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "INVITATION_NOT_FOUND",
                    );
                };
                if invitation.email.to_lowercase() != session.user.email.to_lowercase() {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_THE_RECIPIENT_OF_THE_INVITATION",
                    );
                }
                if options.require_email_verification_on_invitation && !session.user.email_verified
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "EMAIL_VERIFICATION_REQUIRED_BEFORE_ACCEPTING_OR_REJECTING_INVITATION",
                    );
                }
                let rejected = store
                    .update_invitation_status(&invitation.id, InvitationStatus::Rejected)
                    .await?;
                http::json(
                    StatusCode::OK,
                    &serde_json::json!({ "invitation": rejected, "member": null }),
                )
            })
        },
    )
}

fn cancel_invitation(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/cancel-invitation",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: InvitationIdBody = http::body(&request)?;
                let Some(invitation) = store.invitation_by_id(&input.invitation_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "INVITATION_NOT_FOUND",
                    );
                };
                let Some(actor_member) = store
                    .member_by_org_user(&invitation.organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if !has_permission(
                    &actor_member.role,
                    &options,
                    OrganizationPermission::InvitationCancel,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_CANCEL_THIS_INVITATION",
                    );
                }
                let canceled = store
                    .update_invitation_status(&invitation.id, InvitationStatus::Canceled)
                    .await?;
                http::json(
                    StatusCode::OK,
                    &serde_json::json!({ "invitation": canceled }),
                )
            })
        },
    )
}

fn get_invitation() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-invitation",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let id =
                    query_param(&request, "id").or_else(|| query_param(&request, "invitationId"));
                let Some(id) = id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "INVITATION_NOT_FOUND",
                    );
                };
                match store.invitation_by_id(&id).await? {
                    Some(invitation) => http::json(StatusCode::OK, &invitation),
                    None => {
                        http::organization_error(StatusCode::BAD_REQUEST, "INVITATION_NOT_FOUND")
                    }
                }
            })
        },
    )
}

fn list_invitations() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-invitations",
        Method::GET,
        AuthEndpointOptions::new(),
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
                if store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                    .is_none()
                {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                }
                let invitations = store.invitations_for_organization(&organization_id).await?;
                http::json(StatusCode::OK, &invitations)
            })
        },
    )
}

fn list_user_invitations() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-user-invitations",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let invitations = store
                    .invitations_for_email(&session.user.email.to_lowercase())
                    .await?;
                http::json(StatusCode::OK, &invitations)
            })
        },
    )
}

async fn require_session(
    context: &openauth_core::context::AuthContext,
    request: &openauth_core::api::ApiRequest,
    store: &OrganizationStore<'_>,
) -> Result<crate::organization::http::CurrentSession, openauth_core::error::OpenAuthError> {
    match http::current_session(context, request, store).await? {
        Some(session) => Ok(session),
        None => Err(openauth_core::error::OpenAuthError::Api(
            "UNAUTHORIZED".to_owned(),
        )),
    }
}

fn query_param(request: &openauth_core::api::ApiRequest, name: &str) -> Option<String> {
    request.uri().query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then(|| value.to_owned())
        })
    })
}
