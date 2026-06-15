//! MySQL adapter stubs for the feasibility spike; full CRUD lands in plan 015.

#![allow(dead_code)]

use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncMysqlConnection;

/// Async MySQL adapter backed by a `diesel-async` deadpool pool.
#[derive(Clone)]
pub struct DieselMysqlAdapter {
    pub pool: Pool<AsyncMysqlConnection>,
}

/// Auth schema stores for MySQL.
#[derive(Clone)]
pub struct DieselMysqlStores {
    pub adapter: DieselMysqlAdapter,
}

/// Database-backed rate limit store for MySQL.
#[derive(Clone)]
pub struct DieselMysqlRateLimitStore {
    pub pool: Pool<AsyncMysqlConnection>,
}
