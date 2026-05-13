use openauth_oauth::oauth2;

#[test]
fn oauth2_module_exports_placeholder_types() {
    let provider = oauth2::OAuthProviderMetadata::new("example", "Example");

    assert_eq!(provider.id(), "example");
}

#[test]
fn oauth_provider_contract_is_public() {
    fn assert_provider_contract<T: oauth2::OAuthProviderContract>() {}

    struct TestProvider;

    impl oauth2::OAuthProviderContract for TestProvider {
        fn id(&self) -> &str {
            "test"
        }

        fn name(&self) -> &str {
            "Test"
        }
    }

    assert_provider_contract::<TestProvider>();
}
