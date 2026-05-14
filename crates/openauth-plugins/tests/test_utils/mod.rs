#[test]
fn exposes_test_utils_placeholder() {
    assert_eq!(
        openauth_plugins::test_utils::UPSTREAM_PLUGIN_ID,
        "test-utils"
    );
}
