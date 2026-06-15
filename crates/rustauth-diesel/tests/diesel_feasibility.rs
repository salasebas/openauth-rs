//! Feasibility tests for the Diesel async adapter spike.
//!
//! Requires Docker Compose Postgres/MySQL services. Start with:
//! `./scripts/ensure-test-services.sh postgres` or `mysql`.

#![cfg(any(feature = "postgres", feature = "mysql"))]

mod support;

#[cfg(feature = "postgres")]
#[path = "diesel_feasibility/postgres.rs"]
mod postgres_feasibility;

#[cfg(feature = "mysql")]
#[path = "diesel_feasibility/mysql.rs"]
mod mysql_feasibility;
