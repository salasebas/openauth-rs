//! Framework-neutral auth route builders.

use std::sync::Arc;

use http::Method;

use crate::api::AsyncAuthEndpoint;
use crate::db::DbAdapter;

mod session;
mod shared;
mod sign_in;
mod sign_out;
mod sign_up;

/// Build Better Auth-inspired core endpoints backed by an OpenAuth database adapter.
///
/// The returned endpoints are framework-neutral and can be passed to
/// `AuthRouter::with_async_endpoints` or the public `open_auth_with_endpoints`
/// initializer. Concrete web frameworks only need to adapt HTTP requests and
/// responses at their edge.
pub fn core_auth_async_endpoints(adapter: Arc<dyn DbAdapter>) -> Vec<AsyncAuthEndpoint> {
    vec![
        sign_up::sign_up_email_endpoint(Arc::clone(&adapter)),
        sign_in::sign_in_email_endpoint(Arc::clone(&adapter)),
        session::get_session_endpoint(Method::GET, Arc::clone(&adapter)),
        session::get_session_endpoint(Method::POST, Arc::clone(&adapter)),
        session::list_sessions_endpoint(Arc::clone(&adapter)),
        session::revoke_session_endpoint(Arc::clone(&adapter)),
        session::revoke_sessions_endpoint(Arc::clone(&adapter)),
        session::revoke_other_sessions_endpoint(Arc::clone(&adapter)),
        sign_out::sign_out_endpoint(adapter),
    ]
}
