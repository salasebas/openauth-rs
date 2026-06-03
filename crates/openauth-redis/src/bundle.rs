use std::sync::Arc;

use openauth_core::error::OpenAuthError;
use openauth_core::options::{OpenAuthOptions, RateLimitOptions};

use crate::rate_limit::{RedisRateLimitOptions, RedisRateLimitStore};
use crate::secondary::{RedisSecondaryStorage, RedisSecondaryStorageOptions};

/// Shared connection options for rate limiting and secondary storage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedisOpenAuthOptions {
    pub rate_limit: RedisRateLimitOptions,
    pub secondary_storage: RedisSecondaryStorageOptions,
}

/// Rate limit and secondary storage backed by one `ConnectionManager`.
#[derive(Clone)]
pub struct RedisOpenAuthStores {
    pub rate_limit: RedisRateLimitStore,
    pub secondary_storage: RedisSecondaryStorage,
}

impl RedisOpenAuthStores {
    pub async fn connect(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect_with_options(url, RedisOpenAuthOptions::default()).await
    }

    pub async fn connect_redis(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect(url).await
    }

    pub async fn connect_valkey(url: &str) -> Result<Self, OpenAuthError> {
        Self::connect(url).await
    }

    pub async fn connect_with_options(
        url: &str,
        options: RedisOpenAuthOptions,
    ) -> Result<Self, OpenAuthError> {
        let manager = crate::connect_manager(url).await?;
        Ok(Self {
            rate_limit: RedisRateLimitStore::new(manager.clone(), options.rate_limit),
            secondary_storage: RedisSecondaryStorage::new(manager, options.secondary_storage),
        })
    }

    /// Wires both stores into [`OpenAuthOptions`] (secondary storage + distributed rate limit).
    #[must_use]
    pub fn apply_to_options(&self, options: OpenAuthOptions) -> OpenAuthOptions {
        options
            .secondary_storage(Arc::new(self.secondary_storage.clone()))
            .rate_limit(RateLimitOptions::secondary_storage(self.rate_limit.clone()))
    }
}
