use fred::clients::Client;
use fred::interfaces::ClientLike;
use fred::prelude::{Builder, Config};
use fred::types::scripts::Script;
use openauth_core::error::OpenAuthError;
use openauth_core::options::{
    RateLimitConsumeInput, RateLimitDecision, RateLimitFuture, RateLimitStore,
};

use crate::script::{parse_rate_limit_script_result, RATE_LIMIT_SCRIPT};
use crate::{normalize_fred_url, FredRateLimitOptions};

#[derive(Clone)]
pub struct FredRateLimitStore {
    client: Client,
    options: FredRateLimitOptions,
    script: Script,
}

impl FredRateLimitStore {
    pub async fn connect(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect_with_options(url, FredRateLimitOptions::default()).await
    }

    pub async fn connect_redis(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect(url).await
    }

    pub async fn connect_valkey(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect(url).await
    }

    pub async fn connect_with_options(
        url: &str,
        options: FredRateLimitOptions,
    ) -> Result<Self, OpenAuthError> {
        let url = normalize_fred_url(url);
        let config = Config::from_url(url.as_ref())
            .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
        let client = Builder::from_config(config)
            .build()
            .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
        client
            .init()
            .await
            .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
        Ok(Self::new(client, options))
    }

    pub fn new(client: Client, options: FredRateLimitOptions) -> Self {
        Self {
            client,
            options,
            script: Script::from_lua(RATE_LIMIT_SCRIPT),
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}rate-limit:{key}", self.options.key_prefix)
    }
}

impl RateLimitStore for FredRateLimitStore {
    fn consume<'a>(&'a self, input: RateLimitConsumeInput) -> RateLimitFuture<'a> {
        Box::pin(async move {
            let redis_key = self.key(&input.key);
            let window_ms = input.rule.window.saturating_mul(1000);
            let result = self
                .script
                .evalsha_with_reload(
                    &self.client,
                    vec![redis_key],
                    vec![
                        input.now_ms.to_string(),
                        window_ms.to_string(),
                        input.rule.max.to_string(),
                    ],
                )
                .await
                .map_err(|error| OpenAuthError::Adapter(error.to_string()))?;
            let result = parse_rate_limit_script_result(result)?;
            let retry_ms = result
                .last_request
                .saturating_add(window_ms as i64)
                .saturating_sub(input.now_ms)
                .max(0);
            Ok(RateLimitDecision {
                permitted: result.permitted,
                retry_after: if result.permitted {
                    0
                } else {
                    ceil_millis_to_seconds(retry_ms)
                },
                limit: input.rule.max,
                remaining: input.rule.max.saturating_sub(result.count),
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
