use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use serde::Deserialize;

use crate::organization::hooks::{
    AfterAddMember, AfterCreateOrganization, BeforeAddMember, BeforeCreateOrganization,
};
use crate::organization::http;
use crate::organization::models::FullOrganization;
use crate::organization::options::OrganizationOptions;
use crate::organization::permissions::{has_permission, OrganizationPermission};
use crate::organization::store::{OrganizationStore, OrganizationUpdate};

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    vec![
        create(options.clone()),
        check_slug(),
        update(options.clone()),
        delete(options.clone()),
        set_active(),
        get_full(),
        list(),
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody {
    name: String,
    slug: String,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    logo: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    keep_current_active_organization: bool,
}

fn create(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/create",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let input: CreateBody = http::body(&request)?;
                if input.name.trim().is_empty() || input.slug.trim().is_empty() {
                    return http::error(
                        StatusCode::BAD_REQUEST,
                        "INVALID_REQUEST_BODY",
                        "Invalid request body",
                    );
                }

                let session = http::current_session(context, &request, &store).await?;
                let user = match (session.as_ref(), input.user_id.as_deref()) {
                    (Some(session), _) => session.user.clone(),
                    (None, Some(user_id)) => match store.user_by_id(user_id).await? {
                        Some(user) => user,
                        None => {
                            return http::error(
                                StatusCode::UNAUTHORIZED,
                                "UNAUTHORIZED",
                                "Unauthorized",
                            )
                        }
                    },
                    (None, None) => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };

                if !options.allow_user_to_create_organization && session.is_some() {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_CREATE_A_NEW_ORGANIZATION",
                    );
                }
                if let Some(limit) = options.organization_limit {
                    if store.organizations_for_user(&user.id).await?.len() >= limit {
                        return http::organization_error(
                            StatusCode::FORBIDDEN,
                            "YOU_HAVE_REACHED_THE_MAXIMUM_NUMBER_OF_ORGANIZATIONS",
                        );
                    }
                }
                if store.organization_by_slug(&input.slug).await?.is_some() {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_ALREADY_EXISTS",
                    );
                }

                if let Some(hook) = &options.hooks.before_create_organization {
                    hook(&BeforeCreateOrganization {
                        name: input.name.clone(),
                        slug: input.slug.clone(),
                        user: user.clone(),
                    })?;
                }

                let organization = store
                    .create_organization(input.name, input.slug, input.logo, input.metadata)
                    .await?;
                if let Some(hook) = &options.hooks.before_add_member {
                    hook(&BeforeAddMember {
                        organization: organization.clone(),
                        user: user.clone(),
                        role: options.creator_role.clone(),
                    })?;
                }
                let member = store
                    .create_member(&organization.id, &user.id, &options.creator_role)
                    .await?;
                if options.teams.enabled && options.teams.create_default_team {
                    let team = store.create_team(&organization.id, "Default").await?;
                    store.create_team_member(&team.id, &user.id).await?;
                }
                if let Some(hook) = &options.hooks.after_add_member {
                    hook(&AfterAddMember {
                        organization: organization.clone(),
                        member: member.clone(),
                        user: user.clone(),
                    })?;
                }
                if let Some(hook) = &options.hooks.after_create_organization {
                    hook(&AfterCreateOrganization {
                        organization: organization.clone(),
                        member: member.clone(),
                        user: user.clone(),
                    })?;
                }
                if let Some(session) = &session {
                    if !input.keep_current_active_organization {
                        store
                            .set_active_organization(&session.session.token, Some(&organization.id))
                            .await?;
                    }
                }

                http::json(
                    StatusCode::OK,
                    &FullOrganization {
                        organization,
                        members: vec![member],
                        invitations: Vec::new(),
                        teams: Vec::new(),
                    },
                )
            })
        },
    )
}

#[derive(Debug, Deserialize)]
struct CheckSlugBody {
    slug: String,
}

fn check_slug() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/check-slug",
        Method::POST,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let input: CheckSlugBody = http::body(&request)?;
                if store.organization_by_slug(&input.slug).await?.is_some() {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_SLUG_ALREADY_TAKEN",
                    );
                }
                http::json(StatusCode::OK, &serde_json::json!({ "status": true }))
            })
        },
    )
}

