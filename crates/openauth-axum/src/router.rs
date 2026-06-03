use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, Request, Uri};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use openauth::{
    auth::oauth::OAuthBaseUrlOverride,
    utils::host::is_loopback_host,
    utils::url::{is_valid_forwarded_host, is_valid_forwarded_proto},
    OpenAuth, OpenAuthError, RequestBaseUrl,
};

use crate::error::{internal_error_response, OpenAuthAxumError};
use crate::request::to_api_request;
use crate::response::from_api_response;
use crate::OpenAuthAxumOptions;

#[derive(Clone)]
struct OpenAuthAxumState {
    auth: Arc<OpenAuth>,
    options: OpenAuthAxumOptions,
}

/// Convenience extension methods for mounting OpenAuth into Axum.
pub trait OpenAuthAxumExt {
    /// Mount OpenAuth at `OpenAuthOptions.base_path`, defaulting to `/api/auth`.
    fn into_router(self) -> Result<Router, OpenAuthAxumError>;

    /// Mount OpenAuth with adapter-specific options.
    fn into_router_with_options(
        self,
        options: OpenAuthAxumOptions,
    ) -> Result<Router, OpenAuthAxumError>;

    /// Return unmounted OpenAuth routes for callers that want to nest manually.
    fn into_routes(self) -> Router;

    /// Return unmounted OpenAuth routes with adapter-specific options.
    fn into_routes_with_options(self, options: OpenAuthAxumOptions) -> Router;
}

impl OpenAuthAxumExt for OpenAuth {
    fn into_router(self) -> Result<Router, OpenAuthAxumError> {
        router(self)
    }

    fn into_router_with_options(
        self,
        options: OpenAuthAxumOptions,
    ) -> Result<Router, OpenAuthAxumError> {
        router_with_options(self, options)
    }

    fn into_routes(self) -> Router {
        routes(self)
    }

    fn into_routes_with_options(self, options: OpenAuthAxumOptions) -> Router {
        routes_with_options(self, options)
    }
}

/// Mount OpenAuth at `auth.context().base_path`.
pub fn router(auth: OpenAuth) -> Result<Router, OpenAuthAxumError> {
    router_with_options(auth, OpenAuthAxumOptions::default())
}

/// Mount OpenAuth at `auth.context().base_path` with adapter-specific options.
pub fn router_with_options(
    auth: OpenAuth,
    options: OpenAuthAxumOptions,
) -> Result<Router, OpenAuthAxumError> {
    validate_base_url_matches_base_path(&auth)?;
    let base_path = normalize_base_path(&auth.context().base_path)?;
    if base_path == "/" {
        return Ok(routes_with_options(auth, options));
    }
    Ok(Router::new().nest(&base_path, routes_with_options(auth, options)))
}

/// Build unmounted OpenAuth catch-all routes.
///
/// Use this when composing with an existing Axum router manually. The returned
/// router should be nested at the same path as `OpenAuthOptions.base_path`.
pub fn routes(auth: OpenAuth) -> Router {
    routes_with_options(auth, OpenAuthAxumOptions::default())
}

/// Build unmounted OpenAuth catch-all routes with adapter-specific options.
pub fn routes_with_options(auth: OpenAuth, options: OpenAuthAxumOptions) -> Router {
    routes_from_shared(Arc::new(auth), options)
}

fn routes_from_shared(auth: Arc<OpenAuth>, options: OpenAuthAxumOptions) -> Router {
    Router::new()
        .route("/", any(route_handler))
        .route("/{*path}", any(route_handler))
        .with_state(OpenAuthAxumState { auth, options })
}

/// Handle a single Axum request through OpenAuth.
pub async fn handle(auth: &OpenAuth, request: Request<Body>) -> axum::response::Response {
    handle_ref(auth, request).await
}

/// Handle a single Axum request through OpenAuth with adapter-specific options.
pub async fn handle_with_options(
    auth: &OpenAuth,
    options: OpenAuthAxumOptions,
    request: Request<Body>,
) -> axum::response::Response {
    handle_ref_with_options(auth, options, request).await
}

/// Handle a single Axum request through a borrowed OpenAuth instance.
pub async fn handle_ref(auth: &OpenAuth, request: Request<Body>) -> axum::response::Response {
    handle_ref_with_options(auth, OpenAuthAxumOptions::default(), request).await
}

