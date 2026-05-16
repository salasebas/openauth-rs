use fred::types::Value;
use openauth_fred::{
    normalize_fred_url, parse_rate_limit_script_result, FredRateLimitOptions, RateLimitScriptResult,
};

#[test]
fn fred_rate_limit_options_default_to_openauth_prefix() {
    assert_eq!(FredRateLimitOptions::default().key_prefix, "openauth:");
}

#[test]
fn fred_urls_normalize_valkey_aliases() {
    assert_eq!(
        normalize_fred_url("valkey://localhost:6379").as_ref(),
        "redis://localhost:6379"
    );
    assert_eq!(
        normalize_fred_url("valkeys://localhost:6380").as_ref(),
        "rediss://localhost:6380"
    );
}

#[test]
fn fred_urls_leave_redis_and_unix_urls_unchanged() {
    assert_eq!(
        normalize_fred_url("redis://localhost:6379").as_ref(),
        "redis://localhost:6379"
    );
    assert_eq!(
        normalize_fred_url("rediss://localhost:6380").as_ref(),
        "rediss://localhost:6380"
    );
    assert_eq!(
        normalize_fred_url("unix:///tmp/redis.sock").as_ref(),
        "unix:///tmp/redis.sock"
    );
}

#[test]
fn parses_valid_lua_result() -> Result<(), Box<dyn std::error::Error>> {
    let result = parse_rate_limit_script_result(Value::Array(vec![
        Value::Integer(1),
        Value::Integer(3),
        Value::Integer(1_000),
    ]))?;

    assert_eq!(
        result,
        RateLimitScriptResult {
            permitted: true,
            count: 3,
            last_request: 1_000,
        }
    );
    Ok(())
}

#[test]
fn rejects_malformed_lua_result() {
    let result = parse_rate_limit_script_result(Value::Array(vec![Value::Integer(1)]));

    assert!(result.is_err());
}