#[derive(Debug, Deserialize, Default)]
struct UpdateData {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    logo: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBody {
    data: UpdateData,
    #[serde(default)]
    organization_id: Option<String>,
}

fn update(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/update",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = match http::current_session(context, &request, &store).await? {
                    Some(session) => session,
                    None => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };
                let input: UpdateBody = http::body(&request)?;
                let Some(organization_id) = super::resolve_organization_id(
                    input.organization_id,
                    session.active_organization_id.as_deref(),
                ) else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                let Some(member) = store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_NOT_A_MEMBER_OF_THE_ORGANIZATION",
                    );
                };
                if !has_permission(
                    &member.role,
                    &options,
                    OrganizationPermission::OrganizationUpdate,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_UPDATE_THIS_ORGANIZATION",
                    );
                }
                if let Some(slug) = &input.data.slug {
                    if let Some(existing) = store.organization_by_slug(slug).await? {
                        if existing.id != organization_id {
                            return http::organization_error(
                                StatusCode::BAD_REQUEST,
                                "ORGANIZATION_SLUG_ALREADY_TAKEN",
                            );
                        }
                    }
                }
                let update = OrganizationUpdate {
                    name: input.data.name,
                    slug: input.data.slug,
                    logo: input.data.logo,
                    logo_set: true,
                    metadata: input.data.metadata,
                    metadata_set: true,
                };
                match store.update_organization(&organization_id, update).await? {
                    Some(organization) => http::json(StatusCode::OK, &organization),
                    None => {
                        http::organization_error(StatusCode::BAD_REQUEST, "ORGANIZATION_NOT_FOUND")
                    }
                }
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrganizationIdBody {
    organization_id: String,
}

fn delete(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/delete",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                if options.disable_organization_deletion {
                    return http::error(
                        StatusCode::NOT_FOUND,
                        "ORGANIZATION_DELETION_DISABLED",
                        "Organization deletion is disabled",
                    );
                }
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = match http::current_session(context, &request, &store).await? {
                    Some(session) => session,
                    None => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };
                let input: OrganizationIdBody = http::body(&request)?;
                let Some(member) = store
                    .member_by_org_user(&input.organization_id, &session.user.id)
                    .await?
                else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_NOT_A_MEMBER_OF_THE_ORGANIZATION",
                    );
                };
                if !has_permission(
                    &member.role,
                    &options,
                    OrganizationPermission::OrganizationDelete,
                ) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_DELETE_THIS_ORGANIZATION",
                    );
                }
                let Some(organization) = store.organization_by_id(&input.organization_id).await?
                else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                if session.active_organization_id.as_deref() == Some(&input.organization_id) {
                    store
                        .set_active_organization(&session.session.token, None)
                        .await?;
                }
                store.delete_organization(&input.organization_id).await?;
                http::json(StatusCode::OK, &organization)
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetActiveBody {
    #[serde(default)]
    organization_id: Option<String>,
}

fn set_active() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/set-active",
        Method::POST,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = match http::current_session(context, &request, &store).await? {
                    Some(session) => session,
                    None => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };
                let input: SetActiveBody = http::body(&request)?;
                if let Some(organization_id) = &input.organization_id {
                    if store
                        .member_by_org_user(organization_id, &session.user.id)
                        .await?
                        .is_none()
                    {
                        return http::organization_error(
                            StatusCode::BAD_REQUEST,
                            "USER_IS_NOT_A_MEMBER_OF_THE_ORGANIZATION",
                        );
                    }
                }
                store
                    .set_active_organization(
                        &session.session.token,
                        input.organization_id.as_deref(),
                    )
                    .await?;
                http::json(StatusCode::OK, &serde_json::json!({ "success": true }))
            })
        },
    )
}

fn get_full() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/get-full-organization",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = match http::current_session(context, &request, &store).await? {
                    Some(session) => session,
                    None => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };
                let Some(organization_id) = session.active_organization_id else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let Some(organization) = store.organization_by_id(&organization_id).await? else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                if store
                    .member_by_org_user(&organization_id, &session.user.id)
                    .await?
                    .is_none()
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "USER_IS_NOT_A_MEMBER_OF_THE_ORGANIZATION",
                    );
                }
                let teams = store
                    .teams_for_organization(&organization_id)
                    .await
                    .unwrap_or_default();
                http::json(
                    StatusCode::OK,
                    &FullOrganization {
                        organization,
                        members: store.members(&organization_id).await?,
                        invitations: store.invitations_for_organization(&organization_id).await?,
                        teams,
                    },
                )
            })
        },
    )
}

fn list() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = match http::current_session(context, &request, &store).await? {
                    Some(session) => session,
                    None => {
                        return http::error(
                            StatusCode::UNAUTHORIZED,
                            "UNAUTHORIZED",
                            "Unauthorized",
                        )
                    }
                };
                let organizations = store.organizations_for_user(&session.user.id).await?;
                http::json(StatusCode::OK, &organizations)
            })
        },
    )
}
