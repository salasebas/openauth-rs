//! Bundled database adapter + SQL-backed rate-limit store for each SQLx backend.

use std::sync::Arc;

use openauth_core::db::{auth_schema, AuthSchemaOptions, DbAdapter, DbSchema};
use openauth_core::error::OpenAuthError;
use openauth_core::options::{OpenAuthOptions, RateLimitOptions};

macro_rules! sql_stores {
    ($stores:ident, $builder:ident, $adapter:ty, $rate_limit:ty) => {
        /// Database adapter and matching SQL-backed rate-limit store sharing one pool.
        #[derive(Debug, Clone)]
        pub struct $stores {
            pub adapter: $adapter,
            pub rate_limit: $rate_limit,
        }

        /// Configures and connects a [`$stores`] bundle.
        #[derive(Debug, Clone)]
        pub struct $builder {
            schema: DbSchema,
        }

        impl Default for $builder {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $builder {
            pub fn new() -> Self {
                Self {
                    schema: auth_schema(AuthSchemaOptions::default()),
                }
            }

            #[must_use]
            pub fn schema(mut self, schema: DbSchema) -> Self {
                self.schema = schema;
                self
            }

            pub async fn connect(self, database_url: &str) -> Result<$stores, OpenAuthError> {
                let adapter = <$adapter>::connect_with_schema(database_url, self.schema).await?;
                let rate_limit = <$rate_limit>::from(&adapter);
                Ok($stores {
                    adapter,
                    rate_limit,
                })
            }
        }

        impl $stores {
            pub fn builder() -> $builder {
                $builder::new()
            }

            pub async fn connect(database_url: &str) -> Result<Self, OpenAuthError> {
                Self::builder().connect(database_url).await
            }

            pub async fn connect_with_schema(
                database_url: &str,
                schema: DbSchema,
            ) -> Result<Self, OpenAuthError> {
                Self::builder().schema(schema).connect(database_url).await
            }

            /// Wires the SQL-backed rate-limit store into [`OpenAuthOptions`].
            #[must_use]
            pub fn apply_to_options(&self, options: OpenAuthOptions) -> OpenAuthOptions {
                options.rate_limit(RateLimitOptions::database(self.rate_limit.clone()))
            }

            pub fn adapter(&self) -> Arc<dyn DbAdapter> {
                Arc::new(self.adapter.clone())
            }

            pub fn adapter_ref(&self) -> &$adapter {
                &self.adapter
            }
        }
    };
}

#[cfg(feature = "sqlite")]
sql_stores!(
    SqliteStores,
    SqliteStoresBuilder,
    crate::SqliteAdapter,
    crate::SqliteRateLimitStore
);

#[cfg(feature = "postgres")]
sql_stores!(
    PostgresStores,
    PostgresStoresBuilder,
    crate::PostgresAdapter,
    crate::PostgresRateLimitStore
);

#[cfg(feature = "mysql")]
sql_stores!(
    MySqlStores,
    MySqlStoresBuilder,
    crate::MySqlAdapter,
    crate::MySqlRateLimitStore
);
