//! Pooled Postgres database adapter for OpenAuth.
//!
//! This crate is the recommended Postgres adapter for production deployments.
//! It keeps pooling in `deadpool-postgres` and reuses OpenAuth's shared SQL
//! planning plus `openauth-tokio-postgres` driver helpers.

mod adapter;
mod builder;
mod config;
mod rate_limit;
mod transaction;
mod tx_guard;

pub use self::adapter::DeadpoolPostgresAdapter;
pub use self::builder::{DeadpoolPostgresBuilder, DeadpoolPostgresStores};
pub use self::rate_limit::DeadpoolPostgresRateLimitStore;
