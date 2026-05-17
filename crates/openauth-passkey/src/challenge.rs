use openauth_core::crypto::random::generate_random_string;
use openauth_core::db::DbAdapter;
use openauth_core::error::OpenAuthError;
use openauth_core::verification::{CreateVerificationInput, DbVerificationStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Duration, OffsetDateTime};

use crate::options::PasskeyRegistrationUser;

pub const CHALLENGE_MAX_AGE_SECONDS: u64 = 60 * 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeKind {
    Registration,
    Authentication,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChallengeValue {
    pub kind: ChallengeKind,
    pub state: Value,
    pub user: Option<PasskeyRegistrationUser>,
    pub context: Option<String>,
}

pub async fn create_challenge(
    adapter: &dyn DbAdapter,
    value: ChallengeValue,
) -> Result<String, OpenAuthError> {
    let token = generate_random_string(32);
    let expires_at =
        OffsetDateTime::now_utc() + Duration::seconds(CHALLENGE_MAX_AGE_SECONDS as i64);
    DbVerificationStore::new(adapter)
        .create_verification(CreateVerificationInput::new(
            token.clone(),
            serde_json::to_string(&value).map_err(|error| OpenAuthError::Api(error.to_string()))?,
            expires_at,
        ))
        .await?;
    Ok(token)
}

pub async fn find_challenge(
    adapter: &dyn DbAdapter,
    token: &str,
) -> Result<Option<ChallengeValue>, OpenAuthError> {
    DbVerificationStore::new(adapter)
        .find_verification(token)
        .await?
        .map(|verification| {
            serde_json::from_str::<ChallengeValue>(&verification.value)
                .map_err(|error| OpenAuthError::Api(error.to_string()))
        })
        .transpose()
}
