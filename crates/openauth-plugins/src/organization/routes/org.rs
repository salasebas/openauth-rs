use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint};
use openauth_core::error::OpenAuthError;
use serde::Deserialize;

use crate::organization::additional_fields;
use crate::organization::hooks::{
    AfterAddMember, AfterCreateOrganization, AfterDeleteOrganization, AfterUpdateOrganization,
    BeforeAddMember, BeforeCreateOrganization, BeforeDeleteOrganization, BeforeUpdateOrganization,
    MemberHookData, OrganizationUpdateData,
};
use crate::organization::http;
use crate::organization::models::{FullOrganization, Organization};
use crate::organization::options::OrganizationOptions;
use crate::organization::permissions::{has_permission, OrganizationPermission};
use crate::organization::store::{OrganizationStore, OrganizationUpdate};

use super::validation::invalid_body;

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    vec![
        create(options.clone()),
        check_slug(),
        update(options.clone()),
        delete(options.clone()),
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
        super::metadata::options(
            "organizationCreate",
            vec![
                super::metadata::string("name"),
                super::metadata::string("slug"),
                super::metadata::optional_string("userId"),
                super::metadata::optional_string("logo"),
                super::metadata::optional_object("metadata"),
                super::metadata::optional_bool("keepCurrentActiveOrganization"),
            ],
        ),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let body: serde_json::Value = http::body(&request)?;
                let input: CreateBody =
                    serde_json::from_value(body.clone()).map_err(json_body_error)?;
                let additional_fields = additional_fields::create_values(
                    &options.schema.organization.additional_fields,
                    body.as_object().ok_or_else(|| {
                        OpenAuthError::Api("request body must be an object".to_owned())
                    })?,
                )?;
                if input.name.trim().is_empty() || input.slug.trim().is_empty() {
                    return invalid_body();
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

                let mut organization = store
                    .create_organization(
                        input.name,
                        input.slug,
                        input.logo,
                        input.metadata,
                        additional_fields,
                    )
                    .await?;
                retain_returned_organization_fields(&mut organization, &options);
                let mut creator_member = MemberHookData {
                    organization_id: organization.id.clone(),
                    user_id: user.id.clone(),
                    role: options.creator_role.clone(),
                };
                if let Some(hook) = &options.hooks.before_add_member {
                    creator_member = hook(&BeforeAddMember {
                        organization: organization.clone(),
                        user: user.clone(),
                        member: creator_member,
                    })?;
                }
                let member = store
                    .create_member(
                        &creator_member.organization_id,
                        &creator_member.user_id,
                        &creator_member.role,
                        openauth_core::db::DbRecord::new(),
                    )
                    .await?;
                if options.teams.enabled && options.teams.create_default_team {
                    let team = store
                        .create_team(
                            &organization.id,
                            "Default",
                            openauth_core::db::DbRecord::new(),
                        )
                        .await?;
                    store
                        .create_team_member(&team.id, &user.id, openauth_core::db::DbRecord::new())
                        .await?;
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
                let cookies = if let Some(session) = &session {
                    if !input.keep_current_active_organization {
                        store
                            .set_active_organization(&session.session.token, Some(&organization.id))
                            .await?;
                        http::refreshed_session_cookies(context, &session.session, &session.user)?
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                http::json_with_cookies(
                    StatusCode::OK,
                    &FullOrganization {
                        organization,
                        members: vec![member],
                        invitations: Vec::new(),
                        teams: Vec::new(),
                    },
                    cookies,
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
        super::metadata::options(
            "organizationCheckSlug",
            vec![super::metadata::string("slug")],
        ),
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
        super::metadata::options(
            "organizationUpdate",
            vec![
                super::metadata::object("data"),
                super::metadata::optional_string("organizationId"),
            ],
        ),
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
                let body: serde_json::Value = http::body(&request)?;
                let input: UpdateBody =
                    serde_json::from_value(body.clone()).map_err(json_body_error)?;
                let additional_fields = body
                    .get("data")
                    .and_then(serde_json::Value::as_object)
                    .map(|data| {
                        additional_fields::update_values(
                            &options.schema.organization.additional_fields,
                            data,
                        )
                    })
                    .transpose()?
                    .unwrap_or_default();
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
                let Some(existing_organization) =
                    store.organization_by_id(&organization_id).await?
                else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "ORGANIZATION_NOT_FOUND",
                    );
                };
                let mut data = OrganizationUpdateData {
                    name: input.data.name,
                    slug: input.data.slug,
                    logo: input.data.logo,
                    metadata: input.data.metadata,
                };
                if let Some(hook) = &options.hooks.before_update_organization {
                    data = hook(&BeforeUpdateOrganization {
                        organization: existing_organization.clone(),
                        user: session.user.clone(),
                        data,
                    })?;
                }
                if let Some(slug) = &data.slug {
                    if slug.trim().is_empty() {
                        return invalid_body();
                    }
                    if let Some(existing) = store.organization_by_slug(slug).await? {
                        if existing.id != organization_id {
                            return http::organization_error(
                                StatusCode::BAD_REQUEST,
                                "ORGANIZATION_SLUG_ALREADY_TAKEN",
                            );
                        }
                    }
                }
                if let Some(name) = &data.name {
                    if name.trim().is_empty() {
                        return invalid_body();
                    }
                }
                let update = OrganizationUpdate {
                    name: data.name,
                    slug: data.slug,
                    logo: data.logo,
                    logo_set: true,
                    metadata: data.metadata,
                    metadata_set: true,
                    additional_fields,
                };
                match store.update_organization(&organization_id, update).await? {
                    Some(mut organization) => {
                        retain_returned_organization_fields(&mut organization, &options);
                        if let Some(hook) = &options.hooks.after_update_organization {
                            hook(&AfterUpdateOrganization {
                                organization: organization.clone(),
                                user: session.user,
                            })?;
                        }
                        http::json(StatusCode::OK, &organization)
                    }
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
        super::metadata::options(
            "organizationDelete",
            vec![super::metadata::string("organizationId")],
        ),
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
                if let Some(hook) = &options.hooks.before_delete_organization {
                    hook(&BeforeDeleteOrganization {
                        organization: organization.clone(),
                        user: session.user.clone(),
                    })?;
                }
                let cookies =
                    if session.active_organization_id.as_deref() == Some(&input.organization_id) {
                        store
                            .set_active_organization(&session.session.token, None)
                            .await?;
                        store.set_active_team(&session.session.token, None).await?;
                        http::refreshed_session_cookies(context, &session.session, &session.user)?
                    } else {
                        Vec::new()
                    };
                store.delete_organization(&input.organization_id).await?;
                if let Some(hook) = &options.hooks.after_delete_organization {
                    hook(&AfterDeleteOrganization {
                        organization: organization.clone(),
                        user: session.user,
                    })?;
                }
                http::json_with_cookies(StatusCode::OK, &organization, cookies)
            })
        },
    )
}

fn retain_returned_organization_fields(
    organization: &mut Organization,
    options: &OrganizationOptions,
) {
    let fields = &options.schema.organization.additional_fields;
    additional_fields::retain_returned(&mut organization.additional_fields, fields);
}

fn json_body_error(error: serde_json::Error) -> OpenAuthError {
    OpenAuthError::Api(error.to_string())
}
