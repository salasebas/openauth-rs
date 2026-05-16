use openauth_core::options::{RateLimitConsumeInput, RateLimitRule, RateLimitStore};
use openauth_redis::RedisRateLimitStore;

#[tokio::test]
async fn redis_rate_limit_store_enforces_atomic_max_one() -> Result<(), Box<dyn std::error::Error>>
{
    let Ok(redis_url) = std::env::var("OPENAUTH_REDIS_URL") else {
        eprintln!("skipping redis test; OPENAUTH_REDIS_URL is not set");
        return Ok(());
    };
    let store = RedisRateLimitStore::connect(&redis_url).await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;
    let key = format!("test:{}|/limited", now_ms);
    let rule = RateLimitRule { window: 60, max: 1 };

    let first = store
        .consume(RateLimitConsumeInput {
            key: key.clone(),
            rule: rule.clone(),
            now_ms,
        })
        .await?;
    let second = store
        .consume(RateLimitConsumeInput { key, rule, now_ms })
        .await?;

    assert!(first.permitted);
    assert!(!second.permitted);
    assert_eq!(second.remaining, 0);
    Ok(())
}

#[tokio::test]
async fn redis_rate_limit_store_allows_exactly_one_concurrent_request(
) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(redis_url) = std::env::var("OPENAUTH_REDIS_URL") else {
        eprintln!("skipping redis test; OPENAUTH_REDIS_URL is not set");
        return Ok(());
    };
    let store = RedisRateLimitStore::connect(&redis_url).await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;
    let key = format!("test:{now_ms}|/concurrent");
    let rule = RateLimitRule { window: 60, max: 1 };
    let first = RateLimitConsumeInput {
        key: key.clone(),
        rule: rule.clone(),
        now_ms,
    };
    let second = RateLimitConsumeInput { key, rule, now_ms };

    let (first, second) = tokio::join!(store.consume(first), store.consume(second));
    let permitted = [first?, second?]
        .into_iter()
        .filter(|decision| decision.permitted)
        .count();

    assert_eq!(permitted, 1);
    Ok(())
}
