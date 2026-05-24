use serde::Serialize;

use super::*;

pub(super) fn metadata_endpoint(
    path: &'static str,
    options: Arc<ResolvedOAuthProviderOptions>,
    oidc: bool,
) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        path,
        Method::GET,
        AuthEndpointOptions::new(),
        move |context, _request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                if oidc {
                    if !options.scopes.contains(&"openid".to_owned()) {
                        return error_response(OAuthProviderError::new(
                            StatusCode::NOT_FOUND,
                            "not_found",
                            "OpenID Connect is disabled",
                        ));
                    }
                    metadata_response(&oidc_server_metadata(context, &options))
                } else {
                    metadata_response(&auth_server_metadata(context, &options))
                }
            })
        },
    )
}

const METADATA_CACHE_CONTROL: &str =
    "public, max-age=15, stale-while-revalidate=15, stale-if-error=86400";

fn metadata_response<T: Serialize>(body: &T) -> Result<ApiResponse, OpenAuthError> {
    let body = serde_json::to_vec(body).map_err(|error| OpenAuthError::Api(error.to_string()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, METADATA_CACHE_CONTROL)
        .body(body)
        .map_err(|error| OpenAuthError::Api(error.to_string()))
}
