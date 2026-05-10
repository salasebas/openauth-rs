use openauth_oauth::{oauth2, social_providers};

#[test]
fn oauth2_module_exports_placeholder_types() {
    let provider = oauth2::OAuthProviderMetadata::new("example", "Example");

    assert_eq!(provider.id(), "example");
}

#[test]
fn social_provider_registry_contains_upstream_provider_names() {
    assert!(social_providers::PROVIDER_IDS.contains(&"github"));
    assert!(social_providers::PROVIDER_IDS.contains(&"microsoft"));
    assert!(social_providers::PROVIDER_IDS.contains(&"wechat"));
}
