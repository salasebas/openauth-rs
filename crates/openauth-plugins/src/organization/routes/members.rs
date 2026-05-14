use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use serde::Deserialize;

use crate::organization::hooks::{AfterAddMember, BeforeAddMember};
use crate::organization::http;
use crate::organization::options::OrganizationOptions;
use crate::organization::permissions::{has_permission, parse_roles, OrganizationPermission};
use crate::organization::store::OrganizationStore;

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    vec![
        add_member(options.clone()),
        remove_member(options.clone()),
        update_member_role(options.clone()),
        get_active_member(),
        leave(options.clone()),
        list_members(),
        get_active_member_role(),
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddMemberBody {
    user_id: String,
    role: RoleInput,
    #[serde(default)]
    organization_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RoleInput {
    One(String),
    Many(Vec<String>),
}

impl RoleInput {
    fn normalized(&self) -> String {
        match self {
            Self::One(role) => parse_roles(role),
            Self::Many(roles) => parse_roles(roles.join(",")),
        }
    }
}

fn add_member(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/add-member",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = http::current_session(context, &request, &store).await?;
                let input: AddMemberBody = http::body(&request)?;
                let organization_id = super::resolve_organization_id(
                    input.organization_id,
                    session
                        .as_ref()
                        .and_then(|session| session.active_organization_id.as_deref()),
                );
                let Some(organization_id) = organization_id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let Some(actor) = session else {
                    return http::error(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Unauthorized");
                };
                let Some(actor_member) = store
                    .member_by_org_user(&organization_id, &actor.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if !has_permission(
                    &actor_member.role,
                    &options,
                    OrganizationPermission::MemberCreate,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_UPDATE_THIS_MEMBER",
                    );
                }
                let Some(user) = store.user_by_id(&input.user_id).await? else {
                    return http::error(
                        StatusCode::BAD_REQUEST,
                        "USER_NOT_FOUND",
                        "User not found",
                    );
                };
                if store
                    .member_by_org_user(&organization_id, &user.id)
                    .await?
                    .is_some()
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_ALREADY_A_MEMBER_OF_THIS_ORGANIZATION",
                    );
                }
                if store.count_members(&organization_id).await? as usize >= options.membership_limit
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "ORGANIZATION_MEMBERSHIP_LIMIT_REACHED",
                    );
                }
                let Some(organization) = store.organization_by_id(&organization_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                let role = input.role.normalized();
                if let Some(hook) = &options.hooks.before_add_member {
                    hook(&BeforeAddMember {
                        organization: organization.clone(),
                        user: user.clone(),
                        role: role.clone(),
                    })?;
                }
                let member = store
                    .create_member(&organization_id, &user.id, &role)
                    .await?;
                if let Some(hook) = &options.hooks.after_add_member {
                    hook(&AfterAddMember {
                        organization,
                        member: member.clone(),
                        user,
                    })?;
                }
                http::json(StatusCode::OK, &member)
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveMemberBody {
    member_id_or_email: String,
    #[serde(default)]
    organization_id: Option<String>,
}

fn remove_member(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/remove-member",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: RemoveMemberBody = http::body(&request)?;
                let Some(organization_id) = super::resolve_organization_id(
                    input.organization_id,
                    session.active_organization_id.as_deref(),
                ) else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let Some(actor_member) = store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                let target = if input.member_id_or_email.contains('@') {
                    store
                        .member_by_email(&organization_id, &input.member_id_or_email)
                        .await?
                } else {
                    store.member_by_id(&input.member_id_or_email).await?
                };
                let Some(target) = target else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if target.organization_id != organization_id {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                }
                if is_last_owner(&store, &organization_id, &target, &options).await? {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "YOU_CANNOT_LEAVE_THE_ORGANIZATION_AS_THE_ONLY_OWNER",
                    );
                }
                if !has_permission(
                    &actor_member.role,
                    &options,
                    OrganizationPermission::MemberDelete,
                ) {
                    return http::organization_error(
                        StatusCode::UNAUTHORIZED,
                        "YOU_ARE_NOT_ALLOWED_TO_DELETE_THIS_MEMBER",
                    );
                }
                store.delete_member(&target.id).await?;
                if target.user_id == session.user.id
                    && session.active_organization_id.as_deref() == Some(&target.organization_id)
                {
                    store
                        .set_active_organization(&session.session.token, None)
                        .await?;
                }
                http::json(StatusCode::OK, &serde_json::json!({ "member": target }))
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateMemberRoleBody {
    member_id: String,
    role: RoleInput,
    #[serde(default)]
    organization_id: Option<String>,
}

fn update_member_role(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/update-member-role",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: UpdateMemberRoleBody = http::body(&request)?;
                let Some(organization_id) = super::resolve_organization_id(
                    input.organization_id,
                    session.active_organization_id.as_deref(),
                ) else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let Some(actor_member) = store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                let Some(target) = store.member_by_id(&input.member_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND");
                };
                if target.organization_id != organization_id {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_UPDATE_THIS_MEMBER",
                    );
                }
                let next_role = input.role.normalized();
                if target.user_id == session.user.id
                    && target
                        .role
                        .split(',')
                        .any(|role| role.trim() == options.creator_role)
                    && !next_role
                        .split(',')
                        .any(|role| role.trim() == options.creator_role)
                    && owners(&store, &organization_id, &options).await? <= 1
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "YOU_CANNOT_LEAVE_THE_ORGANIZATION_WITHOUT_AN_OWNER",
                    );
                }
                if !has_permission(
                    &actor_member.role,
                    &options,
                    OrganizationPermission::MemberUpdate,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_UPDATE_THIS_MEMBER",
                    );
                }
                match store.update_member_role(&target.id, &next_role).await? {
                    Some(member) => http::json(StatusCode::OK, &member),
                    None => http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND"),
                }
            })
        },
    )
}

fn get_active_member() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-active-member",
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
                match store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                {
                    Some(member) => http::json(StatusCode::OK, &member),
                    None => http::organization_error(StatusCode::BAD_REQUEST, "MEMBER_NOT_FOUND"),
                }
            })
        },
    )
}

fn leave(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/leave",
        Method::POST,
        AuthEndpointOptions::new(),
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
                store.delete_member(&member.id).await?;
                store
                    .set_active_organization(&session.session.token, None)
                    .await?;
                http::json(StatusCode::OK, &serde_json::json!({ "member": member }))
            })
        },
    )
}

fn list_members() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-members",
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
                http::json(
                    StatusCode::OK,
                    &serde_json::json!({ "members": store.members(&organization_id).await? }),
                )
            })
        },
    )
}

fn get_active_member_role() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-active-member-role",
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

async fn is_last_owner(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    member: &crate::organization::Member,
    options: &OrganizationOptions,
) -> Result<bool, openauth_core::error::OpenAuthError> {
    Ok(member
        .role
        .split(',')
        .any(|role| role.trim() == options.creator_role)
        && owners(store, organization_id, options).await? <= 1)
}

async fn owners(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    options: &OrganizationOptions,
) -> Result<usize, openauth_core::error::OpenAuthError> {
    Ok(store
        .members(organization_id)
        .await?
        .iter()
        .filter(|member| {
            member
                .role
                .split(',')
                .any(|role| role.trim() == options.creator_role)
        })
        .count())
}
