use ::http::{Method, StatusCode};
use openauth_core::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions};
use serde::Deserialize;

use crate::organization::http;
use crate::organization::options::OrganizationOptions;
use crate::organization::permissions::{has_permission, OrganizationPermission};
use crate::organization::store::OrganizationStore;

pub fn endpoints(options: OrganizationOptions) -> Vec<AsyncAuthEndpoint> {
    if !options.teams.enabled {
        return Vec::new();
    }
    vec![
        create_team(options.clone()),
        list_teams(),
        remove_team(options.clone()),
        update_team(options.clone()),
        set_active_team(),
        list_user_teams(),
        list_team_members(),
        add_team_member(options.clone()),
        remove_team_member(options),
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamBody {
    name: String,
    #[serde(default)]
    organization_id: Option<String>,
}

fn create_team(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/create-team",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: TeamBody = http::body(&request)?;
                if input.name.trim().is_empty() {
                    return http::error(
                        StatusCode::BAD_REQUEST,
                        "INVALID_REQUEST_BODY",
                        "Invalid request body",
                    );
                }
                let Some(organization_id) = super::resolve_organization_id(
                    input.organization_id,
                    session.active_organization_id.as_deref(),
                ) else {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "NO_ACTIVE_ORGANIZATION",
                    );
                };
                let actor = require_member(&store, &organization_id, &session.user.id).await?;
                if !has_permission(&actor.role, &options, OrganizationPermission::TeamCreate) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_CREATE_TEAMS_IN_THIS_ORGANIZATION",
                    );
                }
                if let Some(max) = options.teams.maximum_teams {
                    if store.teams_for_organization(&organization_id).await?.len() >= max {
                        return http::organization_error(
                            StatusCode::BAD_REQUEST,
                            "YOU_HAVE_REACHED_THE_MAXIMUM_NUMBER_OF_TEAMS",
                        );
                    }
                }
                let team = store
                    .create_team(&organization_id, input.name.trim())
                    .await?;
                store.create_team_member(&team.id, &session.user.id).await?;
                http::json(StatusCode::OK, &team)
            })
        },
    )
}

fn list_teams() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-teams",
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
                require_member(&store, &organization_id, &session.user.id).await?;
                http::json(
                    StatusCode::OK,
                    &store.teams_for_organization(&organization_id).await?,
                )
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamIdBody {
    team_id: String,
    #[serde(default)]
    organization_id: Option<String>,
}

fn remove_team(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/remove-team",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: TeamIdBody = http::body(&request)?;
                let Some(team) = store.team_by_id(&input.team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                let organization_id = input
                    .organization_id
                    .unwrap_or_else(|| team.organization_id.clone());
                if team.organization_id != organization_id {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                }
                let actor = require_member(&store, &organization_id, &session.user.id).await?;
                if !has_permission(&actor.role, &options, OrganizationPermission::TeamDelete) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_DELETE_THIS_TEAM",
                    );
                }
                if !options.teams.allow_removing_all_teams
                    && store.teams_for_organization(&organization_id).await?.len() <= 1
                {
                    return http::organization_error(
                        StatusCode::BAD_REQUEST,
                        "UNABLE_TO_REMOVE_LAST_TEAM",
                    );
                }
                store.delete_team(&team.id).await?;
                http::json(StatusCode::OK, &serde_json::json!({ "team": team }))
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTeamBody {
    team_id: String,
    name: String,
}

fn update_team(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/update-team",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: UpdateTeamBody = http::body(&request)?;
                let Some(team) = store.team_by_id(&input.team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                let actor = require_member(&store, &team.organization_id, &session.user.id).await?;
                if !has_permission(&actor.role, &options, OrganizationPermission::TeamUpdate) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_UPDATE_THIS_TEAM",
                    );
                }
                let updated = store.update_team(&team.id, input.name.trim()).await?;
                http::json(StatusCode::OK, &updated)
            })
        },
    )
}

fn set_active_team() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/set-active-team",
        Method::POST,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: TeamIdBody = http::body(&request)?;
                let Some(team) = store.team_by_id(&input.team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                require_member(&store, &team.organization_id, &session.user.id).await?;
                if store
                    .team_member(&team.id, &session.user.id)
                    .await?
                    .is_none()
                {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "USER_IS_NOT_A_MEMBER_OF_THE_TEAM",
                    );
                }
                store
                    .set_active_team(&session.session.token, Some(&team.id))
                    .await?;
                http::json(StatusCode::OK, &team)
            })
        },
    )
}