/// Handle a single Axum request through a borrowed OpenAuth instance with options.
pub async fn handle_ref_with_options(
    auth: &OpenAuth,
    options: OpenAuthAxumOptions,
    request: Request<Body>,
) -> axum::response::Response {
    match to_api_request(request, options).await {
        Ok(mut request) => {
            maybe_insert_base_url(auth, &mut request, options);
            match auth.handler_async(request).await {
                Ok(response) => from_api_response(response),
                Err(error) => {
                    log_internal_error(auth, &error);
                    internal_error_response()
                }
            }
        }
        Err(response) => response,
    }
}

async fn route_handler(
    State(state): State<OpenAuthAxumState>,
    request: Request<Body>,
) -> impl IntoResponse {
    handle_ref_with_options(state.auth.as_ref(), state.options, request).await
}

fn validate_base_url_matches_base_path(auth: &OpenAuth) -> Result<(), OpenAuthAxumError> {
    let base_url = auth.context().base_url.as_str();
    if base_url.is_empty() {
        return Ok(());
    }

    let parsed = url::Url::parse(base_url)
        .map_err(|_| OpenAuthAxumError::InvalidBaseUrl(base_url.to_owned()))?;
    let url_path = trim_path_suffix(parsed.path());
    let base_path = trim_path_suffix(&auth.context().base_path);
    if url_path == base_path {
        return Ok(());
    }

    Err(OpenAuthAxumError::InconsistentBaseUrlPath {
        url_path,
        base_path,
    })
}

