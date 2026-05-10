use std::sync::Arc;

use http::{Method, StatusCode};

use super::shared::{json_response, request_cookie_header, sign_out_openapi_response};
use crate::api::{create_auth_endpoint, AsyncAuthEndpoint, AuthEndpointOptions, OpenApiOperation};
use crate::auth::session::SessionAuth;
use crate::db::DbAdapter;

pub(super) fn sign_out_endpoint(adapter: Arc<dyn DbAdapter>) -> AsyncAuthEndpoint {
    create_auth_endpoint(
        "/sign-out",
        Method::POST,
        AuthEndpointOptions::new().operation_id("signOut").openapi(
            OpenApiOperation::new("signOut")
                .description("Sign out the current user")
                .response("200", sign_out_openapi_response()),
        ),
        move |context, request| {
            let adapter = Arc::clone(&adapter);
            Box::pin(async move {
                let cookie_header = request_cookie_header(&request).unwrap_or_default();
                let result = SessionAuth::new(adapter.as_ref(), context)
                    .sign_out(cookie_header)
                    .await?;
                json_response(StatusCode::OK, &result, result.cookies.clone())
            })
        },
    )
}
