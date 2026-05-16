//! SQLx database adapters for OpenAuth.

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "mysql")]
mod mysql;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteAdapter;

#[cfg(feature = "postgres")]
pub use postgres::PostgresAdapter;

#[cfg(feature = "mysql")]
pub use mysql::MySqlAdapter;

#[cfg(feature = "mysql")]
pub use mysql::MySqlRateLimitStore;

#[cfg(feature = "postgres")]
pub use postgres::PostgresRateLimitStore;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteRateLimitStore;

use openauth_core::db::DbSchema;
use openauth_core::options::{RateLimitConsumeInput, RateLimitDecision, RateLimitRecord};

#[derive(Debug, Clone)]
pub(crate) struct RateLimitSqlNames {
    pub table: String,
    pub key: String,
    pub count: String,
    pub last_request: String,
}

impl RateLimitSqlNames {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            key: "key".to_owned(),
            count: "count".to_owned(),
            last_request: "last_request".to_owned(),
        }
    }

    pub fn from_schema(schema: &DbSchema) -> Self {
        let Some(table) = schema.table("rate_limit") else {
            return Self::new("rate_limits");
        };
        Self {
            table: table.name.clone(),
            key: table
                .field("key")
                .map(|field| field.name.clone())
                .unwrap_or_else(|| "key".to_owned()),
            count: table
                .field("count")
                .map(|field| field.name.clone())
                .unwrap_or_else(|| "count".to_owned()),
            last_request: table
                .field("last_request")
                .map(|field| field.name.clone())
                .unwrap_or_else(|| "last_request".to_owned()),
        }
    }
}

fn consume_record(
    input: RateLimitConsumeInput,
    existing: Option<RateLimitRecord>,
) -> (RateLimitDecision, RateLimitRecord, bool) {
    let window_ms = input.rule.window.saturating_mul(1000) as i64;
    match existing {
        Some(record)
            if input.now_ms.saturating_sub(record.last_request) <= window_ms
                && record.count >= input.rule.max =>
        {
            let retry_ms = record
                .last_request
                .saturating_add(window_ms)
                .saturating_sub(input.now_ms)
                .max(0);
            (
                RateLimitDecision {
                    permitted: false,
                    retry_after: ceil_millis_to_seconds(retry_ms),
                    limit: input.rule.max,
                    remaining: 0,
                    reset_after: ceil_millis_to_seconds(retry_ms),
                },
                record,
                true,
            )
        }
        Some(mut record) if input.now_ms.saturating_sub(record.last_request) <= window_ms => {
            record.key = input.key;
            record.count = record.count.saturating_add(1);
            record.last_request = input.now_ms;
            let remaining = input.rule.max.saturating_sub(record.count);
            (
                RateLimitDecision {
                    permitted: true,
                    retry_after: 0,
                    limit: input.rule.max,
                    remaining,
                    reset_after: input.rule.window,
                },
                record,
                true,
            )
        }
        Some(mut record) => {
            record.key = input.key;
            record.count = 1;
            record.last_request = input.now_ms;
            (
                RateLimitDecision {
                    permitted: true,
                    retry_after: 0,
                    limit: input.rule.max,
                    remaining: input.rule.max.saturating_sub(1),
                    reset_after: input.rule.window,
                },
                record,
                true,
            )
        }
        None => {
            let record = RateLimitRecord {
                key: input.key,
                count: 1,
                last_request: input.now_ms,
            };
            (
                RateLimitDecision {
                    permitted: true,
                    retry_after: 0,
                    limit: input.rule.max,
                    remaining: input.rule.max.saturating_sub(1),
                    reset_after: input.rule.window,
                },
                record,
                false,
            )
        }
    }
}

fn ceil_millis_to_seconds(milliseconds: i64) -> u64 {
    if milliseconds <= 0 {
        return 0;
    }
    ((milliseconds as u64).saturating_add(999)) / 1000
}
