use ::http::StatusCode;
use openauth_core::error::OpenAuthError;

use crate::organization::http;
use crate::organization::models::Member;
use crate::organization::options::OrganizationOptions;
use crate::organization::permissions::is_known_static_role;
use crate::organization::store::OrganizationStore;

pub(super) async fn require_session(
    context: &openauth_core::context::AuthContext,
    request: &openauth_core::api::ApiRequest,
    store: &OrganizationStore<'_>,
) -> Result<crate::organization::http::CurrentSession, OpenAuthError> {
    match http::current_session(context, request, store).await? {
        Some(session) => Ok(session),
        None => Err(OpenAuthError::Api("UNAUTHORIZED".to_owned())),
    }
}

pub(super) fn query_param(request: &openauth_core::api::ApiRequest, name: &str) -> Option<String> {
    request.uri().query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then(|| value.to_owned())
        })
    })
}

pub(super) fn valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(char::is_whitespace)
}

pub(super) async fn roles_exist(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    roles: &str,
    options: &OrganizationOptions,
) -> Result<bool, OpenAuthError> {
    for role in roles
        .split(',')
        .map(str::trim)
        .filter(|role| !role.is_empty())
    {
        if is_known_static_role(role, options) {
            continue;
        }
        if !options.dynamic_access_control.enabled {
            return Ok(false);
        }
        if store
            .organization_role_by_name(organization_id, role)
            .await?
            .is_none()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) async fn is_last_owner(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    member: &Member,
    options: &OrganizationOptions,
) -> Result<bool, OpenAuthError> {
    Ok(member
        .role
        .split(',')
        .any(|role| role.trim() == options.creator_role)
        && owners(store, organization_id, options).await? <= 1)
}

pub(super) async fn owners(
    store: &OrganizationStore<'_>,
    organization_id: &str,
    options: &OrganizationOptions,
) -> Result<usize, OpenAuthError> {
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

pub(super) fn invalid_body() -> Result<openauth_core::api::ApiResponse, OpenAuthError> {
    http::error(
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST_BODY",
        "Invalid request body",
    )
}
