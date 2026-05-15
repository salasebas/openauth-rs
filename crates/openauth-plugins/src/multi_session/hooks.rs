use openauth_core::context::request_state::current_new_session;
use openauth_core::context::AuthContext;
use openauth_core::db::DbAdapter;
use openauth_core::error::OpenAuthError;
use openauth_core::plugin::{PluginAfterHookAction, PluginAfterHookFuture};
use openauth_core::session::DbSessionStore;

use super::cookies::{
    append_cookies, expire_multi_cookie_name, multi_cookie_name, multi_session_cookie,
    signed_multi_tokens,
};
use super::options::MultiSessionConfig;

pub fn store_multi_session_cookie(
    config: MultiSessionConfig,
) -> impl for<'a> Fn(
    &'a AuthContext,
    &'a openauth_core::api::ApiRequest,
    openauth_core::api::ApiResponse,
) -> PluginAfterHookFuture<'a>
       + Send
       + Sync
       + 'static {
    move |context, request, response| {
        Box::pin(async move {
            store_multi_session_cookie_inner(context, request, response, config).await
        })
    }
}

pub fn revoke_multi_session_cookies() -> impl for<'a> Fn(
    &'a AuthContext,
    &'a openauth_core::api::ApiRequest,
    openauth_core::api::ApiResponse,
) -> PluginAfterHookFuture<'a>
       + Send
       + Sync
       + 'static {
    move |context, request, response| {
        Box::pin(
            async move { revoke_multi_session_cookies_inner(context, request, response).await },
        )
    }
}

async fn store_multi_session_cookie_inner(
    context: &AuthContext,
    request: &openauth_core::api::ApiRequest,
    mut response: openauth_core::api::ApiResponse,
    config: MultiSessionConfig,
) -> Result<PluginAfterHookAction, OpenAuthError> {
    let Some(created) = current_new_session()? else {
        return Ok(PluginAfterHookAction::Continue(response));
    };
    let cookie_header = request_cookie_header(request);
    let cookie_name = multi_cookie_name(context, &created.session.token);
    if response_has_cookie(&response, &cookie_name)
        || request_has_cookie(&cookie_header, &cookie_name)
    {
        return Ok(PluginAfterHookAction::Continue(response));
    }
    let Some(adapter) = context.adapter() else {
        return Ok(PluginAfterHookAction::Continue(response));
    };

    let current_signed_count = signed_multi_tokens(context, &cookie_header)?.len();
    let deleted = delete_same_user_sessions(
        adapter.as_ref(),
        context,
        &cookie_header,
        &created.user.id,
        &mut response,
    )
    .await?;
    let current_count = current_signed_count.saturating_sub(deleted) + 1;
    if current_count > config.maximum_sessions {
        return Ok(PluginAfterHookAction::Continue(response));
    }
    append_cookies(
        &mut response,
        [multi_session_cookie(context, &created.session.token)?],
    )?;
    Ok(PluginAfterHookAction::Continue(response))
}

async fn revoke_multi_session_cookies_inner(
    context: &AuthContext,
    request: &openauth_core::api::ApiRequest,
    mut response: openauth_core::api::ApiResponse,
) -> Result<PluginAfterHookAction, OpenAuthError> {
    let Some(adapter) = context.adapter() else {
        return Ok(PluginAfterHookAction::Continue(response));
    };
    let cookie_header = request_cookie_header(request);
    let tokens = signed_multi_tokens(context, &cookie_header)?;
    for (key, token) in &tokens {
        append_cookies(&mut response, [expire_multi_cookie_name(context, key)])?;
        DbSessionStore::new(adapter.as_ref())
            .delete_session(token)
            .await?;
    }
    Ok(PluginAfterHookAction::Continue(response))
}

async fn delete_same_user_sessions(
    adapter: &dyn DbAdapter,
    context: &AuthContext,
    cookie_header: &str,
    user_id: &str,
    response: &mut openauth_core::api::ApiResponse,
) -> Result<usize, OpenAuthError> {
    let mut deleted = 0;
    for (key, token) in signed_multi_tokens(context, cookie_header)? {
        let Some(session) = DbSessionStore::new(adapter).find_session(&token).await? else {
            continue;
        };
        if session.user_id != user_id {
            continue;
        }
        DbSessionStore::new(adapter).delete_session(&token).await?;
        append_cookies(response, [expire_multi_cookie_name(context, &key)])?;
        deleted += 1;
    }
    Ok(deleted)
}

fn request_cookie_header(request: &openauth_core::api::ApiRequest) -> String {
    request
        .headers()
        .get(http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

fn request_has_cookie(cookie_header: &str, name: &str) -> bool {
    openauth_core::cookies::parse_cookies(cookie_header).contains_key(name)
}

fn response_has_cookie(response: &openauth_core::api::ApiResponse, name: &str) -> bool {
    response
        .headers()
        .get_all(http::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|cookie| cookie.starts_with(&format!("{name}=")))
}
