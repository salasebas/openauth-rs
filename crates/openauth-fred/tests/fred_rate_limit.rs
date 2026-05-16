use http::{Method, Request, StatusCode};
use openauth::OpenAuth;
use openauth_core::options::{RateLimitConsumeInput, RateLimitRule, RateLimitStore};
use openauth_fred::FredRateLimitStore;

const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379";
const DEFAULT_VALKEY_URL: &str = "valkey://127.0.0.1:6380";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FredTestTarget {
    name: &'static str,
    url: String,
}

fn fred_targets() -> Vec<FredTestTarget> {
    fred_targets_from_env(
        std::env::var("OPENAUTH_FRED_REDIS_URL").ok(),
        std::env::var("OPENAUTH_FRED_VALKEY_URL").ok(),
    )
}

fn fred_targets_from_env(
    redis_url: Option<String>,
    valkey_url: Option<String>,
) -> Vec<FredTestTarget> {
    let mut targets = Vec::new();
    if let Some(url) = redis_url {
        targets.push(FredTestTarget { name: "redis", url });
    }
    if let Some(url) = valkey_url {
        targets.push(FredTestTarget {
            name: "valkey",
            url,
        });
    }
    if targets.is_empty() {
        targets.push(FredTestTarget {
            name: "redis",
            url: DEFAULT_REDIS_URL.to_owned(),
        });
        targets.push(FredTestTarget {
            name: "valkey",
            url: DEFAULT_VALKEY_URL.to_owned(),
        });
    }
    targets
}

#[test]
fn fred_targets_default_to_docker_compose_redis_and_valkey_when_env_is_unset() {
    assert_eq!(
        fred_targets_from_env(None, None),
        vec![
            FredTestTarget {
                name: "redis",
                url: DEFAULT_REDIS_URL.to_owned(),
            },
            FredTestTarget {
                name: "valkey",
                url: DEFAULT_VALKEY_URL.to_owned(),
            },
        ]
    );
}

#[test]
fn fred_targets_allow_env_overrides() {
    assert_eq!(
        fred_targets_from_env(
            Some("redis://redis.test:6379".to_owned()),
            Some("valkey://valkey.test:6379".to_owned()),
        ),
        vec![
            FredTestTarget {
                name: "redis",
                url: "redis://redis.test:6379".to_owned(),
            },
            FredTestTarget {
                name: "valkey",
                url: "valkey://valkey.test:6379".to_owned(),
            },
        ]
    );
}

#[tokio::test]
async fn fred_rate_limit_store_enforces_atomic_max_one() -> Result<(), Box<dyn std::error::Error>> {
    for target in fred_targets() {
        let store = FredRateLimitStore::connect(&target.url).await?;
        let now_ms = now_ms();
        let key = format!("test:{}:{}|/limited", target.name, now_ms);
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

        assert!(first.permitted, "{} should permit first call", target.name);
        assert!(!second.permitted, "{} should deny second call", target.name);
        assert_eq!(second.remaining, 0);
    }
    Ok(())
}

#[tokio::test]
async fn fred_rate_limit_store_allows_exactly_one_concurrent_request(
) -> Result<(), Box<dyn std::error::Error>> {
    for target in fred_targets() {
        let store = FredRateLimitStore::connect(&target.url).await?;
        let now_ms = now_ms();
        let key = format!("test:{}:{now_ms}|/concurrent", target.name);
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

        assert_eq!(
            permitted, 1,
            "{} should permit exactly one concurrent call",
            target.name
        );
    }
    Ok(())
}

#[tokio::test]
async fn fred_rate_limit_store_resets_after_window() -> Result<(), Box<dyn std::error::Error>> {
    for target in fred_targets() {
        let store = FredRateLimitStore::connect(&target.url).await?;
        let now_ms = now_ms();
        let key = format!("test:{}:{now_ms}|/reset", target.name);
        let rule = RateLimitRule { window: 1, max: 1 };

        let first = store
            .consume(RateLimitConsumeInput {
                key: key.clone(),
                rule: rule.clone(),
                now_ms,
            })
            .await?;
        let second = store
            .consume(RateLimitConsumeInput {
                key,
                rule,
                now_ms: now_ms + 1_001,
            })
            .await?;

        assert!(first.permitted);
        assert!(
            second.permitted,
            "{} should reset after window",
            target.name
        );
        assert_eq!(second.remaining, 0);
    }
    Ok(())
}

#[tokio::test]
async fn openauth_handler_async_uses_fred_rate_limit_store(
) -> Result<(), Box<dyn std::error::Error>> {
    for target in fred_targets() {
        let store = FredRateLimitStore::connect(&target.url).await?;
        let auth = OpenAuth::builder()
            .secret("secret-a-at-least-32-chars-long!!")
            .rate_limit(
                openauth::RateLimitOptions::secondary_storage(store)
                    .enabled(true)
                    .window(60)
                    .max(1),
            )
            .build()?;

        let ip = unique_ip(if target.name == "redis" { 0 } else { 1 });
        let first = auth
            .handler_async(
                Request::builder()
                    .method(Method::GET)
                    .uri("http://localhost:3000/api/auth/ok")
                    .header("x-forwarded-for", &ip)
                    .body(Vec::new())?,
            )
            .await?;
        let second = auth
            .handler_async(
                Request::builder()
                    .method(Method::GET)
                    .uri("http://localhost:3000/api/auth/ok")
                    .header("x-forwarded-for", &ip)
                    .body(Vec::new())?,
            )
            .await?;

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn unique_ip(offset: u8) -> String {
    let seed = now_ms() as u64;
    let second = ((seed >> 16) & 0xff) as u8;
    let third = ((seed >> 8) & 0xff) as u8;
    let fourth = ((seed & 0xfe) as u8).saturating_add(offset).max(1);
    format!("10.{second}.{third}.{fourth}")
}
