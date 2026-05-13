use openauth_social_providers::PROVIDER_IDS;

#[test]
fn social_provider_registry_contains_upstream_provider_names() {
    assert!(PROVIDER_IDS.contains(&"github"));
    assert!(PROVIDER_IDS.contains(&"linkedin"));
    assert!(PROVIDER_IDS.contains(&"microsoft"));
    assert!(PROVIDER_IDS.contains(&"wechat"));
}
