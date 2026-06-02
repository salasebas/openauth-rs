use super::*;

pub(super) fn metadata_endpoint(
    path: &'static str,
    options: Arc<ResolvedOAuthProviderOptions>,
    mode: MetadataEndpointMode,
) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        path,
        Method::GET,
        AuthEndpointOptions::new(),
        move |context, _request| {
            let options = Arc::clone(&options);
            Box::pin(async move {
                match mode {
                    MetadataEndpointMode::OpenIdConfiguration => {
                        if !options.scopes.contains(&"openid".to_owned()) {
                            return error_response(OAuthProviderError::new(
                                StatusCode::NOT_FOUND,
                                "not_found",
                                "OpenID Connect is disabled",
                            ));
                        }
                        well_known_metadata_response(&oidc_server_metadata(context, &options))
                    }
                    MetadataEndpointMode::OAuthAuthorizationServer => {
                        if options.scopes.contains(&"openid".to_owned()) {
                            well_known_metadata_response(&oidc_server_metadata(context, &options))
                        } else {
                            well_known_metadata_response(&auth_server_metadata(context, &options))
                        }
                    }
                }
            })
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MetadataEndpointMode {
    OAuthAuthorizationServer,
    OpenIdConfiguration,
}