fn list_user_teams() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-user-teams",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let mut teams = Vec::new();
                for organization in store.organizations_for_user(&session.user.id).await? {
                    for team in store.teams_for_organization(&organization.id).await? {
                        if store
                            .team_member(&team.id, &session.user.id)
                            .await?
                            .is_some()
                        {
                            teams.push(team);
                        }
                    }
                }
                http::json(StatusCode::OK, &teams)
            })
        },
    )
}

fn list_team_members() -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/list-team-members",
        Method::GET,
        AuthEndpointOptions::new(),
        |context, request| {
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let Some(team_id) =
                    query_param(&request, "teamId").or_else(|| query_param(&request, "team_id"))
                else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                let Some(team) = store.team_by_id(&team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                require_member(&store, &team.organization_id, &session.user.id).await?;
                http::json(StatusCode::OK, &store.team_members(&team.id).await?)
            })
        },
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamMemberBody {
    team_id: String,
    user_id: String,
}

fn add_team_member(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/add-team-member",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: TeamMemberBody = http::body(&request)?;
                let Some(team) = store.team_by_id(&input.team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                let actor = require_member(&store, &team.organization_id, &session.user.id).await?;
                if !has_permission(&actor.role, &options, OrganizationPermission::TeamUpdate) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_CREATE_A_NEW_TEAM_MEMBER",
                    );
                }
                require_member(&store, &team.organization_id, &input.user_id).await?;
                if let Some(max) = options.teams.maximum_members_per_team {
                    if store.count_team_members(&team.id).await? as usize >= max {
                        return http::organization_error(
                            StatusCode::FORBIDDEN,
                            "TEAM_MEMBER_LIMIT_REACHED",
                        );
                    }
                }
                if let Some(existing) = store.team_member(&team.id, &input.user_id).await? {
                    return http::json(StatusCode::OK, &existing);
                }
                http::json(
                    StatusCode::OK,
                    &store.create_team_member(&team.id, &input.user_id).await?,
                )
            })
        },
    )
}

fn remove_team_member(options: OrganizationOptions) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/organization/remove-team-member",
        Method::POST,
        AuthEndpointOptions::new(),
        move |context, request| {
            let options = options.clone();
            Box::pin(async move {
                let adapter = http::adapter(context)?;
                let store = OrganizationStore::new(adapter.as_ref());
                let session = require_session(context, &request, &store).await?;
                let input: TeamMemberBody = http::body(&request)?;
                let Some(team) = store.team_by_id(&input.team_id).await? else {
                    return http::organization_error(StatusCode::BAD_REQUEST, "TEAM_NOT_FOUND");
                };
                let actor = require_member(&store, &team.organization_id, &session.user.id).await?;
                if !has_permission(&actor.role, &options, OrganizationPermission::TeamUpdate) {
                    return http::organization_error(
                        StatusCode::FORBIDDEN,
                        "YOU_ARE_NOT_ALLOWED_TO_REMOVE_A_TEAM_MEMBER",
                    );
                }
                store.delete_team_member(&team.id, &input.user_id).await?;
                http::json(StatusCode::OK, &serde_json::json!({ "status": true }))
            })
        },
    )
}

async fn require_session(
    context: &openauth_core::context::AuthContext,
    request: &openauth_core::api::ApiRequest,
    store: &OrganizationStore<'_>,
) -> Result<http::CurrentSession, openauth_core::error::OpenAuthError> {
    http::current_session(context, request, store)
        .await?
        .ok_or_else(|| openauth_core::error::OpenAuthError::Api("Unauthorized".to_owned()))
}

fn query_param(request: &openauth_core::api::ApiRequest, name: &str) -> Option<String> {
    request.uri().query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
    })
}

async fn require_member(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    user_id: &str,
) -> Result<crate::organization::Member, openauth_core::error::OpenAuthError> {
    store
        .member_by_org_user(organization_id, user_id)
        .await?
        .ok_or_else(|| openauth_core::error::OpenAuthError::Api("Member not found".to_owned()))
}
