//! Redis-backed integrations for OpenAuth.

use openauth_core::error::OpenAuthError;
use openauth_core::options::{
    RateLimitConsumeInput, RateLimitDecision, RateLimitFuture, RateLimitStore,
};
use redis::aio::ConnectionManager;
use redis::Script;
use std::sync::Arc;
use tokio::sync::Mutex;

const RATE_LIMIT_SCRIPT: &str = r#"
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local max = tonumber(ARGV[3])

local data = redis.call("HMGET", key, "count", "last_request")
local count = tonumber(data[1])
local last_request = tonumber(data[2])

if count == nil or last_request == nil or (now - last_request) > window then
  redis.call("HMSET", key, "count", 1, "last_request", now)
  redis.call("PEXPIRE", key, window)
  return {1, 1, now}
end

if count >= max then
  redis.call("PEXPIRE", key, window)
  return {0, count, last_request}
end

count = count + 1
redis.call("HMSET", key, "count", count, "last_request", now)
redis.call("PEXPIRE", key, window)
return {1, count, now}
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisRateLimitOptions {
    pub key_prefix: String,
}

impl Default for RedisRateLimitOptions {
    fn default() -> Self {
        Self {
            key_prefix: "openauth:".to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct RedisRateLimitStore {
    manager: Arc<Mutex<ConnectionManager>>,
    options: RedisRateLimitOptions,
}

impl RedisRateLimitStore {
    pub async fn connect(redis_url: &str) -> Result<Self, OpenAuthError> {
        let client = redis::Client::open(redis_url)
            .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
        let manager = ConnectionManager::new(client)
            .await
            .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
        Ok(Self::new(manager, RedisRateLimitOptions::default()))
    }

    pub fn new(manager: ConnectionManager, options: RedisRateLimitOptions) -> Self {
        Self {
            manager: Arc::new(Mutex::new(manager)),
            options,
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}rate-limit:{key}", self.options.key_prefix)
    }
}

impl RateLimitStore for RedisRateLimitStore {
    fn consume<'a>(&'a self, input: RateLimitConsumeInput) -> RateLimitFuture<'a> {
        Box::pin(async move {
            let redis_key = self.key(&input.key);
            let window_ms = input.rule.window.saturating_mul(1000);
            let mut manager = self.manager.lock().await;
            let result: (i64, i64, i64) = Script::new(RATE_LIMIT_SCRIPT)
                .key(redis_key)
                .arg(input.now_ms)
                .arg(window_ms as i64)
                .arg(input.rule.max as i64)
                .invoke_async(&mut *manager)
                .await
                .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
            let permitted = result.0 == 1;
            let count = result.1.max(0) as u64;
            let last_request = result.2;
            let retry_ms = last_request
                .saturating_add(window_ms as i64)
                .saturating_sub(input.now_ms)
                .max(0);
            Ok(RateLimitDecision {
                permitted,
                retry_after: if permitted {
                    0
                } else {
                    ceil_millis_to_seconds(retry_ms)
                },
                limit: input.rule.max,
                remaining: input.rule.max.saturating_sub(count),
                reset_after: ceil_millis_to_seconds(retry_ms),
            })
        })
    }
}

fn ceil_millis_to_seconds(milliseconds: i64) -> u64 {
    if milliseconds <= 0 {
        return 0;
    }
    ((milliseconds as u64).saturating_add(999)) / 1000
}

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