fn trim_path_suffix(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn normalize_base_path(base_path: &str) -> Result<String, OpenAuthAxumError> {
    if base_path.is_empty() {
        return Ok("/".to_owned());
    }
    if !is_valid_base_path(base_path) {
        return Err(OpenAuthAxumError::InvalidBasePath(base_path.to_owned()));
    }

    let trimmed = base_path.trim_end_matches('/');
    if trimmed.is_empty() {
        Ok("/".to_owned())
    } else {
        Ok(trimmed.to_owned())
    }
}

fn maybe_insert_base_url(
    auth: &OpenAuth,
    request: &mut openauth::ApiRequest,
    options: OpenAuthAxumOptions,
) {
    if !options.infer_base_url_from_request
        || !auth.context().base_url.is_empty()
        || request.extensions().get::<OAuthBaseUrlOverride>().is_some()
    {
        return;
    }

    if let Some(base_url) = infer_base_url(
        request.headers(),
        request.uri(),
        &auth.context().base_path,
        options.trust_proxy_headers_for_base_url,
    ) {
        request
            .extensions_mut()
            .insert(RequestBaseUrl(base_url.clone()));
        request
            .extensions_mut()
            .insert(OAuthBaseUrlOverride(base_url));
    }
}

fn infer_base_url(
    headers: &HeaderMap,
    uri: &Uri,
    base_path: &str,
    trust_proxy_headers: bool,
) -> Option<String> {
    let origin = if trust_proxy_headers {
        forwarded_origin(headers)
    } else {
        None
    }
    .or_else(|| uri_origin(uri))
    .or_else(|| host_header_origin(headers))?;
    Some(with_base_path(origin, base_path))
}

fn forwarded_origin(headers: &HeaderMap) -> Option<String> {
    let host = header_str(headers, "x-forwarded-host")?;
    let proto = header_str(headers, "x-forwarded-proto")?;
    if !is_valid_forwarded_host(host) || !is_valid_forwarded_proto(proto) {
        return None;
    }
    Some(format!("{}://{}", proto.to_ascii_lowercase(), host))
}

fn uri_origin(uri: &Uri) -> Option<String> {
    let scheme = uri.scheme_str()?;
    if !is_valid_forwarded_proto(scheme) {
        return None;
    }
    let authority = uri.authority()?.as_str();
    if !is_valid_forwarded_host(authority) {
        return None;
    }
    Some(format!("{}://{}", scheme, authority))
}

fn host_header_origin(headers: &HeaderMap) -> Option<String> {
    let host = header_str(headers, header::HOST.as_str())?;
    if !is_valid_forwarded_host(host) {
        return None;
    }
    let scheme = if is_loopback_host(host) {
        "http"
    } else {
        "https"
    };
    Some(format!("{scheme}://{host}"))
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn with_base_path(mut origin: String, base_path: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    if !base_path.is_empty() && base_path != "/" {
        origin.push_str(base_path);
    }
    origin
}

fn is_valid_base_path(base_path: &str) -> bool {
    base_path.starts_with('/')
        && !base_path.contains('?')
        && !base_path.contains('#')
        && !base_path.contains('{')
        && !base_path.contains('}')
        && !base_path.contains('*')
}

fn log_internal_error(auth: &OpenAuth, error: &OpenAuthError) {
    let message = error.to_string();
    auth.context()
        .logger
        .error("OpenAuth Axum handler failed", &[message.as_str()]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const SECRET: &str = "test-secret-123456789012345678901234";

    #[test]
    fn normalize_base_path_trims_trailing_slashes_except_root() -> Result<(), OpenAuthAxumError> {
        assert_eq!(normalize_base_path("")?, "/");
        assert_eq!(normalize_base_path("/")?, "/");
        assert_eq!(normalize_base_path("/api/auth/")?, "/api/auth");
        assert_eq!(normalize_base_path("/api/auth///")?, "/api/auth");
        Ok(())
    }

    #[test]
    fn normalize_base_path_rejects_axum_pattern_syntax_and_non_absolute_paths() {
        for base_path in [
            "api/auth",
            "/api/{auth}",
            "/api/*auth",
            "/api/auth?x=1",
            "/api/auth#x",
        ] {
            assert!(matches!(
                normalize_base_path(base_path),
                Err(OpenAuthAxumError::InvalidBasePath(_))
            ));
        }
    }

    #[test]
    fn infer_base_url_rejects_malicious_forwarded_headers_and_falls_back_to_host() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("javascript:alert(1)"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("http"));
        headers.insert(header::HOST, HeaderValue::from_static("app.example.com"));

        let base = infer_base_url(
            &headers,
            &Uri::from_static("/api/auth/ok"),
            "/api/auth",
            true,
        );
        assert_eq!(base.as_deref(), Some("https://app.example.com/api/auth"));
    }

    #[test]
    fn infer_base_url_uses_forwarded_headers_when_trusted_and_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("public.example.com"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        headers.insert(header::HOST, HeaderValue::from_static("internal.local"));

        let base = infer_base_url(&headers, &Uri::from_static("/ok"), "/api/auth", true);
        assert_eq!(base.as_deref(), Some("https://public.example.com/api/auth"));
    }

    #[test]
    fn infer_base_url_uses_absolute_request_uri_origin() {
        let headers = HeaderMap::new();
        let uri = Uri::from_static("https://app.example.com/api/auth/sign-in/social");
        let base = infer_base_url(&headers, &uri, "/api/auth", false);
        assert_eq!(base.as_deref(), Some("https://app.example.com/api/auth"));
    }

    #[test]
    fn infer_base_url_uses_http_for_loopback_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));

        let base = infer_base_url(&headers, &Uri::from_static("/ok"), "/api/auth", false);
        assert_eq!(base.as_deref(), Some("http://127.0.0.1:3000/api/auth"));
    }

    #[test]
    fn validate_base_url_accepts_matching_pathname() -> Result<(), OpenAuthError> {
        let auth = OpenAuth::builder()
            .secret(SECRET)
            .base_path("/api/auth")
            .base_url("http://localhost:3000/api/auth/")
            .build()?;
        assert!(validate_base_url_matches_base_path(&auth).is_ok());
        Ok(())
    }

    #[test]
    fn validate_base_url_rejects_mismatched_pathname() -> Result<(), OpenAuthError> {
        let auth = OpenAuth::builder()
            .secret(SECRET)
            .base_path("/api/auth")
            .base_url("http://localhost:3000/wrong")
            .build()?;
        assert!(matches!(
            validate_base_url_matches_base_path(&auth),
            Err(OpenAuthAxumError::InconsistentBaseUrlPath { .. })
        ));
        Ok(())
    }

    #[test]
    fn validate_base_url_rejects_invalid_absolute_url() -> Result<(), OpenAuthError> {
        let auth = OpenAuth::builder()
            .secret(SECRET)
            .base_path("/api/auth")
            .base_url("not-a-url")
            .build()?;
        assert!(matches!(
            validate_base_url_matches_base_path(&auth),
            Err(OpenAuthAxumError::InvalidBaseUrl(_))
        ));
        Ok(())
    }

    #[test]
    fn axum_state_clones_only_the_shared_auth_pointer() -> Result<(), OpenAuthError> {
        let auth = Arc::new(OpenAuth::builder().secret(SECRET).build()?);
        let state = OpenAuthAxumState {
            auth: Arc::clone(&auth),
            options: OpenAuthAxumOptions::default(),
        };

        let cloned = state.clone();

        assert_eq!(Arc::strong_count(&auth), 3);
        drop(cloned);
        assert_eq!(Arc::strong_count(&auth), 2);
        Ok(())
    }
}
