use openauth_redis::normalize_redis_url;

#[test]
fn redis_urls_normalize_valkey_aliases() {
    assert_eq!(
        normalize_redis_url("valkey://localhost:6379").as_ref(),
        "redis://localhost:6379"
    );
    assert_eq!(
        normalize_redis_url("valkeys://localhost:6380").as_ref(),
        "rediss://localhost:6380"
    );
}
