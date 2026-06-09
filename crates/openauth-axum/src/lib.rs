//! Axum integration for OpenAuth.

mod error;
mod options;
mod request;
mod response;
mod router;

pub use error::OpenAuthAxumError;
pub use options::OpenAuthAxumOptions;
pub use router::{handle, handle_with_options, OpenAuthAxumExt};
